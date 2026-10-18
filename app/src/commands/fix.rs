//! `sisho fix`.

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use sisho_run::{Job, Report, run_parallel};
use sisho_tools::fix_steps;

use super::{exit_code, tool_versions, version_steps, write_json};
use crate::cli::RunArgs;
use crate::report;

/// One step after another, each reported as it ends: these write, so each has
/// to be done before the next looks at what it wrote. Regenerate, export,
/// then format.
pub fn fix(args: &RunArgs) -> Result<ExitCode> {
    let versions = version_steps();
    let outcomes = run_parallel(versions.iter().map(Job::step).collect());
    let versions = tool_versions(&outcomes);

    let mut out = io::stdout().lock();
    if !args.json {
        write!(out, "{}", report::header("fix", &args.path, &versions))?;
    }

    let mut step_outcomes = Vec::new();
    for step in fix_steps(&args.path) {
        let outcome = step.run();
        if !args.json {
            write!(out, "{}", report::outcome(&outcome, args.verbose))?;
            out.flush()?;
        }
        step_outcomes.push(outcome);
    }

    if args.json {
        write_json(&mut out, args, &versions, &step_outcomes)?;
    } else {
        write!(out, "{}", report::summary(&step_outcomes))?;
    }
    Ok(exit_code(&Report::new(step_outcomes)))
}
