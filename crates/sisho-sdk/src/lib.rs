//! Layer 0: SDK and protocol definitions for `sisho` extensions.
//!
//! An extension is an independent subprocess on `PATH` named `sisho-<word>`.
//! This crate provides optional, minimal helpers for extension developers:
//! - [`info`]: Metadata disclosure protocol (`--sisho-extension-info`).
//! - [`jobserver`]: Cooperative parallelism via GNU Jobserver tokens.

pub mod info;
pub mod jobserver;

pub use info::{
    CURRENT_SCHEMA_VERSION, Capabilities, EXTENSION_INFO_FLAG, ExtensionInfo,
};
pub use jobserver::{JobServerGate, JobToken};
