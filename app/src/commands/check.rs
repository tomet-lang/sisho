//! `sisho check`.

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use sisho_run::{Job, Report, run_parallel};
use sisho_tools::check_steps;

use super::{exit_code, tool_versions, version_steps, write_json};
use crate::cli::RunArgs;
use crate::report;

/// Every step at the same time, reported in plan order. None of them writes,
/// so none has to wait for another.
pub fn check(args: &RunArgs) -> Result<ExitCode> {
    let versions = version_steps();
    let steps = check_steps(&args.path);

    let jobs = versions.iter().chain(&steps).map(Job::step).collect();
    let mut outcomes = run_parallel(jobs);
    let step_outcomes = outcomes.split_off(versions.len());
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
