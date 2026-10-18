//! Layer 1: what sisho knows about the core tools.
//!
//! For `check` and `fix`, which invocation of `tomet`, `twrit` and `tandr`
//! to run, and how a pass-through `sisho <tool> ...` resolves. The tools are
//! called as subprocesses through `sisho-run`, never linked as crates (see
//! the `tools-by-subprocess` rule in the workspace `.writ.tmt`).
//!
//! This is a table, written by hand. A tool is not asked what it can do, and
//! a project does not say which tools apply: each tool decides for itself
//! whether there is anything for it to look at, and says so when there is not
//! (`twrit` prints `no .writ.tmt found under .`, `tomet` prints
//! `OK: 0 file(s)`). So every tool here runs every time.
//!
//! Only the steps that run a tool are here. `sisho`'s own checks and repairs
//! (`export --check`, `export`) are not tool steps; the program puts them in
//! the plan beside these. `fix` is split in two around `export` for that
//! reason.

mod plan;
mod tool;

#[cfg(test)]
mod tests;

pub use plan::{check_steps, fix_format_steps, fix_generate_steps};
pub use tool::Tool;
