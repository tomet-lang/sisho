use std::ffi::OsStr;
use std::path::Path;

use sisho_run::Step;

use crate::tool::Tool;

/// A format Tomet can be written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    CommonMark,
    Html,
    Typst,
    /// Pandoc's own AST, JSON-encoded: `pandoc -f json` turns it into every
    /// format Pandoc writes.
    Pandoc,
}

impl Target {
    /// The format called `name`: `commonmark` (or `md`), `html`, `typst`,
    /// `pandoc`.
    pub fn parse(name: &str) -> Option<Target> {
        match name.to_ascii_lowercase().as_str() {
            "commonmark" | "md" => Some(Target::CommonMark),
            "html" => Some(Target::Html),
            "typst" => Some(Target::Typst),
            "pandoc" => Some(Target::Pandoc),
            _ => None,
        }
    }

    /// The names [`Target::parse`] takes, for a message.
    pub const NAMES: &'static str = "commonmark (md), html, typst, pandoc";
}

/// A format that can be read as Tomet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    CommonMark,
    /// Pandoc's JSON AST (`pandoc -t json <file>`): the bridge from every
    /// format Pandoc reads.
    Pandoc,
}

impl Source {
    /// The format called `name`: `md` (or `commonmark`), `pandoc`.
    pub fn parse(name: &str) -> Option<Source> {
        match name.to_ascii_lowercase().as_str() {
            "commonmark" | "md" => Some(Source::CommonMark),
            "pandoc" => Some(Source::Pandoc),
            _ => None,
        }
    }

    /// The names [`Source::parse`] takes, for a message.
    pub const NAMES: &'static str = "md (commonmark), pandoc";
}

/// Which way a conversion goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// From Tomet.
    To(Target),
    /// To Tomet.
    From(Source),
}

impl Direction {
    /// The `tomet` command that converts this way.
    pub fn tomet_command(self) -> &'static str {
        match self {
            Direction::To(Target::CommonMark) => "to-md",
            Direction::To(Target::Html) => "html",
            Direction::To(Target::Typst) => "to-typst",
            Direction::To(Target::Pandoc) => "to-pandoc",
            Direction::From(Source::CommonMark) => "from-md",
            Direction::From(Source::Pandoc) => "from-pandoc",
        }
    }
}

/// One conversion, as `tomet` is asked to do it: what to convert, which way,
/// and where the result goes.
///
/// `tomet` does the work. It reads a file, a directory or standard input (`-`)
/// and writes to standard output, to `out`, or next to the source; `sisho`
/// holds the one verb over its six commands and leaves the rest to it.
#[derive(Debug, Clone)]
pub struct Conversion<'a> {
    /// A file, a directory, or `-` for standard input.
    pub path: &'a Path,
    pub direction: Direction,
    /// Write here instead of standard output.
    pub out: Option<&'a Path>,
    /// Write next to each source, with the new extension.
    pub in_place: bool,
    /// Delete each source once its result is written (a conversion to Tomet).
    pub remove_original: bool,
    /// Convert and check, and write nothing.
    pub dry_run: bool,
    /// Only for [`Target::Html`]: number headings and give them slugs.
    pub advanced: bool,
    /// Only for [`Target::Html`]: `<html lang="...">` of the page shell.
    pub lang: Option<&'a str>,
    /// Only for [`Target::Html`]: the rendered body alone, with no page shell.
    pub body: bool,
}

impl<'a> Conversion<'a> {
    /// `path` converted in `direction`, to standard output.
    pub fn new(path: &'a Path, direction: Direction) -> Self {
        Self {
            path,
            direction,
            out: None,
            in_place: false,
            remove_original: false,
            dry_run: false,
            advanced: false,
            lang: None,
            body: false,
        }
    }

    /// `tomet <command> [options] <path>`.
    pub fn step(&self) -> Step {
        let command = self.direction.tomet_command();
        let mut step = Step::new(format!("tomet {command}"), Tool::Tomet.name()).arg(command);
        if let Some(out) = self.out {
            step = step.arg("--out").arg(out);
        }
        for (set, flag) in [
            (self.in_place, "--in-place"),
            (self.remove_original, "--remove-original"),
            (self.dry_run, "--dry-run"),
            (self.advanced, "--advanced"),
            (self.body, "--body"),
        ] {
            if set {
                step = step.arg(flag);
            }
        }
        if let Some(lang) = self.lang {
            step = step.arg("--lang").arg(lang);
        }
        // `-` is standard input and is a path as it is; any other path that
        // starts with a dash would be read as an option without this.
        if self.path.as_os_str() != "-" && starts_with_dash(self.path.as_os_str()) {
            step = step.arg("--");
        }
        step.arg(self.path)
    }
}

fn starts_with_dash(path: &OsStr) -> bool {
    path.as_encoded_bytes().first() == Some(&b'-')
}
