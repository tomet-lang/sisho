//! Extensions: discovery, validation, trust management, listing and running.
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
use serde::Serialize;

use crate::commands::with_standard_streams;
use crate::output::{BROKEN_PIPE, is_broken_pipe};
use crate::trust::{TrustConfig, TrustState, canonicalize_path};

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
    /// Canonicalized filesystem path to the executable program.
    pub canonical_path: PathBuf,
    /// Whether this entry is active or shadowed.
    pub status: ExtensionStatus,
    /// Trust state of the binary (`Untrusted`, `Allowed`, or `Denied`).
    pub trust: TrustState,
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

#[derive(Serialize)]
struct JsonExtensionEntry<'a> {
    word: &'a str,
    path: &'a Path,
    canonical_path: &'a Path,
    status: &'static str,
    shadowed_by: Option<String>,
    trust: TrustState,
    info: Option<sisho_sdk::ExtensionInfo>,
}

/// Run `sisho --list`: scan `PATH` and print installed extensions.
pub fn list(all: bool, verbose: bool, json: bool) -> ExitCode {
    let result = with_standard_streams(|out, _err| run_list(out, all, verbose, json));
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
pub fn run_list(out: &mut dyn Write, all: bool, verbose: bool, json: bool) -> Result<()> {
    let entries = scan();
    if json {
        format_entries_json(out, &entries, all, verbose)
    } else {
        format_entries(out, &entries, all, verbose)
    }
}

fn filter_and_sort(entries: &[ExtensionEntry], all: bool) -> Vec<&ExtensionEntry> {
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
    visible
}

/// Format and write `entries` as JSON to `out`.
pub fn format_entries_json(
    out: &mut dyn Write,
    entries: &[ExtensionEntry],
    all: bool,
    verbose: bool,
) -> Result<()> {
    let visible = filter_and_sort(entries, all);
    let mut json_entries = Vec::with_capacity(visible.len());

    for entry in visible {
        let (status, shadowed_by) = match &entry.status {
            ExtensionStatus::Active => ("active", None),
            ExtensionStatus::ShadowedByBuiltin => {
                ("shadowed_by_builtin", Some("builtin".to_string()))
            }
            ExtensionStatus::ShadowedByDuplicate(orig) => (
                "shadowed_by_duplicate",
                Some(orig.display().to_string()),
            ),
        };
        let info = if verbose
            && entry.status == ExtensionStatus::Active
            && entry.trust == TrustState::Allowed
        {
            query_info(&entry.word, &entry.path).filter(|info| info.word == entry.word)
        } else {
            None
        };

        json_entries.push(JsonExtensionEntry {
            word: &entry.word,
            path: &entry.path,
            canonical_path: &entry.canonical_path,
            status,
            shadowed_by,
            trust: entry.trust,
            info,
        });
    }

    let rendered = serde_json::to_string_pretty(&json_entries)?;
    writeln!(out, "{rendered}")?;
    Ok(())
}

/// Format and write `entries` to `out`.
pub fn format_entries(
    out: &mut dyn Write,
    entries: &[ExtensionEntry],
    all: bool,
    verbose: bool,
) -> Result<()> {
    let visible = filter_and_sort(entries, all);

    let max_word_len = visible.iter().map(|e| e.word.len()).max().unwrap_or(0);
    let width = max_word_len.max(8);

    for entry in visible {
        match &entry.status {
            ExtensionStatus::Active => {
                writeln!(
                    out,
                    "{:width$}  {} [{}]",
                    entry.word,
                    entry.path.display(),
                    entry.trust
                )?;
                if verbose
                    && entry.trust == TrustState::Allowed
                    && let Some(info) = query_info(&entry.word, &entry.path)
                    && info.word == entry.word
                {
                    let mut details = Vec::new();
                    if !info.version.is_empty() {
                        let ver = if info.version.starts_with('v') || info.version.starts_with('V')
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

/// Update the trust state of `<target>` in `trust.json`.
///
/// `<target>` may be an extension word resolved via the active `PATH` entry
/// (e.g. `desk` or `sisho-desk`), or an explicit path to a `sisho-<word>` executable.
pub fn set_trust(target: &str, state: TrustState) -> ExitCode {
    let mut config = TrustConfig::load();
    let Some((word, canonical_path)) = resolve_target(target, &config, state) else {
        eprintln!("sisho: extension '{target}' not found on PATH");
        return ExitCode::from(crate::USAGE_ERROR);
    };

    config.set_state(&canonical_path, state);
    if let Err(error) = config.save() {
        eprintln!("sisho: {error:#}");
        return ExitCode::from(crate::OWN_ERROR);
    }

    let result = with_standard_streams(|out, _err| {
        match state {
            TrustState::Allowed => {
                writeln!(
                    out,
                    "Allowed sisho-{word} ({})",
                    canonical_path.display()
                )?;
            }
            TrustState::Denied => {
                writeln!(out, "Denied sisho-{word} ({})", canonical_path.display())?;
            }
            TrustState::Untrusted => {
                writeln!(
                    out,
                    "Reset trust for sisho-{word} ({}) to untrusted",
                    canonical_path.display()
                )?;
            }
        }
        Ok(())
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if is_broken_pipe(&error) => ExitCode::from(BROKEN_PIPE),
        Err(error) => {
            eprintln!("sisho: {error:#}");
            ExitCode::from(crate::OWN_ERROR)
        }
    }
}

fn resolve_target(
    target: &str,
    config: &TrustConfig,
    state: TrustState,
) -> Option<(String, PathBuf)> {
    if target.is_empty() {
        return None;
    }

    // If `target` contains a path separator or refers directly to an existing file on disk
    let looks_like_path =
        target.contains('/') || target.contains(std::path::MAIN_SEPARATOR) || target.starts_with('.');
    if looks_like_path {
        let path = Path::new(target);
        let file_name = path.file_name()?.to_str()?;
        let word = file_name.strip_prefix("sisho-")?;
        if !is_valid_word(word) {
            return None;
        }
        let canonical = canonicalize_path(path);
        if is_executable(path)
            || (state == TrustState::Untrusted && config.binaries.contains_key(&canonical))
        {
            return Some((word.to_string(), canonical));
        }
        return None;
    }

    // Otherwise resolve by extension word (or `sisho-<word>`) on PATH
    let entries = scan_with_trust(config);
    if is_valid_word(target)
        && let Some(entry) = entries
            .iter()
            .find(|e| e.status == ExtensionStatus::Active && e.word == target)
    {
        return Some((entry.word.clone(), entry.canonical_path.clone()));
    }
    if let Some(stripped) = target.strip_prefix("sisho-")
        && is_valid_word(stripped)
        && let Some(entry) = entries
            .iter()
            .find(|e| e.status == ExtensionStatus::Active && e.word == stripped)
    {
        return Some((entry.word.clone(), entry.canonical_path.clone()));
    }

    None
}

/// Find the active extension entry on `PATH` for `word`.
pub fn find_active(word: &str) -> Option<ExtensionEntry> {
    scan()
        .into_iter()
        .find(|entry| entry.status == ExtensionStatus::Active && entry.word == word)
}

/// Scan directories on `PATH` for extensions.
pub fn scan() -> Vec<ExtensionEntry> {
    let trust_config = TrustConfig::load();
    scan_with_trust(&trust_config)
}

/// Scan directories on `PATH` for extensions with a pre-loaded [`TrustConfig`].
pub fn scan_with_trust(trust_config: &TrustConfig) -> Vec<ExtensionEntry> {
    let path_var = env::var_os("PATH").unwrap_or_default();
    scan_in_with_trust(env::split_paths(&path_var), trust_config)
}

/// Scan specific directories for extensions with a pre-loaded [`TrustConfig`].
pub fn scan_in_with_trust(
    dirs: impl IntoIterator<Item = PathBuf>,
    trust_config: &TrustConfig,
) -> Vec<ExtensionEntry> {
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

            let canonical_path = canonicalize_path(&path);
            let trust = trust_config.state_for_canonical(&canonical_path);

            entries.push(ExtensionEntry {
                word: word.to_string(),
                path,
                canonical_path,
                status,
                trust,
                order: counter,
            });
        }
    }

    entries
}

fn is_builtin_command_or_tool(word: &str) -> bool {
    matches!(word, "check" | "fix" | "convert" | "serve" | "spec")
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

/// Compute `SISHO_CONFIG_DIR` and `SISHO_DATA_DIR` environment variable pairs for `word`.
///
/// Resolves `$XDG_CONFIG_HOME/sisho/<word>` (fallback `$HOME/.config/sisho/<word>`) and
/// `$XDG_DATA_HOME/sisho/<word>` (fallback `$HOME/.local/share/sisho/<word>`), ignoring any
/// inherited `SISHO_CONFIG_DIR` or `SISHO_DATA_DIR` from a parent process so an outer
/// extension's directory never leaks into another extension.
pub fn extension_dir_envs(word: &str) -> Vec<(&'static str, PathBuf)> {
    extension_dir_envs_with(word, |key| env::var_os(key))
}

fn extension_dir_envs_with<F>(word: &str, lookup: F) -> Vec<(&'static str, PathBuf)>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let lookup_without_sisho_override = |key: &str| match key {
        sisho_sdk::CONFIG_DIR_ENV | sisho_sdk::DATA_DIR_ENV => None,
        _ => lookup(key),
    };
    let mut envs = Vec::with_capacity(2);
    if let Some(dir) = sisho_sdk::resolve_config_dir_with(word, lookup_without_sisho_override) {
        envs.push((sisho_sdk::CONFIG_DIR_ENV, dir));
    }
    if let Some(dir) = sisho_sdk::resolve_data_dir_with(word, lookup_without_sisho_override) {
        envs.push((sisho_sdk::DATA_DIR_ENV, dir));
    }
    envs
}

/// Query an extension executable for its metadata using `--sisho-extension-info`.
///
/// Enforces a strict 500ms timeout and discards output on failure or timeout.
/// Returns `None` if the executable does not support the protocol or fails.
pub fn query_info(word: &str, path: &Path) -> Option<sisho_sdk::ExtensionInfo> {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);
    const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

    let mut command = std::process::Command::new(path);
    command
        .arg(sisho_sdk::EXTENSION_INFO_FLAG)
        .envs(extension_dir_envs(word))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    let mut child = command.spawn().ok()?;

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
        assert_eq!(
            query_info("tool", Path::new("/non/existent/sisho-tool")),
            None
        );
    }

    #[test]
    fn extension_dir_envs_ignores_parent_sisho_dir_overrides() {
        use std::ffi::OsString;

        let envs = extension_dir_envs_with("echo", |k| match k {
            "SISHO_CONFIG_DIR" => Some(OsString::from("/parent/config/board")),
            "SISHO_DATA_DIR" => Some(OsString::from("/parent/data/board")),
            "XDG_CONFIG_HOME" => Some(OsString::from("/xdg/config")),
            "XDG_DATA_HOME" => Some(OsString::from("/xdg/data")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(
            envs,
            vec![
                (
                    "SISHO_CONFIG_DIR",
                    PathBuf::from("/xdg/config/sisho/echo")
                ),
                ("SISHO_DATA_DIR", PathBuf::from("/xdg/data/sisho/echo")),
            ]
        );

        let fallback_envs = extension_dir_envs_with("echo", |k| match k {
            "SISHO_CONFIG_DIR" => Some(OsString::from("/parent/config/board")),
            "SISHO_DATA_DIR" => Some(OsString::from("/parent/data/board")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(
            fallback_envs,
            vec![
                (
                    "SISHO_CONFIG_DIR",
                    PathBuf::from("/home/alice/.config/sisho/echo")
                ),
                (
                    "SISHO_DATA_DIR",
                    PathBuf::from("/home/alice/.local/share/sisho/echo")
                ),
            ]
        );
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
