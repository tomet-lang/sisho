//! The vault commands: what they print, and the exit code of a failure.
//!
//! The work is `sisho-vault`'s and returns data. These functions show it.
//! Each `run_*` writes to the `out` and `err` it is given, so the same code
//! serves the command (standard output and error) and a step of `check` or
//! `fix` (buffers, kept in the step's outcome). A command that could not do
//! what was asked prints `sisho: <reason>` and exits 1, as `tomet` does, so a
//! script that moves from `tomet export` to `sisho export` keeps working.

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use sisho_vault::convert::{self, Converted, Direction, Plan, Source, Target};
use sisho_vault::export::{self, Action};
use sisho_vault::links;
use sisho_vault::render::Html;

use crate::cli::{CheckLinksArgs, ConvertArgs, ExportArgs, ServeArgs, TuiArgs};
use crate::output::{BROKEN_PIPE, is_broken_pipe};

/// The exit code of a vault command that failed.
const FAILED: u8 = 1;

/// Where a command writes: its standard output and its standard error.
pub type Sink<'a> = &'a mut dyn Write;

pub fn check_links(args: &CheckLinksArgs) -> ExitCode {
    match with_standard_streams(|out, err| run_check_links(args, out, err)) {
        Ok(true) => ExitCode::from(FAILED),
        Ok(false) => ExitCode::SUCCESS,
        Err(error) => failed(&error),
    }
}

pub fn export(args: &ExportArgs) -> ExitCode {
    finish(with_standard_streams(|out, err| run_export(args, out, err)))
}

fn with_standard_streams<T>(run: impl FnOnce(Sink<'_>, Sink<'_>) -> Result<T>) -> Result<T> {
    let (mut out, mut err) = (io::stdout().lock(), io::stderr().lock());
    run(&mut out, &mut err)
}

fn finish(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => failed(&error),
    }
}

/// A command that did not do what was asked. If the reason is only that
/// nobody is reading the output any more, there is nothing to say.
fn failed(error: &anyhow::Error) -> ExitCode {
    if is_broken_pipe(error) {
        return ExitCode::from(BROKEN_PIPE);
    }
    let _ = writeln!(io::stderr(), "sisho: {error:#}");
    ExitCode::from(FAILED)
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

pub fn run_export(args: &ExportArgs, out: Sink<'_>, err: Sink<'_>) -> Result<()> {
    let directory = args.path.is_dir();
    let options = export::Options {
        only_type: args.r#type.as_deref(),
        out: args.out.as_deref(),
        advanced: args.advanced,
        check: args.check,
    };

    // Shown as each document finishes, so a long run is not silent. A write
    // that fails inside the callback cannot be returned from it, so the first
    // failure is kept and raised once the run is over.
    let mut write_error: Option<io::Error> = None;
    let run = export::export_with(&args.path, options, |document| {
        if write_error.is_some() {
            return;
        }
        let shown = show_document(document, directory, out, err);
        if let Err(error) = shown {
            write_error = Some(error);
        }
    })?;
    if let Some(error) = write_error {
        return Err(error.into());
    }

    if !directory {
        // A single file: its failure is the run's failure, said plainly.
        if let Some(error) = run.documents.first().and_then(|d| d.error.clone()) {
            return Err(anyhow::anyhow!(error));
        }
        return report_stale(&run, args.check, out);
    }

    if run.documents.is_empty() {
        writeln!(
            out,
            "No .tmt or .tmt files found in {}",
            args.path.display()
        )?;
        return Ok(());
    }

    // One document that fails does not stop the others, but it is not a
    // success: under `--check` a document that could not be read has not
    // been checked, and passing there says it is up to date.
    let failed = run.failed();
    let total = run.documents.len();
    if args.check {
        if failed == 0 {
            return report_stale(&run, true, out);
        }
        for path in run.stale() {
            writeln!(out, "Stale export: {}", path.display())?;
        }
        let differ = match run.stale().len() {
            0 => String::new(),
            n => format!(", and {n} export(s) differ from their source"),
        };
        return Err(anyhow::anyhow!(
            "{failed} of {total} file(s) could not be checked{differ}"
        ));
    }
    writeln!(
        out,
        "Batch export finished: {} file(s) processed.",
        total - failed
    )?;
    if failed > 0 {
        return Err(anyhow::anyhow!(
            "{failed} of {total} file(s) could not be exported"
        ));
    }
    Ok(())
}

fn show_document(
    document: &export::Document,
    directory: bool,
    out: Sink<'_>,
    err: Sink<'_>,
) -> io::Result<()> {
    for action in &document.actions {
        match action {
            Action::Wrote { dest } => writeln!(
                out,
                "Exported {} -> {}",
                document.source.display(),
                dest.display()
            )?,
            Action::Printed { text } => write!(out, "{text}")?,
            Action::Stale { .. } | Action::Current { .. } => {}
        }
    }
    if directory && let Some(error) = &document.error {
        writeln!(
            err,
            "Error exporting {}: {error}",
            document.source.display()
        )?;
    }
    Ok(())
}

/// Fails when `--check` found something out of date, so this can gate a
/// commit the way `refactor --check` does.
fn report_stale(run: &export::Run, check: bool, out: Sink<'_>) -> Result<()> {
    let stale = run.stale();
    if !check || stale.is_empty() {
        if check {
            writeln!(out, "All exports are up to date.")?;
        }
        return Ok(());
    }
    for path in &stale {
        writeln!(out, "Stale export: {}", path.display())?;
    }
    Err(anyhow::anyhow!(
        "{} export(s) differ from their source; re-run without --check",
        stale.len()
    ))
}

/// What `convert` was asked for, read off the command line: which way, and
/// into what. `Err` is a command line that cannot mean anything, with the kind
/// of mistake it is.
pub fn convert_direction(
    args: &ConvertArgs,
) -> Result<Direction, (clap::error::ErrorKind, String)> {
    use clap::error::ErrorKind::{ArgumentConflict, InvalidValue};

    let html_option = args.advanced || args.lang.is_some() || args.body;
    let direction = match (&args.to, &args.from) {
        (Some(name), None) => {
            let target = Target::parse(name).ok_or_else(|| {
                (
                    InvalidValue,
                    format!(
                        "unknown format '{name}' for --to; one of: {}",
                        Target::NAMES
                    ),
                )
            })?;
            if html_option && target != Target::Html {
                return Err((
                    ArgumentConflict,
                    "--advanced, --lang and --body apply to --to html only".to_owned(),
                ));
            }
            Direction::To(
                target,
                Html {
                    advanced: args.advanced,
                    lang: args.lang.clone(),
                    body: args.body,
                },
            )
        }
        (None, Some(name)) => {
            if html_option {
                return Err((
                    ArgumentConflict,
                    "--advanced, --lang and --body apply to --to html only".to_owned(),
                ));
            }
            Direction::From(Source::parse(name).ok_or_else(|| {
                (
                    InvalidValue,
                    format!(
                        "unknown format '{name}' for --from; one of: {}",
                        Source::NAMES
                    ),
                )
            })?)
        }
        // clap requires exactly one of the two.
        _ => unreachable!("exactly one of --to and --from"),
    };

    if args.remove_original {
        if matches!(direction, Direction::To(..)) {
            return Err((
                ArgumentConflict,
                "--remove-original applies to --from: converting to Tomet is a migration, \
                 and a Tomet source is not something to throw away"
                    .to_owned(),
            ));
        }
        if args.out.is_none() && !args.in_place {
            return Err((
                ArgumentConflict,
                "--remove-original needs --out or --in-place: there is nothing to replace \
                 the original with otherwise"
                    .to_owned(),
            ));
        }
    }
    Ok(direction)
}

pub fn convert(args: &ConvertArgs, direction: &Direction) -> ExitCode {
    finish(with_standard_streams(|out, err| {
        run_convert(args, direction, out, err)
    }))
}

pub fn run_convert(
    args: &ConvertArgs,
    direction: &Direction,
    out: Sink<'_>,
    err: Sink<'_>,
) -> Result<()> {
    let plan = Plan {
        out: args.out.as_deref(),
        in_place: args.in_place,
        dry_run: args.dry_run,
        remove_original: args.remove_original,
    };
    let path = &args.path;

    if convert::is_stdin(path) || path.is_file() {
        match convert::convert_file(path, direction, &plan)? {
            // Exactly the text, as it would be written to a file.
            Converted::Text(text) => write!(out, "{text}")?,
            Converted::Written(dest) => {
                writeln!(out, "Converted {} -> {}", path.display(), dest.display())?;
            }
            Converted::Checked => writeln!(out, "OK (dry-run): {}", path.display())?,
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Err(anyhow::anyhow!("path '{}' does not exist", path.display()));
    }

    let tree = convert::convert_tree(path, direction, &plan)?;
    if tree.items.is_empty() {
        let found = match direction {
            Direction::From(_) => ".md",
            Direction::To(..) => ".tmt or .tmt",
        };
        writeln!(out, "No {found} files found in {}", path.display())?;
        return Ok(());
    }

    let (total, succeeded, failed) = (tree.items.len(), tree.succeeded(), tree.failed());
    let finished = if args.dry_run {
        "Dry-run finished"
    } else {
        "Conversion finished"
    };
    writeln!(
        out,
        "{finished}: {total} total, {succeeded} succeeded, {failed} failed"
    )?;
    if failed > 0 {
        writeln!(err, "\nErrors encountered ({failed}):")?;
        for item in &tree.items {
            if let Err(error) = &item.result {
                writeln!(err, "  {}: {error}", item.source.display())?;
            }
        }
        return Err(anyhow::anyhow!("{failed} file(s) failed conversion"));
    }
    Ok(())
}

pub fn serve(args: &ServeArgs) -> ExitCode {
    finish(with_standard_streams(|out, err| run_serve(args, out, err)))
}

pub fn run_serve(args: &ServeArgs, out: Sink<'_>, _err: Sink<'_>) -> Result<()> {
    let options = sisho_serve::Options {
        file: args.file.clone(),
        advanced: args.advanced,
        lang: args.lang.clone(),
    };
    // Said once the port is bound, which is when it can be used. Flushed, so
    // that whoever started this through a pipe sees it now and not at exit.
    let mut announced: io::Result<()> = Ok(());
    let served = sisho_serve::serve(args.port, options, |addr| {
        announced =
            writeln!(out, "Serving at http://{addr} (Ctrl+C to stop)").and_then(|()| out.flush());
    });
    served.map_err(|error| anyhow::anyhow!("cannot serve on port {}: {error}", args.port))?;
    announced?;
    Ok(())
}

pub fn tui(args: &TuiArgs) -> ExitCode {
    finish(sisho_tui::run_tui(args.path.clone(), args.config.clone()))
}
