//! `sisho fix`.

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use sisho_run::{Job, Report, run_parallel};
use sisho_tools::fix_steps;

use super::{exit_code, tool_versions, version_problems, version_steps, write_json};
use crate::cli::RunArgs;
use crate::report;

/// One step after another, each reported as it ends: these write, so each has
/// to be done before the next looks at what it wrote. Regenerate, export,
/// then format.
///
/// A tool older than `sisho` works with stops it before anything is written.
/// `check` can go on and say what it sees, because it only reads; this would
/// have an older formatter or exporter rewrite files in a way the version
/// `sisho` expects would not.
pub fn fix(args: &RunArgs) -> Result<ExitCode> {
    let versions = version_steps();
    let outcomes = run_parallel(versions.iter().map(Job::step).collect());
    let problems = version_problems(&outcomes);
    let versions = tool_versions(&outcomes);

    let mut out = io::stdout().lock();
    if !args.json {
        write!(out, "{}", report::header("fix", &args.path, &versions))?;
    }

    let mut step_outcomes = Vec::new();
    if problems.is_empty() {
        for step in fix_steps(&args.path) {
            let outcome = step.run();
            if !args.json {
                write!(out, "{}", report::outcome(&outcome, args.verbose))?;
                out.flush()?;
            }
            step_outcomes.push(outcome);
        }
    } else {
        for problem in problems {
            if !args.json {
                write!(out, "{}", report::outcome(&problem, args.verbose))?;
            }
            step_outcomes.push(problem);
        }
        if !args.json {
            writeln!(
                out,
                "\nnothing was changed: `fix` writes files, and not with a tool older than sisho works with"
            )?;
        }
    }

    if args.json {
        write_json(&mut out, args, &versions, &step_outcomes)?;
    } else {
        write!(out, "{}", report::summary(&step_outcomes))?;
    }
    Ok(exit_code(&Report::new(step_outcomes)))
}
