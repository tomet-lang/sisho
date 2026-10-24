use std::path::Path;

use crate::{
    Conversion, Direction, SHELF, Source, Target, Tool, check_links_step, check_steps, fix_steps,
    program,
};

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
            "catasto check vault",
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
            "twrit check",
            "catasto check",
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
fn programs_are_found_by_word() {
    assert_eq!(program("tomet"), Some(Tool::Tomet.as_program()));
    assert_eq!(program("twrit"), Some(Tool::Twrit.as_program()));
    assert_eq!(program("catasto"), Some(Tool::Catasto.as_program()));
    assert_eq!(program("shelf"), Some(SHELF));
    assert_eq!(program("cargo"), None);
}

#[test]
fn a_tool_runs_under_its_own_name() {
    for tool in Tool::ALL {
        let program = tool.as_program();
        assert_eq!(program.word(), tool.name());
        assert_eq!(program.program(), tool.name());
    }
}

#[test]
fn the_shelf_word_runs_tomet_shelf() {
    assert_eq!(SHELF.word(), "shelf");
    assert_eq!(SHELF.program(), "tomet-shelf");
}

#[test]
fn shelf_takes_no_part_in_check_or_fix() {
    // Not a `Tool`, so it has no version step and no minimum version, and
    // neither plan runs it.
    assert!(!Tool::ALL.iter().any(|tool| tool.as_program() == SHELF));
    let path = Path::new(".");
    for step in check_steps(path).iter().chain(&fix_steps(path)) {
        assert_ne!(step.program(), SHELF.program(), "{}", step.command_line());
    }
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
    fn twrit_needs_at_least_0_2_0_and_catasto_0_1_0() {
        assert_eq!(Tool::Twrit.min_version(), v(0, 2, 0));
        assert_eq!(Tool::Catasto.min_version(), v(0, 1, 0));
        assert!(Tool::Twrit.judge(Some("twrit 0.2.0")).is_fine());
        assert!(!Tool::Twrit.judge(Some("twrit 0.1.0")).is_fine());
        assert!(Tool::Catasto.judge(Some("catasto 0.1.0")).is_fine());
        assert!(!Tool::Catasto.judge(None).is_fine());
    }

    /// `sisho convert` leaves directories, `-` and `--dry-run` to `tomet`,
    /// which does all of that for every conversion from 0.4.0 on, and the
    /// counter reads `tomet check-links --json`, which 0.5.0 brought back.
    #[test]
    fn tomet_needs_the_version_with_check_links_and_conversions_that_take_every_input() {
        assert_eq!(Tool::Tomet.min_version(), v(0, 5, 0));
        assert!(Tool::Tomet.judge(Some("tomet 0.5.0")).is_fine());
        assert!(!Tool::Tomet.judge(Some("tomet 0.4.0")).is_fine());
    }
}

mod conversions {
    use std::path::Path;

    use super::*;

    fn line(conversion: &Conversion<'_>) -> String {
        conversion.step().command_line()
    }

    #[test]
    fn each_way_is_the_tomet_command_that_converts_that_way() {
        let path = Path::new("doc");
        for (direction, command) in [
            (Direction::To(Target::CommonMark), "to-md"),
            (Direction::To(Target::Html), "html"),
            (Direction::To(Target::Typst), "to-typst"),
            (Direction::To(Target::Pandoc), "to-pandoc"),
            (Direction::From(Source::CommonMark), "from-md"),
            (Direction::From(Source::Pandoc), "from-pandoc"),
        ] {
            let conversion = Conversion::new(path, direction);
            assert_eq!(line(&conversion), format!("tomet {command} doc"));
            assert_eq!(conversion.step().name(), format!("tomet {command}"));
        }
    }

    #[test]
    fn where_the_result_goes_is_passed_on() {
        let out = Path::new("out");
        let mut conversion = Conversion::new(Path::new("src"), Direction::From(Source::CommonMark));
        conversion.out = Some(out);
        conversion.remove_original = true;
        assert_eq!(
            line(&conversion),
            "tomet from-md --out out --remove-original src"
        );

        let mut conversion = Conversion::new(Path::new("src"), Direction::To(Target::Typst));
        conversion.in_place = true;
        conversion.dry_run = true;
        assert_eq!(line(&conversion), "tomet to-typst --in-place --dry-run src");
    }

    #[test]
    fn the_html_options_are_passed_on() {
        let mut conversion = Conversion::new(Path::new("doc.tmt"), Direction::To(Target::Html));
        conversion.advanced = true;
        conversion.lang = Some("en");
        conversion.body = true;
        assert_eq!(
            line(&conversion),
            "tomet html --advanced --body --lang en doc.tmt"
        );
    }

    #[test]
    fn standard_input_is_a_path_as_it_is() {
        let conversion = Conversion::new(Path::new("-"), Direction::From(Source::Pandoc));
        assert_eq!(line(&conversion), "tomet from-pandoc -");
    }

    /// A file called `-x.tmt` is a file, not an option of `tomet`.
    #[test]
    fn a_path_that_starts_with_a_dash_is_not_taken_for_an_option() {
        let conversion = Conversion::new(Path::new("-x.tmt"), Direction::To(Target::Html));
        assert_eq!(line(&conversion), "tomet html -- -x.tmt");
    }

    #[test]
    fn the_formats_are_found_by_name_in_any_case_with_their_aliases() {
        assert_eq!(Target::parse("MD"), Some(Target::CommonMark));
        assert_eq!(Target::parse("commonmark"), Some(Target::CommonMark));
        assert_eq!(Target::parse("Typst"), Some(Target::Typst));
        assert_eq!(Target::parse("docx"), None);
        assert_eq!(Source::parse("md"), Some(Source::CommonMark));
        assert_eq!(Source::parse("PANDOC"), Some(Source::Pandoc));
        assert_eq!(Source::parse("html"), None);
    }
}

#[test]
fn the_link_check_is_tomets_and_asks_for_json() {
    let step = check_links_step(Path::new("vault"));
    assert_eq!(step.command_line(), "tomet check-links --json vault");
    assert_eq!(step.name(), "tomet check-links --json");
}

/// Not part of `check` yet: `tomet` has broken links to clean up first.
#[test]
fn check_does_not_run_the_link_check_yet() {
    for step in check_steps(Path::new(".")) {
        assert!(
            !step.command_line().contains("check-links"),
            "{}",
            step.command_line()
        );
    }
}
