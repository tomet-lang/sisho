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
pub fn list(all: bool, verbose: bool) -> ExitCode {
    let result = with_standard_streams(|out, _err| run_list(out, all, verbose));
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
pub fn run_list(out: &mut dyn Write, all: bool, verbose: bool) -> Result<()> {
    let entries = scan();
    format_entries(out, &entries, all, verbose)
}

/// Format and write `entries` to `out`.
pub fn format_entries(
    out: &mut dyn Write,
    entries: &[ExtensionEntry],
    all: bool,
    verbose: bool,
) -> Result<()> {
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
                if verbose {
                    if let Some(info) = query_info(&entry.path) {
                        if info.word == entry.word {
                            let mut details = Vec::new();
                            if !info.version.is_empty() {
                                let ver = if info.version.starts_with('v')
                                    || info.version.starts_with('V')
                                {
                                    info.version.clone()
                                } else {
                                    format!("v{}", info.version)
                                };
                                details.push(ver);
                            }
                            if !info.about.is_empty() {
                                details.push(info.about.clone());
                            }
                            let caps = format_capabilities(&info.capabilities);
                            if !caps.is_empty() {
                                details.push(format!("[{caps}]"));
                            }
                            if !details.is_empty() {
                                let indent = " ".repeat(width + 2);
                                writeln!(out, "{indent}{}", details.join("  "))?;
                            }
                        }
                    }
                }
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

fn format_capabilities(caps: &sisho_sdk::Capabilities) -> String {
    let mut list = Vec::new();
    if caps.tui {
        list.push("tui");
    }
    if caps.interactive {
        list.push("interactive");
    }
    for (k, v) in &caps.extra {
        if v.as_bool() == Some(true) {
            list.push(k.as_str());
        }
    }
    list.join(", ")
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
    matches!(
        word,
        "check" | "fix" | "check-links" | "convert" | "serve" | "spec"
    ) || sisho_tools::program(word).is_some()
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
/// Enforces a strict 500ms timeout and discards output on failure or timeout.
/// Returns `None` if the executable does not support the protocol or fails.
pub fn query_info(path: &Path) -> Option<sisho_sdk::ExtensionInfo> {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);
    const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

    let mut child = std::process::Command::new(path)
        .arg(sisho_sdk::EXTENSION_INFO_FLAG)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut stdout_buf = Vec::new();
                if let Some(mut stream) = child.stdout.take() {
                    use std::io::Read;
                    let _ = stream.read_to_end(&mut stdout_buf);
                }
                let text = std::str::from_utf8(&stdout_buf).ok()?;
                return sisho_sdk::ExtensionInfo::from_json_str(text).ok();
            }
            Ok(None) => {
                if start.elapsed() >= TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
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

    #[test]
    fn format_capabilities_formats_tui_interactive_and_extra() {
        let mut caps = sisho_sdk::Capabilities {
            tui: true,
            interactive: false,
            extra: std::collections::BTreeMap::new(),
        };
        assert_eq!(format_capabilities(&caps), "tui");

        caps.interactive = true;
        assert_eq!(format_capabilities(&caps), "tui, interactive");

        caps.extra
            .insert("custom".to_string(), serde_json::json!(true));
        caps.extra
            .insert("ignored".to_string(), serde_json::json!(false));
        assert_eq!(format_capabilities(&caps), "tui, interactive, custom");
    }
}
