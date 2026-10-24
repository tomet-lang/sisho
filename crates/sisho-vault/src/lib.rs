//! Layer 1: the vault commands that `sisho` does itself.
//!
//! `check-links`. Unlike `sisho-tools`, this uses the `tomet` crates
//! directly, through git dependencies: `sisho` is the part of the ecosystem
//! that knows what a vault is, as `cargo` knows what a workspace is. It
//! follows a vault's sweeps and caches, which a document alone does not
//! decide.
//!
//! What belongs to the language stays in `tomet`, where its rules live: `new`
//! (it uses the expression evaluator), `refactor` (the migration rules change
//! with the language), `export` (it follows the `@config(export: ...)` a
//! document declares for itself) and the conversions (a document alone decides
//! them; `sisho convert` asks `tomet` for them). `sisho tomet new ...` reaches
//! the commands that are not `sisho`'s.
//!
//! Each command is a module that does the work and returns what happened as
//! data. Printing is the program's: this crate writes nothing to the terminal,
//! so the same work can be shown as text or as JSON. The `tomet` types stay
//! inside; the program does not depend on `tomet` crates.

pub mod diagnostic;
pub mod links;
