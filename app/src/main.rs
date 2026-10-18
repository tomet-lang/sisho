#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod cli;
mod commands;
mod output;
mod report;

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
            commands::tool::tool_names()
        ))
        .get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };

    let result = match &cli.command {
        Command::Check(args) => commands::check::check(args),
        Command::Fix(args) => commands::fix::fix(args),
        Command::CheckLinks(args) => Ok(commands::check_links::check_links(args)),
        Command::Serve(args) => Ok(commands::serve::serve(args)),
        Command::Tui(args) => Ok(commands::tui::tui(args)),
        Command::Convert(args) => match commands::convert::convert_direction(args) {
            Ok(direction) => Ok(commands::convert::convert(args, &direction)),
            Err((kind, message)) => Cli::command().error(kind, message).exit(),
        },
        Command::Tool(words) => commands::tool::tool(words),
    };
    match result {
        Ok(code) => code,
        Err(error) if output::is_broken_pipe(&error) => ExitCode::from(output::BROKEN_PIPE),
        Err(error) => {
            eprintln!("sisho: {error:#}");
            ExitCode::from(OWN_ERROR)
        }
    }
}
