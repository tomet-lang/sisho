//! `sisho convert`: one verb over the six conversion commands of `tomet`.

use std::process::ExitCode;

use sisho_run::Step;
use sisho_tools::{Conversion, Direction, Source, Target, Tool};

use super::tool::run_inherited;
use crate::cli::ConvertArgs;

/// What `convert` was asked for, read off the command line: which way. `Err`
/// is a command line that cannot mean anything, with the kind of mistake it
/// is.
///
/// This is `sisho`'s part: naming the formats, and refusing what only means
/// something in one direction before `tomet` is started. The converting is
/// `tomet`'s.
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
            Direction::To(target)
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

/// The step `convert` runs: the `tomet` command that converts this way, with
/// the rest of the command line.
pub fn convert_step(args: &ConvertArgs, direction: Direction) -> Step {
    let mut conversion = Conversion::new(&args.path, direction);
    conversion.out = args.out.as_deref();
    conversion.in_place = args.in_place;
    conversion.remove_original = args.remove_original;
    conversion.dry_run = args.dry_run;
    conversion.advanced = args.advanced;
    conversion.lang = args.lang.as_deref();
    conversion.body = args.body;
    conversion.step()
}

/// Run it with the terminal handed to `tomet`, so that a pipe into `-` and
/// out of standard output are `tomet`'s own, and what it prints and the code
/// it exits with are what `sisho` prints and exits with.
pub fn convert(args: &ConvertArgs, direction: &Direction) -> ExitCode {
    run_inherited(&convert_step(args, *direction), Tool::Tomet.name())
}
