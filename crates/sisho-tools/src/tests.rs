use std::path::Path;

use crate::{Tool, check_steps, fix_steps};

fn lines(steps: &[sisho_run::Step]) -> Vec<String> {
    steps.iter().map(|s| s.command_line()).collect()
}

#[test]
fn check_runs_these_commands_in_this_order() {
    let steps = check_steps(Path::new("vault"));
    assert_eq!(
        lines(&steps),
        [
            "tomet check vault",
            "tomet format --check vault",
            "tomet export --check vault",
            "twrit check vault",
        ]
    );
}

#[test]
fn fix_generates_before_the_export_and_formats_after_it() {
    assert_eq!(
        lines(&fix_steps(Path::new("vault"))),
        [
            "twrit check --fix vault",
            "tomet export vault",
            "tomet format -i vault",
        ]
    );
}

#[test]
fn check_never_writes() {
    let writes = ["--fix", "-i", "--in-place", "--write"];
    for step in check_steps(Path::new(".")) {
        for arg in step.arguments() {
            assert!(
                !writes.iter().any(|w| arg == w),
                "{} is a write flag in a check step: {}",
                arg.to_string_lossy(),
                step.command_line()
            );
        }
    }
}

#[test]
fn every_tool_is_in_the_check_plan() {
    let steps = check_steps(Path::new("."));
    for tool in Tool::ALL {
        assert!(
            steps.iter().any(|s| s.program() == tool.name()),
            "{} has no check step",
            tool.name()
        );
    }
}

#[test]
fn step_names_say_what_the_step_does() {
    let names: Vec<_> = check_steps(Path::new("."))
        .iter()
        .map(|s| s.name().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "tomet check",
            "tomet format --check",
            "tomet export --check",
            "twrit check"
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_path_is_passed_through_unchanged() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let odd = OsStr::from_bytes(b"caf\xe9");
    let steps = check_steps(Path::new(odd));
    assert!(steps[0].arguments().iter().any(|a| a == odd));
}

#[test]
fn tools_are_found_by_name() {
    assert_eq!(Tool::from_name("tomet"), Some(Tool::Tomet));
    assert_eq!(Tool::from_name("twrit"), Some(Tool::Twrit));
    assert_eq!(Tool::from_name("cargo"), None);
}

#[test]
fn the_version_step_asks_for_the_version() {
    assert_eq!(Tool::Twrit.version_step().command_line(), "twrit --version");
}
