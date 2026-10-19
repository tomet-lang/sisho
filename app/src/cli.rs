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
    /// Number of parallel jobs to run.
    ///
    /// Only matters for the commands that run tools in parallel (`check`, `fix`).
    /// If a jobserver was inherited from the environment, it wins and `-j` is not used.
    #[arg(short = 'j', long = "jobs", global = true, value_name = "N")]
    pub jobs: Option<std::num::NonZeroUsize>,

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
    /// Check every `@file`, `@dir`, `@embed` and `@link` in a document, or in
    /// every document under a directory, for a local target that does not
    /// exist.
    ///
    /// What it learns about a document is kept in an SQLite cache in the
    /// user's cache directory (`~/.cache/tomet/` on Unix), never inside the
    /// vault, so checking a large vault again does not parse what has not
    /// changed. Exits 1 if a link is broken or a document could not be read.
    CheckLinks(CheckLinksArgs),
    /// Change a document, or a directory of them, from one format to another.
    ///
    /// `--to` reads Tomet and writes commonmark, html, typst or pandoc (its JSON
    /// AST, which `pandoc -f json` turns into every format Pandoc writes).
    /// `--from` reads md or pandoc (`pandoc -t json x.docx | sisho convert -
    /// --from pandoc`) and writes Tomet. The input is a file, `-` for standard
    /// input, or a directory; the output goes to standard output, to `--out`,
    /// or next to the source with `--in-place`. A directory is converted into
    /// files, so it needs `--out`, `--in-place` or `--dry-run`.
    ///
    /// Nothing is read from a document's own `@config`: that is `sisho
    /// export`. This is the conversion you ask for by name.
    Convert(ConvertArgs),
    /// Serve a document as HTML over HTTP on 127.0.0.1, rendering it again on
    /// every request: edit the file and reload the page.
    ///
    /// It is the rendering of `sisho convert --to html`. A document that
    /// cannot be rendered is shown as an error page with status 500, so the
    /// browser tells you what is wrong. Stop it with Ctrl+C.
    Serve(ServeArgs),
    /// Run a tool or extension with the rest of the command line, unchanged:
    /// `sisho tomet format -i .` is `tomet format -i .`. `sisho shelf build` is
    /// `tomet-shelf build`. An extension `sisho-<word>` found on `PATH` is run
    /// as `sisho <word>`.
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

#[derive(Args)]
pub struct CheckLinksArgs {
    /// File or directory to check (defaults to ".").
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Print the report as JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct ConvertArgs {
    /// File or directory to convert, or `-` for standard input.
    pub path: PathBuf,
    /// Convert from Tomet to this format: commonmark (md), html, typst, pandoc.
    #[arg(
        long,
        value_name = "FORMAT",
        conflicts_with = "from",
        required_unless_present = "from"
    )]
    pub to: Option<String>,
    /// Convert to Tomet from this format: md (commonmark), pandoc.
    #[arg(long, value_name = "FORMAT")]
    pub from: Option<String>,
    /// Write here instead of standard output: a file, or a directory (for a
    /// file, it gets one named after the source; for a directory, it is
    /// written into, keeping the sub-directories).
    #[arg(short, long)]
    pub out: Option<PathBuf>,
    /// Write next to each source, with the new extension.
    #[arg(short = 'i', long, alias = "write", conflicts_with = "out")]
    pub in_place: bool,
    /// Delete each source once its result is written. Only with `--from`:
    /// converting to Tomet is a migration, and a Tomet source is not
    /// something to throw away. Needs `--out` or `--in-place`.
    #[arg(long)]
    pub remove_original: bool,
    /// Convert and check, and write nothing.
    #[arg(long)]
    pub dry_run: bool,
    /// With `--to html`: number headings by nesting level and give them
    /// slugs.
    #[arg(long)]
    pub advanced: bool,
    /// With `--to html`: `<html lang="...">` for the page shell.
    #[arg(long)]
    pub lang: Option<String>,
    /// With `--to html`: just the rendered body, with no `<html>` / `<head>`
    /// shell, for templating into a page of your own. `--lang` has no effect
    /// alongside this.
    #[arg(long)]
    pub body: bool,
}

#[derive(Args)]
pub struct ServeArgs {
    /// The document to serve.
    pub file: PathBuf,
    /// The port to listen on; 0 takes a free one.
    #[arg(short, long, default_value_t = 8787)]
    pub port: u16,
    /// Number headings by nesting level and give them slugs.
    #[arg(long)]
    pub advanced: bool,
    /// `<html lang="...">` for the page shell. Defaults to "ja".
    #[arg(long)]
    pub lang: Option<String>,
}
