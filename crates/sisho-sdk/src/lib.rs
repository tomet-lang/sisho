//! Layer 0: SDK and protocol definitions for `sisho` extensions.
//!
//! An extension is an independent subprocess on `PATH` named `sisho-<word>`.
//! This crate provides definitions and utilities for extension developers:
//! - [`dirs`]: Standard configuration and data directory resolution (`SISHO_CONFIG_DIR`, `SISHO_DATA_DIR`).
//! - [`info`]: Metadata disclosure protocol (`--sisho-extension-info`).
//! - [`jobserver`]: GNU Jobserver coordination via [`jobserver_gate`].

pub mod dirs;
pub mod info;
pub use jobserver_gate as jobserver;

pub use dirs::{
    CONFIG_DIR_ENV, DATA_DIR_ENV, config_dir, data_dir, resolve_config_dir_with,
    resolve_data_dir_with,
};
pub use info::{CURRENT_SCHEMA_VERSION, Capabilities, EXTENSION_INFO_FLAG, ExtensionInfo};
pub use jobserver::{JobServerGate, JobToken};
