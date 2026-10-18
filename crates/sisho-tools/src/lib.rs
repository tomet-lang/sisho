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
//! Only commands the tools still own are listed. `export`, `check-links` and
//! `refactor` join the plans as in-process steps once they have moved from
//! `tomet` into `sisho-vault`.

mod plan;
mod tool;

#[cfg(test)]
mod tests;

pub use plan::{check_steps, fix_steps};
pub use tool::Tool;
