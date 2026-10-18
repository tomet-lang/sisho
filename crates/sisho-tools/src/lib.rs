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
//! Only the steps that run a tool are here. If `sisho` does a step itself
//! one day, in-process, the program puts it in the plan beside these.

mod plan;
mod tool;
mod version;

#[cfg(test)]
mod tests;

pub use plan::{check_steps, fix_steps};
pub use tool::Tool;
pub use version::{Verdict, Version, judge};
