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

mod versions {
    use crate::{Tool, Verdict, Version, judge};

    fn v(major: u64, minor: u64, patch: u64) -> Version {
        Version::new(major, minor, patch)
    }

    #[test]
    fn a_version_is_found_in_what_a_tool_prints() {
        assert_eq!(Version::find("tomet 0.2.0"), Some(v(0, 2, 0)));
        assert_eq!(Version::find("0.2.0"), Some(v(0, 2, 0)));
        assert_eq!(Version::find("twrit 1.14.7-dev"), Some(v(1, 14, 7)));
        assert_eq!(
            Version::find("tomet 0.3.1+build5 (abc123)"),
            Some(v(0, 3, 1))
        );
    }

    #[test]
    fn what_is_not_a_version_is_not_found() {
        assert_eq!(Version::find(""), None);
        assert_eq!(Version::find("tomet"), None);
        assert_eq!(Version::find("tomet 1.2"), None);
        assert_eq!(
            Version::find("error: unexpected argument '--version' found"),
            None
        );
    }

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(v(0, 10, 0) > v(0, 9, 0));
        assert!(v(1, 0, 0) > v(0, 99, 99));
        assert!(Version::find("x 0.10.0") > Version::find("x 0.9.0"));
    }

    #[test]
    fn a_version_at_or_above_the_minimum_is_fine() {
        assert_eq!(
            judge(Some("tomet 0.2.0"), v(0, 2, 0)),
            Verdict::Fine(v(0, 2, 0))
        );
        assert_eq!(
            judge(Some("tomet 0.3.0"), v(0, 2, 0)),
            Verdict::Fine(v(0, 3, 0))
        );
        assert_eq!(
            judge(Some("tomet 1.0.0"), v(0, 2, 0)),
            Verdict::Fine(v(1, 0, 0))
        );
    }

    #[test]
    fn an_older_version_is_too_old() {
        assert_eq!(
            judge(Some("tomet 0.1.9"), v(0, 2, 0)),
            Verdict::TooOld {
                found: v(0, 1, 9),
                needs: v(0, 2, 0)
            }
        );
    }

    #[test]
    fn no_version_at_all_is_older_than_any_minimum() {
        let needs = v(0, 2, 0);
        assert_eq!(judge(None, needs), Verdict::Unreported { needs });
        assert_eq!(
            judge(Some("not a version"), needs),
            Verdict::Unreported { needs }
        );
    }

    #[test]
    fn both_tools_need_at_least_0_2_0() {
        for tool in Tool::ALL {
            assert!(tool.min_version() >= v(0, 2, 0), "{}", tool.name());
        }
        assert!(Tool::Tomet.judge(Some("tomet 0.2.0")).is_fine());
        assert!(!Tool::Twrit.judge(Some("twrit 0.1.0")).is_fine());
    }
}
