//! Extensions: discovery, validation, listing and running.
//!
//! An extension is a standalone executable on `PATH` named `sisho-<word>`,
//! invoked as `sisho <word> ...`. `sisho --list` prints all installed extensions.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;

use crate::commands::with_standard_streams;
use crate::output::{BROKEN_PIPE, is_broken_pipe};

/// An extension word consists only of ASCII lowercase letters, digits, and
/// hyphens, and does not start with a hyphen.
pub fn is_valid_word(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with('-')
        && word
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// An extension program found on `PATH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionEntry {
    /// The extension word, e.g. "desk" for `sisho-desk`.
    pub word: String,
    /// Absolute or relative path to the executable program as found on `PATH`.
    pub path: PathBuf,
    /// Whether this entry is active or shadowed.
    pub status: ExtensionStatus,
    /// Encounter order to maintain stable sorting among duplicates.
    order: usize,
}

/// The status of a discovered extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionStatus {
    /// This extension is runnable as `sisho <word>`.
    Active,
    /// A built-in command or known tool has precedence and prevents this extension from running.
    ShadowedByBuiltin,
    /// Another program with the same word appeared earlier on `PATH`.
    ShadowedByDuplicate(PathBuf),
}

/// Run `sisho --list`: scan `PATH` and print installed extensions.
pub fn list(all: bool) -> ExitCode {
    let result = with_standard_streams(|out, _err| run_list(out, all));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if is_broken_pipe(&error) => ExitCode::from(BROKEN_PIPE),
        Err(error) => {
            eprintln!("sisho: {error:#}");
            ExitCode::from(crate::OWN_ERROR)
        }
    }
}

/// Write the list of extensions to `out`.
pub fn run_list(out: &mut dyn Write, all: bool) -> Result<()> {
    let entries = scan();
    format_entries(out, &entries, all)
}

/// Format and write `entries` to `out`.
pub fn format_entries(out: &mut dyn Write, entries: &[ExtensionEntry], all: bool) -> Result<()> {
    let mut visible: Vec<&ExtensionEntry> = entries
        .iter()
        .filter(|entry| {
            if all {
                true
            } else {
                matches!(
                    entry.status,
                    ExtensionStatus::Active | ExtensionStatus::ShadowedByBuiltin
                )
            }
        })
        .collect();

    // Sort alphabetically by word, then by encounter order for duplicates.
    visible.sort_by(|a, b| a.word.cmp(&b.word).then_with(|| a.order.cmp(&b.order)));

    let max_word_len = visible.iter().map(|e| e.word.len()).max().unwrap_or(0);
    let width = max_word_len.max(8);

    for entry in visible {
        match &entry.status {
            ExtensionStatus::Active => {
                writeln!(out, "{:width$}  {}", entry.word, entry.path.display())?;
            }
            ExtensionStatus::ShadowedByBuiltin => {
                writeln!(
                    out,
                    "{:width$}  {} (shadowed by builtin)",
                    entry.word,
                    entry.path.display()
                )?;
            }
            ExtensionStatus::ShadowedByDuplicate(orig) => {
                writeln!(
                    out,
                    "{:width$}  {} (shadowed by {})",
                    entry.word,
                    entry.path.display(),
                    orig.display()
                )?;
            }
        }
    }

    Ok(())
}

/// Scan directories on `PATH` for extensions.
pub fn scan() -> Vec<ExtensionEntry> {
    let path_var = env::var_os("PATH").unwrap_or_default();
    scan_in(env::split_paths(&path_var))
}

/// Scan specific directories for extensions.
pub fn scan_in(dirs: impl IntoIterator<Item = PathBuf>) -> Vec<ExtensionEntry> {
    let mut entries = Vec::new();
    let mut first_seen: HashMap<String, PathBuf> = HashMap::new();
    let mut counter = 0;

    for dir in dirs {
        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };

        // Collect and sort directory entries to ensure deterministic scan order
        // within the same directory.
        let mut dir_entries = Vec::new();
        for entry in read_dir.flatten() {
            dir_entries.push(entry);
        }
        dir_entries.sort_by_key(|e| e.file_name());

        for entry in dir_entries {
            let file_name = entry.file_name();
            let Some(name_str) = file_name.to_str() else {
                continue;
            };
            let Some(word) = name_str.strip_prefix("sisho-") else {
                continue;
            };
            if !is_valid_word(word) {
                continue;
            }
            let path = entry.path();
            if !is_executable(&path) {
                continue;
            }

            counter += 1;
            let is_builtin = is_builtin_command_or_tool(word);
            let status = if is_builtin {
                if let Some(first_path) = first_seen.get(word) {
                    ExtensionStatus::ShadowedByDuplicate(first_path.clone())
                } else {
                    first_seen.insert(word.to_string(), path.clone());
                    ExtensionStatus::ShadowedByBuiltin
                }
            } else if let Some(first_path) = first_seen.get(word) {
                ExtensionStatus::ShadowedByDuplicate(first_path.clone())
            } else {
                first_seen.insert(word.to_string(), path.clone());
                ExtensionStatus::Active
            };

            entries.push(ExtensionEntry {
                word: word.to_string(),
                path,
                status,
                order: counter,
            });
        }
    }

    entries
}

fn is_builtin_command_or_tool(word: &str) -> bool {
    matches!(word, "check" | "fix" | "check-links" | "convert" | "serve")
        || sisho_tools::program(word).is_some()
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    metadata.is_file() && (metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    metadata.is_file()
}

/// Query an extension executable for its metadata using `--sisho-extension-info`.
///
/// Returns `None` if the executable does not support the protocol or fails.
#[allow(dead_code)]
pub fn query_info(path: &Path) -> Option<sisho_sdk::ExtensionInfo> {
    let outcome = sisho_run::Step::new("extension-info", path)
        .arg(sisho_sdk::EXTENSION_INFO_FLAG)
        .run();
    if outcome.status == sisho_run::Status::Passed {
        sisho_sdk::ExtensionInfo::from_json_str(&outcome.stdout).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_extension_words_only_contain_allowed_characters() {
        assert!(is_valid_word("desk"));
        assert!(is_valid_word("sisho-desk"));
        assert!(is_valid_word("foo123"));
        assert!(is_valid_word("a-b-c-1"));

        assert!(!is_valid_word(""));
        assert!(!is_valid_word("-desk"));
        assert!(!is_valid_word("Desk"));
        assert!(!is_valid_word("a/b"));
        assert!(!is_valid_word("../x"));
        assert!(!is_valid_word("foo.bar"));
        assert!(!is_valid_word("foo bar"));
    }

    #[test]
    fn query_info_returns_none_for_missing_or_unsupported_binary() {
        assert_eq!(query_info(Path::new("/non/existent/sisho-tool")), None);
    }
}
