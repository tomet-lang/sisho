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
    /// Make a new document from a blueprint the vault declares.
    ///
    /// The vault is the one whose config is found from the working directory
    /// upward. Without `--blueprint`, the blueprint is named after the
    /// directory the document goes in. `--list` shows the declared blueprints
    /// instead of making a document.
    New(NewArgs),
    /// Check every `@file`, `@dir`, `@embed` and `@link` in a document, or in
    /// every document under a directory, for a local target that does not
    /// exist.
    ///
    /// What it learns about a document is kept in an SQLite cache in the
    /// user's cache directory (`~/.cache/tomet/` on Unix), never inside the
    /// vault, so checking a large vault again does not parse what has not
    /// changed. Exits 1 if a link is broken or a document could not be read.
    CheckLinks(CheckLinksArgs),
    /// Migrate documents off spellings the language has moved away from: URLs
    /// that match a `@config` macro become `$macro(...)`, and `@meta{type: ..}`
    /// becomes `@kind(..)`.
    ///
    /// By default it prints what would change and writes nothing. `--check`
    /// exits 1 if any document needs changing, or could not be looked at.
    /// `-i` writes the changes; that re-prints the documents from the syntax
    /// tree, which carries no comments, so read the dry run first.
    Refactor(RefactorArgs),
    /// Write documents out in the formats their `@config(export: ..)` asks
    /// for, or check that what was written is still what the source renders
    /// to.
    ///
    /// A directory exports every document under it; one failing document
    /// does not stop the others, but the run exits 1. With `--check` nothing
    /// is written, and the run exits 1 if an output is out of date or missing,
    /// or a document could not be checked.
    Export(ExportArgs),
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

#[derive(Args)]
pub struct NewArgs {
    /// Path of the document to create (defaults to "document.tmt").
    #[arg(default_value = "document.tmt")]
    pub path: PathBuf,
    /// Blueprint name, as the blueprint gives it in `@blueprint(...)` (e.g.
    /// "daily-note", "rfc"). Resolved among the blueprints the vault declares
    /// in `blueprints`, never searched for on disk.
    #[arg(short, long)]
    pub blueprint: Option<String>,
    /// List the blueprints this vault declares instead of creating a file.
    #[arg(short, long)]
    pub list: bool,
    /// Overwrite the target file if it already exists.
    #[arg(short = 'f', long)]
    pub force: bool,
    /// Pass a custom variable as `key=value`. Can be given more than once.
    #[arg(long = "var", value_parser = parse_key_val)]
    pub vars: Vec<(String, String)>,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
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
pub struct RefactorArgs {
    /// File or directory to refactor (defaults to ".").
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Write the changed documents in place.
    #[arg(short = 'i', long, alias = "write")]
    pub in_place: bool,
    /// Only rewrite URLs that match macros.
    #[arg(long)]
    pub url_macros: bool,
    /// Only promote `@meta` type to `@kind`.
    #[arg(long)]
    pub meta_kind: bool,
    /// Check without modifying files; exits 1 if changes are needed.
    #[arg(long, conflicts_with = "in_place")]
    pub check: bool,
    /// Refactor files and directories the project config excludes
    /// (`workspace.ignore`, `workspace.unswept`) as well. Without it, a file
    /// named explicitly that the config excludes is skipped with a warning, as
    /// it would be in a directory sweep.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct ExportArgs {
    /// File or directory to export (defaults to ".").
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Format to export: `commonmark`, `html`, or `all`. Overrides what the
    /// documents declare.
    #[arg(short = 't', long)]
    pub r#type: Option<String>,
    /// Where to write. Overrides what the documents declare.
    #[arg(short, long)]
    pub out: Option<PathBuf>,
    /// Number HTML headings by nesting level and give them slugs.
    #[arg(long)]
    pub advanced: bool,
    /// Compare each declared output with what the source renders to now,
    /// without writing anything. Exits 1 if any differ or are missing: a
    /// generated file that disagrees with its source is a failure with a
    /// mechanical fix.
    #[arg(long)]
    pub check: bool,
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
