//! Machine-local extension trust database (`$XDG_DATA_HOME/sisho/trust.json`).
//!
//! `sisho` is the sole owner and gatekeeper of extension execution trust.
//! Every discovered `sisho-<word>` executable on `PATH` is keyed by its
//! canonicalized file path and defaults to [`TrustState::Untrusted`] until
//! explicitly allowed (`sisho --allow <target>`) or denied (`sisho --deny <target>`).

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

/// Current schema version for `trust.json`.
pub const TRUST_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    TRUST_SCHEMA_VERSION
}

/// Trust state of an extension binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustState {
    /// Default state for an extension binary not listed in `trust.json`.
    /// `sisho` will refuse to execute it or query `--sisho-extension-info`.
    #[default]
    Untrusted,
    /// Explicitly allowed by the user via `sisho --allow`.
    Allowed,
    /// Explicitly denied by the user via `sisho --deny`.
    Denied,
}

impl TrustState {
    /// Lowercase string representation (`"untrusted"`, `"allowed"`, `"denied"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Untrusted => "untrusted",
            Self::Allowed => "allowed",
            Self::Denied => "denied",
        }
    }
}

impl fmt::Display for TrustState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Persistent extension trust database stored at `$XDG_DATA_HOME/sisho/trust.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustConfig {
    /// Schema version of the trust file (currently `1`).
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Map of canonical binary paths to their configured trust state (`allowed` or `denied`).
    #[serde(default)]
    pub binaries: BTreeMap<PathBuf, TrustState>,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            schema_version: TRUST_SCHEMA_VERSION,
            binaries: BTreeMap::new(),
        }
    }
}

/// Canonicalize an executable path, falling back to `path.to_path_buf()` if
/// filesystem resolution fails.
pub fn canonicalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Resolve the path to `trust.json`.
///
/// Precedence:
/// 1. `SISHO_TRUST_FILE` if set and non-empty.
/// 2. `$XDG_DATA_HOME/sisho/trust.json` if `XDG_DATA_HOME` is set and non-empty.
/// 3. `$HOME/.local/share/sisho/trust.json` if `HOME` is set and non-empty.
pub fn default_trust_path() -> Option<PathBuf> {
    if let Some(override_path) = env::var_os("SISHO_TRUST_FILE")
        && !override_path.is_empty()
    {
        return Some(PathBuf::from(override_path));
    }
    if let Some(xdg) = env::var_os("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("sisho").join("trust.json"));
    }
    if let Some(home) = env::var_os("HOME")
        && !home.is_empty()
    {
        return Some(
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("sisho")
                .join("trust.json"),
        );
    }
    None
}

impl TrustConfig {
    /// Load the trust configuration from the default path, returning an empty
    /// configuration if the file does not exist or cannot be read.
    pub fn load() -> Self {
        let Some(path) = default_trust_path() else {
            return Self::default();
        };
        Self::load_from(&path).unwrap_or_default()
    }

    /// Load the trust configuration from a specific path.
    ///
    /// Returns [`TrustConfig::default()`] if the file does not exist.
    pub fn load_from(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(content) => {
                let config: Self = serde_json::from_str(&content)
                    .with_context(|| format!("failed to parse {}", path.display()))?;
                Ok(config)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    /// Save the trust configuration to the default path.
    pub fn save(&self) -> Result<PathBuf> {
        let path = default_trust_path().ok_or_else(|| {
            anyhow!("cannot determine trust database path (neither SISHO_TRUST_FILE, XDG_DATA_HOME, nor HOME is set)")
        })?;
        self.save_to(&path)?;
        Ok(path)
    }

    /// Save the trust configuration to a specific path, creating parent directories if needed.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        let mut json = serde_json::to_string_pretty(self)
            .context("failed to serialize trust configuration")?;
        json.push('\n');
        fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    /// Look up the [`TrustState`] for an already-canonicalized executable path.
    pub fn state_for_canonical(&self, canonical_path: &Path) -> TrustState {
        self.binaries
            .get(canonical_path)
            .copied()
            .unwrap_or(TrustState::Untrusted)
    }

    /// Update the [`TrustState`] for an executable path (canonicalizing it first).
    ///
    /// Setting [`TrustState::Untrusted`] removes the entry from `binaries`.
    /// Returns the canonicalized path.
    pub fn set_state(&mut self, path: &Path, state: TrustState) -> PathBuf {
        let canonical = canonicalize_path(path);
        match state {
            TrustState::Untrusted => {
                self.binaries.remove(&canonical);
            }
            TrustState::Allowed | TrustState::Denied => {
                self.binaries.insert(canonical.clone(), state);
            }
        }
        canonical
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_state_defaults_to_untrusted_and_roundtrips() {
        let mut config = TrustConfig::default();
        let path = Path::new("/nix/store/xxx-sisho-desk/bin/sisho-desk");
        assert_eq!(config.state_for_canonical(path), TrustState::Untrusted);

        config.binaries.insert(path.to_path_buf(), TrustState::Allowed);
        assert_eq!(config.state_for_canonical(path), TrustState::Allowed);

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"/nix/store/xxx-sisho-desk/bin/sisho-desk\": \"allowed\""));

        let parsed: TrustConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn setting_untrusted_removes_entry_from_binaries() {
        let mut config = TrustConfig::default();
        let path = Path::new("/tmp/nonexistent-sisho-foo");
        let canonical = config.set_state(path, TrustState::Denied);
        assert_eq!(config.state_for_canonical(&canonical), TrustState::Denied);

        config.set_state(path, TrustState::Untrusted);
        assert_eq!(config.state_for_canonical(&canonical), TrustState::Untrusted);
        assert!(config.binaries.is_empty());
    }
}
