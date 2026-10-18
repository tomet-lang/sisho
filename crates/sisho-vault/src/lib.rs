//! Layer 1: the vault and workspace commands that `tomet` hands over.
//!
//! `export`, `check-links` and `convert`. Unlike `sisho-tools`, these use the
//! `tomet` crates directly, through git dependencies: `sisho` is the part of
//! the ecosystem that knows what a vault is, as `cargo` knows what a
//! workspace is.
//!
//! `new` and `refactor` stay in `tomet`, where the language's own rules live;
//! `sisho tomet new ...` reaches them.
//!
//! Each command is a module that does the work and returns what happened as
//! data. Printing is the program's: this crate writes nothing to the terminal,
//! so the same work can be shown as text, as JSON, or run as a step of
//! `check` and `fix`. The `tomet` types stay inside; the program does not
//! depend on `tomet` crates.

pub mod convert;
pub mod diagnostic;
pub mod export;
pub mod links;
pub mod render;
