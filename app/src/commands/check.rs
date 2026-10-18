//! `sisho check`.

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use sisho_run::{Job, Report, run_parallel};
use sisho_tools::check_steps;

use super::{exit_code, tool_versions, version_problems, version_steps, write_json};
use crate::cli::RunArgs;
use crate::report;

/// Every step at the same time, reported in plan order. None of them writes,
/// so none has to wait for another.
///
/// A tool older than `sisho` works with is a failed step of its own, first in
/// the report, and the tool's steps still run: they only read, and what they
/// say is more use than silence.
pub fn check(args: &RunArgs) -> Result<ExitCode> {
    let versions = version_steps();
    let steps = check_steps(&args.path);

    let jobs = versions.iter().chain(&steps).map(Job::step).collect();
    let mut outcomes = run_parallel(jobs);
    let tool_outcomes = outcomes.split_off(versions.len());
    let mut step_outcomes = version_problems(&outcomes);
    step_outcomes.extend(tool_outcomes);
    let versions = tool_versions(&outcomes);

    let mut out = io::stdout().lock();
    if args.json {
        write_json(&mut out, args, &versions, &step_outcomes)?;
    } else {
        write!(out, "{}", report::header("check", &args.path, &versions))?;
        for outcome in &step_outcomes {
            write!(out, "{}", report::outcome(outcome, args.verbose))?;
        }
        write!(out, "{}", report::summary(&step_outcomes))?;
    }
    Ok(exit_code(&Report::new(step_outcomes)))
}
