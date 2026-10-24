use std::path::Path;

use sisho_run::Step;

use crate::tool::Tool;

fn step(tool: Tool, args: &[&str], path: &Path) -> Step {
    let name = format!("{} {}", tool.name(), args.join(" "));
    Step::new(name, tool.name()).args(args).arg(path)
}

/// The step that looks for links that point at nothing under `path`:
/// `tomet check-links --json`.
///
/// It is not in [`check_steps`]: `tomet` has broken links of its own to clean
/// up first. A caller that wants the list reads the JSON on standard output;
/// the step exits 1 when there is a broken link or a document it could not
/// read, and the JSON is there all the same.
pub fn check_links_step(path: &Path) -> Step {
    step(Tool::Tomet, &["check-links", "--json"], path)
}

/// The steps of `sisho check`: verify `path` without writing anything.
///
/// Read-only, so a caller may run them at the same time. Plan order is the
/// order they are reported in. It is the gate each repository's `docs-check`
/// has been spelling out by hand: the documents parse, they are formatted,
/// what they export is up to date, the repository's written rules hold, and
/// the files `catasto` manages are as its lock says.
pub fn check_steps(path: &Path) -> Vec<Step> {
    vec![
        step(Tool::Tomet, &["check"], path),
        step(Tool::Tomet, &["format", "--check"], path),
        step(Tool::Tomet, &["export", "--check"], path),
        step(Tool::Twrit, &["check"], path),
        step(Tool::Catasto, &["check"], path),
    ]
}

/// The steps of `sisho fix`: repair what the tools can repair in `path`.
///
/// These write files, so a caller runs them one after another, in this
/// order. The order follows `twrit`'s own `just docs`: regenerate first
/// (`twrit check --fix` rewrites generated API docs), then export what the
/// documents declare, which may include those generated documents, and format
/// last, so the formatter sees what the generators and the export produced.
///
/// `catasto sync` is not here. It reads the network when a commit is not cached
/// and puts a managed file back to what the lock says, which no step of `fix`
/// does to a file someone may have edited. Whether it joins is open (see the
/// design memo).
pub fn fix_steps(path: &Path) -> Vec<Step> {
    vec![
        step(Tool::Twrit, &["check", "--fix"], path),
        step(Tool::Tomet, &["export"], path),
        step(Tool::Tomet, &["format", "-i"], path),
    ]
}
