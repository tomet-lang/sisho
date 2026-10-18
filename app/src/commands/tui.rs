//! `sisho tui`.

use std::process::ExitCode;

use super::finish;
use crate::cli::TuiArgs;

/// The interactive workbench. It takes over the terminal until the person
/// quits, and gives it back however it ends.
pub fn tui(args: &TuiArgs) -> ExitCode {
    finish(sisho_tui::run_tui(args.path.clone(), args.config.clone()))
}
