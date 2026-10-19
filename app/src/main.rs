#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod cli;
mod commands;
mod extension;
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
            "Tools: {}\n`sisho <tool> ...` runs the tool with the rest of the command line.\nExtensions: a program named `sisho-<word>` on PATH runs as `sisho <word>`, for example `sisho desk`.",
            commands::tool::tool_names()
        ))
        .get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };

    if cli.list {
        return extension::list(cli.all);
    }

    let Some(command) = &cli.command else {
        let _ = Cli::command().print_help();
        return ExitCode::from(USAGE_ERROR);
    };

    if let Err(error) = init_jobserver(cli.jobs.map(|n| n.get())) {
        eprintln!("sisho: {error:#}");
        return ExitCode::from(OWN_ERROR);
    }

    let result = match command {
        Command::Check(args) => commands::check::check(args),
        Command::Fix(args) => commands::fix::fix(args),
        Command::CheckLinks(args) => Ok(commands::check_links::check_links(args)),
        Command::Serve(args) => Ok(commands::serve::serve(args)),
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

fn init_jobserver(jobs: Option<usize>) -> std::io::Result<()> {
    // SAFETY: `from_env_ext` reads and takes ownership of file descriptors specified in
    // environment variables (such as `CARGO_MAKEFLAGS`). This function is called once,
    // early in `main` before any other file descriptors are opened or threads are spawned,
    // ensuring those descriptors are valid and not concurrently reused.
    let from_env = unsafe { jobserver::Client::from_env_ext(true) };
    let client = match from_env.client {
        Ok(client) => client,
        Err(err) => {
            if let Some((var_name, var_val)) = from_env.var
                && !matches!(
                    err.kind(),
                    jobserver::FromEnvErrorKind::NoJobserver
                        | jobserver::FromEnvErrorKind::NoEnvVar
                )
            {
                eprintln!(
                    "warning: failed to connect to jobserver from environment variable `{}=\"{}\"`: {}",
                    var_name,
                    var_val.to_string_lossy(),
                    err
                );
            }
            let limit = jobs.unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4)
            });
            let pipe_tokens = limit.saturating_sub(1);
            jobserver::Client::new(pipe_tokens)?
        }
    };
    sisho_run::init_gate(client);
    Ok(())
}
