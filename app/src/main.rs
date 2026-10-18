mod cli;

use std::process::ExitCode;

use clap::Parser;
use cli::Cli;

fn main() -> ExitCode {
    let _cli = Cli::parse();
    ExitCode::SUCCESS
}
