//! One file per command, and what they share.
//!
//! The work of the vault commands is `sisho-vault`'s and returns data; a
//! command here shows it. A command that could not do what was asked prints
//! `sisho: <reason>` and exits 1, as `tomet` does, so a script that moves from
//! a `tomet` command to the `sisho` one keeps working.

pub mod check;
pub mod check_links;
pub mod convert;
pub mod fix;
pub mod serve;
pub mod tool;
pub mod tui;

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use sisho_run::{Outcome, Report, Step};
use sisho_tools::Tool;

use crate::cli::RunArgs;
use crate::output::{BROKEN_PIPE, is_broken_pipe};
use crate::report::{self, ToolVersion};

/// The exit code of a vault command that failed.
pub const FAILED: u8 = 1;

/// Where a command writes: its standard output and its standard error.
pub type Sink<'a> = &'a mut dyn Write;

/// Run `run` with the real standard output and standard error.
pub fn with_standard_streams<T>(run: impl FnOnce(Sink<'_>, Sink<'_>) -> Result<T>) -> Result<T> {
    let (mut out, mut err) = (io::stdout().lock(), io::stderr().lock());
    run(&mut out, &mut err)
}

/// The exit code of a command that either worked or did not.
pub fn finish(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => failed(&error),
    }
}

/// A command that did not do what was asked. If the reason is only that
/// nobody is reading the output any more, there is nothing to say.
pub fn failed(error: &anyhow::Error) -> ExitCode {
    if is_broken_pipe(error) {
        return ExitCode::from(BROKEN_PIPE);
    }
    let _ = writeln!(io::stderr(), "sisho: {error:#}");
    ExitCode::from(FAILED)
}

// What `check` and `fix` share: they report on the same tools in the same way.

/// `<tool> --version` for every tool, for the header of the report.
pub fn version_steps() -> Vec<Step> {
    Tool::ALL.iter().map(|tool| tool.version_step()).collect()
}

/// The versions the tools reported, from the outcomes of [`version_steps`].
pub fn tool_versions(outcomes: &[Outcome]) -> Vec<ToolVersion> {
    Tool::ALL
        .iter()
        .zip(outcomes)
        .map(|(tool, outcome)| ToolVersion::from_outcome(*tool, outcome))
        .collect()
}

/// The run as JSON on `out`.
pub fn write_json(
    out: &mut impl Write,
    args: &RunArgs,
    versions: &[ToolVersion],
    outcomes: &[Outcome],
) -> Result<()> {
    serde_json::to_writer_pretty(&mut *out, &report::json(&args.path, versions, outcomes))?;
    writeln!(out)?;
    Ok(())
}

/// 0 if every step passed, 1 if any did not.
pub fn exit_code(report: &Report) -> ExitCode {
    ExitCode::from(report.exit_code())
}
