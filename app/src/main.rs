mod cli;
mod report;
mod verbs;

use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches};
use cli::{Cli, Command};

/// Exit code for an error of `sisho` itself, as opposed to a step that did
/// not pass (1).
const OWN_ERROR: u8 = 2;

/// Exit code for a command line `sisho` could not make sense of. clap uses
/// the same for its own usage errors.
const USAGE_ERROR: u8 = 2;

fn main() -> ExitCode {
    let matches = Cli::command()
        .after_help(format!(
            "Tools: {}\n`sisho <tool> ...` runs the tool with the rest of the command line.",
            verbs::tool_names()
        ))
        .get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };

    let result = match &cli.command {
        Command::Check(args) => verbs::check(args),
        Command::Fix(args) => verbs::fix(args),
        Command::Tool(words) => verbs::tool(words),
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("sisho: {error:#}");
            ExitCode::from(OWN_ERROR)
        }
    }
}
