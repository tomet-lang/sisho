//! The verbs that run across every tool: `check` and `fix`.

use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::{Result, bail};
use sisho_run::{Job, Outcome, Report, Status, Step, run_parallel};
use sisho_tools::{Tool, check_steps, fix_format_steps, fix_generate_steps};

use crate::USAGE_ERROR;
use crate::cli::RunArgs;
use crate::report::{self, ToolVersion};
use crate::steps;

/// `sisho check`: every step at the same time, reported in plan order. The
/// tool steps come first, then `sisho`'s own checks.
pub fn check(args: &RunArgs) -> Result<ExitCode> {
    let versions = version_steps();
    let steps = check_steps(&args.path);

    // The tool steps, then the checks `sisho` does itself; all at the same
    // time, since none of them writes.
    let path = args.path.as_path();
    let mut jobs: Vec<Job<'_>> = versions.iter().chain(&steps).map(Job::step).collect();
    jobs.push(Job::new(|| steps::export_check(path)));
    jobs.push(Job::new(|| steps::refactor_check(path)));
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

/// `sisho fix`: one step after another, each reported as it ends. Regenerate, export,
/// then format; `refactor -i` is not here, because it drops comments.
pub fn fix(args: &RunArgs) -> Result<ExitCode> {
    let versions = version_steps();
    let outcomes = run_parallel(versions.iter().map(Job::step).collect());
    let versions = tool_versions(&outcomes);

    let mut out = io::stdout().lock();
    if !args.json {
        write!(out, "{}", report::header("fix", &args.path, &versions))?;
    }

    // One after another, because these write. Regenerate first, then render
    // the exports from what was regenerated, then format what came out.
    let path = args.path.as_path();
    let mut step_outcomes = Vec::new();
    let mut record = |outcome: Outcome, out: &mut io::StdoutLock<'_>| -> Result<()> {
        if !args.json {
            write!(out, "{}", report::outcome(&outcome, args.verbose))?;
            out.flush()?;
        }
        step_outcomes.push(outcome);
        Ok(())
    };
    for step in fix_generate_steps(path) {
        record(step.run(), &mut out)?;
    }
    record(steps::export_write(path), &mut out)?;
    for step in fix_format_steps(path) {
        record(step.run(), &mut out)?;
    }

    if args.json {
        write_json(&mut out, args, &versions, &step_outcomes)?;
    } else {
        write!(out, "{}", report::summary(&step_outcomes))?;
    }
    Ok(exit_code(&Report::new(step_outcomes)))
}

fn version_steps() -> Vec<Step> {
    Tool::ALL.iter().map(|tool| tool.version_step()).collect()
}

fn tool_versions(outcomes: &[Outcome]) -> Vec<ToolVersion> {
    Tool::ALL
        .iter()
        .zip(outcomes)
        .map(|(tool, outcome)| ToolVersion::from_outcome(*tool, outcome))
        .collect()
}

fn write_json(
    out: &mut impl Write,
    args: &RunArgs,
    versions: &[ToolVersion],
    outcomes: &[Outcome],
) -> Result<()> {
    serde_json::to_writer_pretty(&mut *out, &report::json(&args.path, versions, outcomes))?;
    writeln!(out)?;
    Ok(())
}

fn exit_code(report: &Report) -> ExitCode {
    ExitCode::from(report.exit_code())
}

/// `sisho <tool> ...`: run the tool with the rest of the command line.
///
/// `words` is what the command line held after `sisho`: the tool's name, then
/// its arguments, untouched.
pub fn tool(words: &[OsString]) -> Result<ExitCode> {
    let Some((name, args)) = words.split_first() else {
        bail!("no tool given");
    };
    let Some(tool) = name.to_str().and_then(Tool::from_name) else {
        eprintln!(
            "sisho: `{}` is neither a command nor a tool\n  tools: {}\n  try `sisho --help`",
            name.to_string_lossy(),
            tool_names()
        );
        return Ok(ExitCode::from(USAGE_ERROR));
    };

    let status = Step::new(tool.name(), tool.name())
        .args(args)
        .run_inherited();
    if let Status::NotFound = status {
        eprintln!("sisho: {} is not on PATH", tool.name());
    } else if let Status::CouldNotStart { reason } = &status {
        eprintln!("sisho: could not start {}: {reason}", tool.name());
    }
    Ok(ExitCode::from(passthrough_code(&status)))
}

/// The names of the tools, for messages.
pub fn tool_names() -> String {
    Tool::ALL
        .iter()
        .map(|tool| tool.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The exit code `sisho` takes on when a tool it passed through ends as `status`.
///
/// The tool's own code is kept as it is. The rest follows `env`: 127 for a
/// program that is not found, 126 for one that cannot be started, 128 plus the
/// signal for one that was killed.
fn passthrough_code(status: &Status) -> u8 {
    match status {
        Status::Passed => 0,
        // A code outside 0..=255 cannot be an exit status; keep the low byte
        // as the operating system does.
        Status::Failed { code } => (code & 0xff) as u8,
        Status::Signalled { signal } => (128 + (signal & 0x7f)) as u8,
        Status::NotFound => 127,
        Status::CouldNotStart { .. } => 126,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tools_own_exit_code_is_kept() {
        assert_eq!(passthrough_code(&Status::Passed), 0);
        assert_eq!(passthrough_code(&Status::Failed { code: 3 }), 3);
        assert_eq!(passthrough_code(&Status::Failed { code: 255 }), 255);
    }

    #[test]
    fn what_is_not_an_exit_code_follows_env() {
        assert_eq!(passthrough_code(&Status::NotFound), 127);
        assert_eq!(
            passthrough_code(&Status::CouldNotStart {
                reason: String::new()
            }),
            126
        );
        assert_eq!(passthrough_code(&Status::Signalled { signal: 9 }), 137);
    }
}
