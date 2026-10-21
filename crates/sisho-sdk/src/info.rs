//! Extension metadata disclosure protocol (`--sisho-extension-info`).
//!
//! An extension can disclose its name, version, description, and capabilities
//! by responding to `--sisho-extension-info` with a single JSON object.

use std::collections::BTreeMap;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

/// Current schema version for extension metadata.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// The standard CLI flag that requests extension metadata.
pub const EXTENSION_INFO_FLAG: &str = "--sisho-extension-info";

/// Metadata disclosed by an extension when queried with `--sisho-extension-info`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionInfo {
    /// Schema version, currently 1.
    pub schema_version: u32,
    /// The extension word (e.g. "desk" for `sisho-desk`).
    pub word: String,
    /// Extension version string (e.g. "0.1.0").
    pub version: String,
    /// One-line description of the extension.
    pub about: String,
    /// Feature capabilities supported by this extension.
    #[serde(default, skip_serializing_if = "Capabilities::is_empty")]
    pub capabilities: Capabilities,
    /// Additional custom metadata fields for future extensibility.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl ExtensionInfo {
    /// Create a new `ExtensionInfo` with schema version 1 and default capabilities.
    pub fn new(
        word: impl Into<String>,
        version: impl Into<String>,
        about: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            word: word.into(),
            version: version.into(),
            about: about.into(),
            capabilities: Capabilities::default(),
            extra: BTreeMap::new(),
        }
    }

    /// Set capabilities for this extension.
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Serialize to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize to a compact JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse `ExtensionInfo` from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Parse `ExtensionInfo` from an arbitrary reader.
    pub fn from_reader<R: io::Read>(reader: R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    /// Write pretty JSON to `writer` followed by a newline and flush.
    pub fn print_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        serde_json::to_writer_pretty(&mut writer, self)?;
        writer.write_all(b"\n")?;
        writer.flush()
    }

    /// Write pretty JSON to standard output followed by a newline and flush.
    pub fn print(&self) -> io::Result<()> {
        self.print_to(io::stdout().lock())
    }
}

/// Feature capabilities disclosed by an extension.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Whether this extension provides an interactive TUI.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tui: bool,
    /// Whether this extension requires a controlling terminal (interactive).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub interactive: bool,
    /// Additional capability flags for future extensions.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Capabilities {
    /// Create empty capabilities.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether TUI is supported.
    pub fn with_tui(mut self, tui: bool) -> Self {
        self.tui = tui;
        self
    }

    /// Set whether controlling terminal interactivity is required.
    pub fn with_interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Returns true if all known capabilities are false and there are no extra fields.
    pub fn is_empty(&self) -> bool {
        !self.tui && !self.interactive && self.extra.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_roundtrip() {
        let info = ExtensionInfo::new(
            "desk",
            "0.1.0",
            "Interactive desk for Tomet: migration, batch metadata, and search",
        )
        .with_capabilities(Capabilities::new().with_tui(true).with_interactive(true));

        let json = info.to_json_pretty().unwrap();
        let parsed = ExtensionInfo::from_json_str(&json).unwrap();
        assert_eq!(info, parsed);
        assert_eq!(parsed.schema_version, 1);
        assert_eq!(parsed.word, "desk");
        assert!(parsed.capabilities.tui);
        assert!(parsed.capabilities.interactive);
    }

    #[test]
    fn omit_empty_capabilities() {
        let info = ExtensionInfo::new("tool", "1.0.0", "A simple CLI tool");
        let json = info.to_json().unwrap();
        assert!(!json.contains("capabilities"));

        let parsed = ExtensionInfo::from_json_str(&json).unwrap();
        assert_eq!(info, parsed);
        assert!(!parsed.capabilities.tui);
    }

    #[test]
    fn preserves_unknown_extra_fields() {
        let json = r#"{
            "schema_version": 1,
            "word": "custom",
            "version": "2.0.0",
            "about": "Custom tool",
            "future_field": "preserved",
            "capabilities": {
                "tui": true,
                "unknown_cap": 42
            }
        }"#;

        let parsed = ExtensionInfo::from_json_str(json).unwrap();
        assert_eq!(parsed.word, "custom");
        assert!(parsed.capabilities.tui);
        assert_eq!(
            parsed.extra.get("future_field"),
            Some(&serde_json::json!("preserved"))
        );
        assert_eq!(
            parsed.capabilities.extra.get("unknown_cap"),
            Some(&serde_json::json!(42))
        );
    }
}
