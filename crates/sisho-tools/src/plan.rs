use std::path::Path;

use sisho_run::Step;

use crate::tool::Tool;

fn step(tool: Tool, args: &[&str], path: &Path) -> Step {
    let name = format!("{} {}", tool.name(), args.join(" "));
    Step::new(name, tool.name()).args(args).arg(path)
}

/// The steps of `sisho check`: verify `path` without writing anything.
///
/// Read-only, so a caller may run them at the same time. Plan order is the
/// order they are reported in.
pub fn check_steps(path: &Path) -> Vec<Step> {
    vec![
        step(Tool::Tomet, &["check"], path),
        step(Tool::Tomet, &["format", "--check"], path),
        step(Tool::Twrit, &["check"], path),
    ]
}

/// The steps of `sisho fix`: repair what the tools can repair in `path`.
///
/// These write files, so a caller runs them one after another, in this
/// order. The order follows `twrit`'s own `just docs`: regenerate first
/// (`twrit check --fix` rewrites generated API docs), format last, so the
/// formatter sees what the generators produced.
pub fn fix_steps(path: &Path) -> Vec<Step> {
    vec![
        step(Tool::Twrit, &["check", "--fix"], path),
        step(Tool::Tomet, &["format", "-i"], path),
    ]
}
