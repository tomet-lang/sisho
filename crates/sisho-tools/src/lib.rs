//! Layer 1: what sisho knows about the core tools.
//!
//! For `check` and `fix`, which invocation of `tomet`, `twrit` and `tandr`
//! to run, and how a pass-through `sisho <word> ...` resolves (see
//! [`Program`]: the tools, and `tomet-shelf`, which is not one of them). The tools are
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
//!
//! `sisho convert` and `sisho serve` are not in a plan but are calls of the
//! same kind: a [`Conversion`] says which `tomet` command converts which way,
//! and `tomet` does the converting.

mod convert;
mod plan;
mod program;
mod tool;
mod version;

#[cfg(test)]
mod tests;

pub use convert::{Conversion, Direction, Source, Target};
pub use plan::{check_steps, fix_steps};
pub use program::{Program, SHELF, program, programs};
pub use tool::Tool;
pub use version::{Verdict, Version, judge};
