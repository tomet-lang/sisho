//! How a run is shown: human text and JSON.

use std::fmt::Write as _;
use std::path::Path;

use serde_json::{Value, json};
use sisho_run::{Outcome, Status};
use sisho_tools::Tool;

/// A tool and the version it reported, if it reported one.
pub struct ToolVersion {
    pub tool: Tool,
    pub version: Option<String>,
}

impl ToolVersion {
    /// Read a version out of the outcome of `<tool> --version`: the first line
    /// of its output, if the command passed.
    pub fn from_outcome(tool: Tool, outcome: &Outcome) -> Self {
        let version = outcome
            .is_pass()
            .then(|| outcome.stdout.lines().next().map(|l| l.trim().to_owned()))
            .flatten()
            .filter(|line| !line.is_empty());
        Self { tool, version }
    }
}

pub fn header(verb: &str, path: &Path, versions: &[ToolVersion]) -> String {
    let mut text = format!("sisho {verb} {}\n", path.display());
    let tools: Vec<String> = versions
        .iter()
        .map(|v| match &v.version {
            Some(version) => version.clone(),
            None => format!("{}: version unknown", v.tool.name()),
        })
        .collect();
    let _ = writeln!(text, "{}\n", tools.join(", "));
    text
}

/// One step: a status line, then its output if it failed (or `verbose`).
pub fn outcome(outcome: &Outcome, verbose: bool) -> String {
    let mark = if outcome.is_pass() { "✓" } else { "✗" };
    let mut text = format!(
        "{mark} {} ({:.2}s){}\n",
        outcome.name,
        outcome.duration.as_secs_f64(),
        detail(&outcome.status)
    );
    if verbose || !outcome.is_pass() {
        for stream in [&outcome.stdout, &outcome.stderr] {
            for line in stream.lines() {
                let _ = writeln!(text, "    {line}");
            }
        }
    }
    text
}

fn detail(status: &Status) -> String {
    match status {
        Status::Passed => String::new(),
        Status::Failed { code } => format!(": exit {code}"),
        Status::Signalled { signal } => format!(": killed by signal {signal}"),
        Status::NotFound => ": not found on PATH".to_owned(),
        Status::CouldNotStart { reason } => format!(": could not start: {reason}"),
    }
}

pub fn summary(outcomes: &[Outcome]) -> String {
    let failed = outcomes.iter().filter(|o| !o.is_pass()).count();
    if failed == 0 {
        format!("\nall {} steps passed\n", outcomes.len())
    } else {
        format!("\n{failed} of {} steps did not pass\n", outcomes.len())
    }
}

pub fn json(path: &Path, versions: &[ToolVersion], outcomes: &[Outcome]) -> Value {
    json!({
        "ok": outcomes.iter().all(Outcome::is_pass),
        "path": path.display().to_string(),
        "tools": versions.iter().map(|v| json!({
            "name": v.tool.name(),
            "version": v.version,
            "needs": v.tool.min_version().to_string(),
        })).collect::<Vec<_>>(),
        "steps": outcomes.iter().map(step_json).collect::<Vec<_>>(),
    })
}

fn step_json(outcome: &Outcome) -> Value {
    let (status, exit_code, signal, reason) = match &outcome.status {
        Status::Passed => ("passed", Some(0), None, None),
        Status::Failed { code } => ("failed", Some(*code), None, None),
        Status::Signalled { signal } => ("signalled", None, Some(*signal), None),
        Status::NotFound => ("not-found", None, None, None),
        Status::CouldNotStart { reason } => ("could-not-start", None, None, Some(reason.as_str())),
    };
    json!({
        "name": outcome.name,
        "command": outcome.command_line,
        "status": status,
        "exit_code": exit_code,
        "signal": signal,
        "reason": reason,
        "duration_ms": u64::try_from(outcome.duration.as_millis()).unwrap_or(u64::MAX),
        "stdout": outcome.stdout,
        "stderr": outcome.stderr,
    })
}
