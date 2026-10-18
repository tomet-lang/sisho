//! `sisho check-links`.

use std::process::ExitCode;

use anyhow::Result;
use sisho_vault::links;

use super::{FAILED, Sink, failed, with_standard_streams};
use crate::cli::CheckLinksArgs;

pub fn check_links(args: &CheckLinksArgs) -> ExitCode {
    match with_standard_streams(|out, err| run_check_links(args, out, err)) {
        Ok(true) => ExitCode::from(FAILED),
        Ok(false) => ExitCode::SUCCESS,
        Err(error) => failed(&error),
    }
}

/// Runs the check and writes its report. `Ok(true)` means there is something
/// to deal with -- a broken link, or a document that could not be read --
/// which is not the same as `Err`, where the check itself could not run.
pub fn run_check_links(args: &CheckLinksArgs, out: Sink<'_>, err: Sink<'_>) -> Result<bool> {
    let report = links::check(&args.path, &links::default_cache_path())?;

    if args.json {
        writeln!(out, "{}", links_json(&report))?;
    } else {
        writeln!(
            out,
            "{} file(s) scanned, {} link(s) checked, {} broken",
            report.files_scanned,
            report.links_checked,
            report.broken.len(),
        )?;
        for link in &report.broken {
            let at = link
                .at
                .map_or("?:?".to_string(), |at| format!("{}:{}", at.line, at.column));
            writeln!(
                out,
                "  {}:{at}: broken link -> {}",
                link.source.display(),
                link.target
            )?;
        }
        for unreadable in &report.unreadable {
            writeln!(
                err,
                "  {}: {}",
                unreadable.source.display(),
                unreadable.error
            )?;
        }
    }
    Ok(report.has_problems())
}

fn links_json(report: &links::Report) -> String {
    let broken: Vec<_> = report
        .broken
        .iter()
        .map(|link| {
            serde_json::json!({
                "source": link.source.display().to_string(),
                "target": link.target,
                "line": link.at.map(|at| at.line),
                "column": link.at.map(|at| at.column),
            })
        })
        .collect();
    let errors: Vec<_> = report
        .unreadable
        .iter()
        .map(|unreadable| {
            serde_json::json!({
                "source": unreadable.source.display().to_string(),
                "error": unreadable.error,
            })
        })
        .collect();
    serde_json::json!({
        "files_scanned": report.files_scanned,
        "links_checked": report.links_checked,
        "broken": broken,
        "errors": errors,
    })
    .to_string()
}
