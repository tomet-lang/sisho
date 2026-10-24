//! Layer 0: SDK and protocol definitions for `sisho` extensions.
//!
//! An extension is an independent subprocess on `PATH` named `sisho-<word>`.
//! This crate provides definitions and utilities for extension developers:
//! - [`info`]: Metadata disclosure protocol (`--sisho-extension-info`).
//! - [`jobserver`]: GNU Jobserver coordination via [`jobserver_gate`].

pub mod info;
pub use jobserver_gate as jobserver;

pub use info::{CURRENT_SCHEMA_VERSION, Capabilities, EXTENSION_INFO_FLAG, ExtensionInfo};
pub use jobserver::{JobServerGate, JobToken};
