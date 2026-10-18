use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sisho",
    version,
    about = "Orchestrate the Tomet ecosystem (tomet, twrit, tandr)",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Verify a project without writing anything: run every tool's checks
    /// over PATH and report which passed.
    ///
    /// The steps run at the same time and are reported in plan order. One
    /// failing step does not stop the others; the run exits 1 if any step
    /// failed, and a tool that is not installed counts as a failure. Exit
    /// code 2 means `sisho` itself could not do its job.
    Check(RunArgs),
    /// Repair what the tools can repair in a project.
    ///
    /// The steps write files, so they run one after another in a fixed
    /// order, each reported as soon as it ends. One failing step does not
    /// stop the others. Run `sisho check` afterwards to confirm.
    Fix(RunArgs),
    /// Run a tool with the rest of the command line, unchanged: `sisho tomet
    /// format -i .` is `tomet format -i .`.
    ///
    /// The tool gets the terminal and its exit code becomes sisho's. A tool
    /// that is not installed exits 127, one that cannot be started 126, one
    /// ended by a signal 128 plus the signal, as `env` does.
    #[command(external_subcommand)]
    Tool(Vec<OsString>),
}

#[derive(Args)]
pub struct RunArgs {
    /// File or directory to run over (defaults to ".").
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Print the result as JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
    /// Show the output of steps that passed too, not only of those that failed.
    #[arg(short, long, conflicts_with = "json")]
    pub verbose: bool,
}
