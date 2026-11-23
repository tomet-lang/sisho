//! Standard configuration and data directory resolution for `sisho` extensions.
//!
//! When `sisho` invokes an allowed extension `sisho-<word>`, it sets
//! [`CONFIG_DIR_ENV`] (`SISHO_CONFIG_DIR`) and [`DATA_DIR_ENV`] (`SISHO_DATA_DIR`)
//! in the child process environment. Extensions can call [`config_dir`] and
//! [`data_dir`] to resolve their isolated directories whether invoked via
//! `sisho <word>` or directly as `sisho-<word>`.

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Environment variable set by `sisho` pointing to the extension's configuration directory
/// (`$XDG_CONFIG_HOME/sisho/<word>` or `$HOME/.config/sisho/<word>`).
pub const CONFIG_DIR_ENV: &str = "SISHO_CONFIG_DIR";

/// Environment variable set by `sisho` pointing to the extension's data directory
/// (`$XDG_DATA_HOME/sisho/<word>` or `$HOME/.local/share/sisho/<word>`).
pub const DATA_DIR_ENV: &str = "SISHO_DATA_DIR";

/// Resolve the configuration directory for the extension named `word`.
///
/// Precedence:
/// 1. `SISHO_CONFIG_DIR` if set and non-empty.
/// 2. `$XDG_CONFIG_HOME/sisho/<word>` if `XDG_CONFIG_HOME` is set and non-empty.
/// 3. `$HOME/.config/sisho/<word>` if `HOME` is set and non-empty.
/// 4. `None` otherwise.
pub fn config_dir(word: &str) -> Option<PathBuf> {
    resolve_config_dir_with(word, |key| env::var_os(key))
}

/// Resolve the data directory for the extension named `word`.
///
/// Precedence:
/// 1. `SISHO_DATA_DIR` if set and non-empty.
/// 2. `$XDG_DATA_HOME/sisho/<word>` if `XDG_DATA_HOME` is set and non-empty.
/// 3. `$HOME/.local/share/sisho/<word>` if `HOME` is set and non-empty.
/// 4. `None` otherwise.
pub fn data_dir(word: &str) -> Option<PathBuf> {
    resolve_data_dir_with(word, |key| env::var_os(key))
}

/// Resolve the configuration directory for `word` using a custom environment lookup function.
pub fn resolve_config_dir_with<F>(word: &str, lookup: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    resolve_dir_with(word, CONFIG_DIR_ENV, "XDG_CONFIG_HOME", &[".config"], lookup)
}

/// Resolve the data directory for `word` using a custom environment lookup function.
pub fn resolve_data_dir_with<F>(word: &str, lookup: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    resolve_dir_with(
        word,
        DATA_DIR_ENV,
        "XDG_DATA_HOME",
        &[".local", "share"],
        lookup,
    )
}

fn is_non_empty(value: &OsStr) -> bool {
    match value.to_str() {
        Some(s) => !s.trim().is_empty(),
        None => !value.is_empty(),
    }
}

fn resolve_dir_with<F>(
    word: &str,
    override_env: &str,
    xdg_env: &str,
    home_subpath: &[&str],
    lookup: F,
) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    if let Some(dir) = lookup(override_env).filter(|v| is_non_empty(v)) {
        return Some(PathBuf::from(dir));
    }
    if let Some(xdg) = lookup(xdg_env).filter(|v| is_non_empty(v)) {
        return Some(PathBuf::from(xdg).join("sisho").join(word));
    }
    if let Some(home) = lookup("HOME").filter(|v| is_non_empty(v)) {
        let mut path = PathBuf::from(home);
        for part in home_subpath {
            path.push(part);
        }
        return Some(path.join("sisho").join(word));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_prefers_sisho_config_dir_then_xdg_then_home() {
        let with_override = resolve_config_dir_with("board", |k| match k {
            "SISHO_CONFIG_DIR" => Some(OsString::from("/custom/config/board")),
            "XDG_CONFIG_HOME" => Some(OsString::from("/xdg/config")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(with_override, Some(PathBuf::from("/custom/config/board")));

        let with_xdg = resolve_config_dir_with("board", |k| match k {
            "SISHO_CONFIG_DIR" => Some(OsString::from("")),
            "XDG_CONFIG_HOME" => Some(OsString::from("/xdg/config")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(with_xdg, Some(PathBuf::from("/xdg/config/sisho/board")));

        let with_whitespace_override = resolve_config_dir_with("board", |k| match k {
            "SISHO_CONFIG_DIR" => Some(OsString::from("   \t")),
            "XDG_CONFIG_HOME" => Some(OsString::from("  ")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(
            with_whitespace_override,
            Some(PathBuf::from("/home/alice/.config/sisho/board"))
        );

        let with_home = resolve_config_dir_with("board", |k| match k {
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(
            with_home,
            Some(PathBuf::from("/home/alice/.config/sisho/board"))
        );

        let with_none = resolve_config_dir_with("board", |_| None);
        assert_eq!(with_none, None);
    }

    #[test]
    fn data_dir_prefers_sisho_data_dir_then_xdg_then_home() {
        let with_override = resolve_data_dir_with("desk", |k| match k {
            "SISHO_DATA_DIR" => Some(OsString::from("/custom/data/desk")),
            "XDG_DATA_HOME" => Some(OsString::from("/xdg/data")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(with_override, Some(PathBuf::from("/custom/data/desk")));

        let with_xdg = resolve_data_dir_with("desk", |k| match k {
            "SISHO_DATA_DIR" => Some(OsString::from("")),
            "XDG_DATA_HOME" => Some(OsString::from("/xdg/data")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(with_xdg, Some(PathBuf::from("/xdg/data/sisho/desk")));

        let with_whitespace_override = resolve_data_dir_with("desk", |k| match k {
            "SISHO_DATA_DIR" => Some(OsString::from("  \n")),
            "XDG_DATA_HOME" => Some(OsString::from("")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(
            with_whitespace_override,
            Some(PathBuf::from("/home/alice/.local/share/sisho/desk"))
        );

        let with_home = resolve_data_dir_with("desk", |k| match k {
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(
            with_home,
            Some(PathBuf::from("/home/alice/.local/share/sisho/desk"))
        );

        let with_none = resolve_data_dir_with("desk", |_| None);
        assert_eq!(with_none, None);
    }
}
