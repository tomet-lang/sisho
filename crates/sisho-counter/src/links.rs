//! The link check as the counter shows it: what `tomet check-links --json`
//! reports about the links of a vault that point at nothing.
//!
//! `tomet` does the check, with the cache it keeps outside the vault; this
//! runs it and reads the one JSON object it writes (the contract is
//! `tomet`'s, in `spec/cli/check-links.tmt` there).

use std::path::{Path, PathBuf};

use serde_json::Value;
use sisho_run::{Outcome, Status};
use sisho_tools::check_links_step;

/// A line and a column, both counted from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// A link whose target does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLink {
    /// The document the link is written in.
    pub source: PathBuf,
    /// The target as written.
    pub target: String,
    /// Where the link is in `source`. `None` when the file can no longer be
    /// read.
    pub at: Option<Position>,
}

/// A document that could not be looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub source: PathBuf,
    pub error: String,
}

/// What a run found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub files_scanned: usize,
    pub links_checked: usize,
    pub broken: Vec<BrokenLink>,
    pub unreadable: Vec<Unreadable>,
}

/// Check the links under `dir` by running `tomet check-links --json`.
///
/// `Err` is why there is no report: `tomet` is not installed, or it could not
/// do the check (a path that is not there, a cache that cannot be opened). A
/// broken link is not an error; `tomet` exits 1 for it and the report is
/// there all the same.
pub fn check(dir: &Path) -> Result<Report, String> {
    report_of(check_links_step(dir).run())
}

/// The report in what `tomet check-links --json` left behind, or why there is
/// none.
fn report_of(outcome: Outcome) -> Result<Report, String> {
    let said = outcome.stderr.trim();
    match &outcome.status {
        // 0: nothing to deal with. 1: a broken link, or a document that could
        // not be read -- or the check could not run, in which case there is no
        // report on standard output.
        Status::Passed | Status::Failed { code: 1 } => match parse(&outcome.stdout) {
            Ok(report) => Ok(report),
            Err(_) if !said.is_empty() => Err(said.to_owned()),
            Err(reason) => Err(reason),
        },
        Status::NotFound => Err("tomet is not on PATH".to_owned()),
        Status::CouldNotStart { reason } => Err(format!("could not start tomet: {reason}")),
        Status::Failed { .. } | Status::Signalled { .. } if !said.is_empty() => {
            Err(said.to_owned())
        }
        Status::Failed { code } => Err(format!(
            "`{}` exited with code {code}",
            outcome.command_line
        )),
        Status::Signalled { signal } => Err(format!(
            "`{}` was ended by signal {signal}",
            outcome.command_line
        )),
    }
}

/// The report in the JSON object `tomet check-links --json` writes.
fn parse(json: &str) -> Result<Report, String> {
    let value: Value = serde_json::from_str(json.trim())
        .map_err(|error| format!("tomet check-links did not write a report: {error}"))?;
    let count = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .ok_or_else(|| missing(key))
    };
    let list = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| missing(key))
    };

    let files_scanned = count("files_scanned")?;
    let links_checked = count("links_checked")?;
    let mut broken = Vec::new();
    for entry in list("broken")? {
        let line = entry.get("line").and_then(Value::as_u64);
        let column = entry.get("column").and_then(Value::as_u64);
        broken.push(BrokenLink {
            source: PathBuf::from(text(entry, "source")?),
            target: text(entry, "target")?.to_owned(),
            at: match (line, column) {
                (Some(line), Some(column)) => Some(Position {
                    line: line as usize,
                    column: column as usize,
                }),
                _ => None,
            },
        });
    }
    let mut unreadable = Vec::new();
    for entry in list("errors")? {
        unreadable.push(Unreadable {
            source: PathBuf::from(text(entry, "source")?),
            error: text(entry, "error")?.to_owned(),
        });
    }
    Ok(Report {
        files_scanned,
        links_checked,
        broken,
        unreadable,
    })
}

fn text<'a>(entry: &'a Value, key: &str) -> Result<&'a str, String> {
    entry
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| missing(key))
}

fn missing(key: &str) -> String {
    format!("tomet check-links wrote a report without `{key}`")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn outcome(status: Status, stdout: &str, stderr: &str) -> Outcome {
        Outcome::in_process(
            "tomet check-links --json",
            status,
            stdout,
            stderr,
            Duration::ZERO,
        )
    }

    const REPORT: &str = r#"{"files_scanned":2,"links_checked":5,
        "broken":[{"source":"a.tmt","target":"missing.png","line":3,"column":4},
                  {"source":"gone.tmt","target":"x","line":null,"column":null}],
        "errors":[{"source":"b.tmt","error":"denied"}]}"#;

    #[test]
    fn the_report_holds_the_counts_the_positions_and_the_unreadable() {
        let report = parse(REPORT).unwrap();
        assert_eq!(report.files_scanned, 2);
        assert_eq!(report.links_checked, 5);
        assert_eq!(report.broken[0].source, PathBuf::from("a.tmt"));
        assert_eq!(report.broken[0].target, "missing.png");
        assert_eq!(report.broken[0].at, Some(Position { line: 3, column: 4 }));
        // A link whose file can no longer be read has no position.
        assert_eq!(report.broken[1].at, None);
        assert_eq!(report.unreadable[0].error, "denied");
    }

    #[test]
    fn a_report_with_a_broken_link_is_a_report_although_tomet_exits_1() {
        let report = report_of(outcome(
            Status::Failed { code: 1 },
            REPORT,
            "check failed\n",
        ));
        assert_eq!(report.unwrap().broken.len(), 2);
    }

    #[test]
    fn a_clean_run_is_a_report() {
        let clean = r#"{"files_scanned":1,"links_checked":0,"broken":[],"errors":[]}"#;
        let report = report_of(outcome(Status::Passed, clean, "")).unwrap();
        assert!(report.broken.is_empty() && report.unreadable.is_empty());
    }

    /// `tomet` exits 1 with nothing on standard output when the check could not
    /// run, and says why on standard error.
    #[test]
    fn a_check_that_could_not_run_says_why_in_tomets_words() {
        let error = report_of(outcome(
            Status::Failed { code: 1 },
            "",
            "path 'x' does not exist\n",
        ))
        .unwrap_err();
        assert_eq!(error, "path 'x' does not exist");
    }

    #[test]
    fn a_tomet_that_is_not_installed_is_said_so() {
        let error = report_of(outcome(Status::NotFound, "", "")).unwrap_err();
        assert_eq!(error, "tomet is not on PATH");
    }

    #[test]
    fn what_is_not_a_report_is_not_taken_for_one() {
        assert!(parse("not json").is_err());
        let error = parse(r#"{"files_scanned":1}"#).unwrap_err();
        assert!(error.contains("links_checked"), "{error}");
        let error = parse(r#"{"files_scanned":1,"links_checked":1,"broken":[{}],"errors":[]}"#)
            .unwrap_err();
        assert!(error.contains("source"), "{error}");
    }

    #[test]
    fn another_failure_is_reported_with_the_command_that_ran() {
        let error = report_of(outcome(Status::Failed { code: 2 }, "", "")).unwrap_err();
        assert!(error.contains("tomet check-links --json"), "{error}");
        assert!(error.contains("code 2"), "{error}");
    }
}
