//! `sisho convert`.

use std::process::ExitCode;

use anyhow::Result;
use sisho_vault::convert::{self, Converted, Direction, Plan, Source, Target};
use sisho_vault::render::Html;

use super::{Sink, finish, with_standard_streams};
use crate::cli::ConvertArgs;

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
