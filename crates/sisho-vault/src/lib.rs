//! Layer 1: the vault commands that `sisho` does itself.
//!
//! `check-links` and `convert`. Unlike `sisho-tools`, these use the `tomet`
//! crates directly, through git dependencies: `sisho` is the part of the
//! ecosystem that knows what a vault is, as `cargo` knows what a workspace is.
//!
//! What belongs to the language stays in `tomet`, where its rules live: `new`
//! (it uses the expression evaluator), `refactor` (the migration rules change
//! with the language) and `export` (it follows the `@config(export: ...)` a
//! document declares for itself). `sisho tomet new ...` reaches them.
//!
//! Each command is a module that does the work and returns what happened as
//! data. Printing is the program's: this crate writes nothing to the terminal,
//! so the same work can be shown as text or as JSON. The `tomet` types stay
//! inside; the program does not depend on `tomet` crates.

pub mod convert;
pub mod diagnostic;
pub mod links;
pub mod render;
