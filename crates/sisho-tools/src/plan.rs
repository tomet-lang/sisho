use std::path::Path;

use sisho_run::Step;

use crate::tool::Tool;

fn step(tool: Tool, args: &[&str], path: &Path) -> Step {
    let name = format!("{} {}", tool.name(), args.join(" "));
    Step::new(name, tool.name()).args(args).arg(path)
}

/// The tool steps of `sisho check`: verify `path` without writing anything.
///
/// Read-only, so a caller may run them at the same time. Plan order is the
/// order they are reported in. `sisho`'s own checks (`export --check`,
/// `refactor --check`) follow them; they are not tool steps and live with the
/// program.
pub fn check_steps(path: &Path) -> Vec<Step> {
    vec![
        step(Tool::Tomet, &["check"], path),
        step(Tool::Tomet, &["format", "--check"], path),
        step(Tool::Twrit, &["check"], path),
    ]
}

/// The tool steps of `sisho fix` that come before `sisho export`: regenerate.
///
/// `twrit check --fix` rewrites generated API documents. They have to be
/// right before `sisho export` renders documents from them, and before the
/// formatter sees them.
pub fn fix_generate_steps(path: &Path) -> Vec<Step> {
    vec![step(Tool::Twrit, &["check", "--fix"], path)]
}

/// The tool steps of `sisho fix` that come after `sisho export`: format.
///
/// Last, so the formatter sees what the generators and the export produced.
/// The order follows `twrit`'s own `just docs`.
pub fn fix_format_steps(path: &Path) -> Vec<Step> {
    vec![step(Tool::Tomet, &["format", "-i"], path)]
}
