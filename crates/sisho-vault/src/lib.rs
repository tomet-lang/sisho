//! Layer 1: the vault and workspace commands that `tomet` hands over.
//!
//! `new`, `export`, `check-links`, `refactor` and `convert`. Unlike
//! `sisho-tools`, these use the `tomet` crates directly, through git
//! dependencies.
//!
//! Each command is a module that does the work and returns what happened as
//! data. Printing is the program's: this crate writes nothing to the terminal,
//! so the same work can be shown as text, as JSON, or run as a step of
//! `check` and `fix`. The `tomet` types stay inside; the program does not
//! depend on `tomet` crates.

pub mod diagnostic;
pub mod export;
pub mod links;
pub mod new;
pub mod refactor;
pub mod render;
