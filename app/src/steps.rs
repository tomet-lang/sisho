//! The steps `sisho` does itself, as outcomes.
//!
//! A tool step runs a program and keeps what it printed. These run the vault
//! commands in-process, through the same functions the commands use, writing
//! to buffers instead of the terminal, and keep what they wrote in the same
//! [`Outcome`]. So the report of `check` and `fix` treats them alike.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use sisho_run::{Outcome, Status};

use crate::cli::{ExportArgs, RefactorArgs};
use crate::vault::{self, NeedsRefactor, Sink};

/// The exit status an in-process step reports when it did not pass: the one
/// the command itself exits with.
const FAILED: i32 = 1;

/// `sisho export --check`: every declared output is what its source renders to.
pub fn export_check(path: &Path) -> Outcome {
    in_process(
        "sisho export --check",
        &["export", "--check"],
        path,
        |out, err| vault::run_export(&export_args(path, true), out, err),
        |_| None,
    )
}

/// `sisho export`: write every declared output.
pub fn export_write(path: &Path) -> Outcome {
    in_process(
        "sisho export",
        &["export"],
        path,
        |out, err| vault::run_export(&export_args(path, false), out, err),
        |_| None,
    )
}

/// `sisho refactor --check`: no document is in a spelling the language has
/// moved off.
///
/// There is no matching step in `fix`: `refactor -i` re-prints the documents
/// from the syntax tree, which carries no comments, so it is something to ask
/// for by name after reading the dry run. When this step fails, it says so.
pub fn refactor_check(path: &Path) -> Outcome {
    in_process(
        "sisho refactor --check",
        &["refactor", "--check"],
        path,
        |out, err| vault::run_refactor(&refactor_args(path), out, err),
        |error| {
            error.downcast_ref::<NeedsRefactor>().map(|_| {
                "hint: `sisho refactor` shows what would change; `sisho refactor -i` applies it, \
                 and drops comments"
            })
        },
    )
}

fn export_args(path: &Path, check: bool) -> ExportArgs {
    ExportArgs {
        path: path.to_path_buf(),
        r#type: None,
        out: None,
        advanced: false,
        check,
    }
}

fn refactor_args(path: &Path) -> RefactorArgs {
    RefactorArgs {
        path: path.to_path_buf(),
        in_place: false,
        url_macros: false,
        meta_kind: false,
        check: true,
        force: false,
    }
}

/// Run `work` with buffers for standard output and error, and keep what it
/// wrote. An `Err` is a failed step, with the reason on standard error as the
/// command would print it, and `hint`'s advice after it if it has any.
fn in_process(
    name: &str,
    command: &[&str],
    path: &Path,
    work: impl FnOnce(Sink<'_>, Sink<'_>) -> Result<()>,
    hint: impl FnOnce(&anyhow::Error) -> Option<&'static str>,
) -> Outcome {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let started = Instant::now();
    let result = work(&mut out, &mut err);
    let duration = started.elapsed();

    let status = match result {
        Ok(()) => Status::Passed,
        Err(error) => {
            err.extend_from_slice(format!("sisho: {error:#}\n").as_bytes());
            if let Some(hint) = hint(&error) {
                err.extend_from_slice(format!("{hint}\n").as_bytes());
            }
            Status::Failed { code: FAILED }
        }
    };
    Outcome {
        name: name.to_owned(),
        command_line: command_line(command, path),
        status,
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
        duration,
    }
}

fn command_line(command: &[&str], path: &Path) -> String {
    let mut line = String::from("sisho");
    for word in command {
        line.push(' ');
        line.push_str(word);
    }
    line.push(' ');
    line.push_str(&PathBuf::from(path).display().to_string());
    line
}
