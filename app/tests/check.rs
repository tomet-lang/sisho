//! `sisho check`, against fake tools.
#![cfg(unix)]

mod common;

use common::{Sandbox, code, stdout};
use serde_json::Value;

#[test]
fn check_passes_when_every_tool_passes() {
    let sandbox = Sandbox::new("pass").both_tools();
    let output = sandbox.sisho(&["check"], "");
    let text = stdout(&output);
    assert_eq!(code(&output), 0, "{text}");
    assert!(text.contains("tomet 9.9.9, twrit 9.9.9"), "{text}");
    assert!(text.contains("✓ tomet check"), "{text}");
    assert!(text.contains("✓ tomet format --check"), "{text}");
    assert!(text.contains("✓ tomet export --check"), "{text}");
    assert!(text.contains("✓ twrit check"), "{text}");
    assert!(text.contains("all 4 steps passed"), "{text}");
}

#[test]
fn a_failing_step_fails_the_run_and_shows_why_but_stops_nothing() {
    let sandbox = Sandbox::new("fail").both_tools();
    let output = sandbox.sisho(&["check"], "tomet:format");
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ tomet format --check"), "{text}");
    assert!(text.contains(": exit 1"), "{text}");
    assert!(text.contains("boom from tomet format --check"), "{text}");
    // The others still ran, and still passed.
    assert!(text.contains("✓ tomet check"), "{text}");
    assert!(text.contains("✓ tomet export --check"), "{text}");
    assert!(text.contains("✓ twrit check"), "{text}");
    assert!(text.contains("1 of 4 steps did not pass"), "{text}");
}

#[test]
fn a_tool_that_is_not_installed_is_a_failure_not_a_skip() {
    let sandbox = Sandbox::new("missing").tool("tomet");
    let output = sandbox.sisho(&["check"], "");
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ twrit check"), "{text}");
    assert!(text.contains("not found on PATH"), "{text}");
    assert!(text.contains("twrit: version unknown"), "{text}");
}

#[test]
fn output_of_a_passing_step_is_shown_only_when_asked() {
    let sandbox = Sandbox::new("verbose").both_tools();

    let quiet = stdout(&sandbox.sisho(&["check"], ""));
    assert!(!quiet.contains("args: check"), "{quiet}");

    let verbose = stdout(&sandbox.sisho(&["check", "--verbose"], ""));
    assert!(verbose.contains("args: check ."), "{verbose}");
}

#[test]
fn the_path_is_handed_to_every_step() {
    let sandbox = Sandbox::new("path").both_tools();
    let text = stdout(&sandbox.sisho(&["check", "-v", "vault"], ""));
    assert!(text.contains("args: check vault"), "{text}");
    assert!(text.contains("args: format --check vault"), "{text}");
    assert!(text.contains("args: export --check vault"), "{text}");
    assert!(text.starts_with("sisho check vault\n"), "{text}");
}

#[test]
fn json_reports_every_step_and_whether_the_run_passed() {
    let sandbox = Sandbox::new("json").both_tools();
    let output = sandbox.sisho(&["check", "--json"], "tomet:format");
    assert_eq!(code(&output), 1);

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["tools"][0]["name"], "tomet");
    assert_eq!(json["tools"][0]["version"], "tomet 9.9.9");

    let steps = json["steps"].as_array().unwrap();
    let names: Vec<_> = steps.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "tomet check",
            "tomet format --check",
            "tomet export --check",
            "twrit check",
        ]
    );
    assert_eq!(steps[0]["status"], "passed");
    assert_eq!(steps[1]["status"], "failed");
    assert_eq!(steps[1]["exit_code"], 1);
    assert!(steps[1]["stderr"].as_str().unwrap().contains("boom"));
}

#[test]
fn json_marks_a_missing_tool() {
    let sandbox = Sandbox::new("json-missing").tool("tomet");
    let output = sandbox.sisho(&["check", "--json"], "");
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["steps"][3]["status"], "not-found");
    assert_eq!(json["tools"][1]["version"], Value::Null);
}

#[test]
fn json_and_verbose_cannot_be_combined() {
    let sandbox = Sandbox::new("usage").both_tools();
    assert_eq!(
        code(&sandbox.sisho(&["check", "--json", "--verbose"], "")),
        2
    );
}

#[test]
fn a_tool_older_than_sisho_works_with_is_a_failed_step_first_in_the_report() {
    let sandbox = Sandbox::new("old-tool").both_tools();
    let output = sandbox.sisho_with(&["check"], "", &[("FAKE_VERSION_TOMET", "0.1.0")]);
    let text = stdout(&output);

    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ tomet version"), "{text}");
    assert!(
        text.contains("tomet 0.1.0 is older than sisho needs (0.4.0 or newer): update tomet"),
        "{text}"
    );
    // First in the report, and the tool's own steps still ran: they only read.
    assert!(
        text.find("tomet version") < text.find("tomet check"),
        "{text}"
    );
    assert!(text.contains("✓ twrit check"), "{text}");
    assert!(text.contains("1 of 5 steps did not pass"), "{text}");
}

#[test]
fn a_tool_that_cannot_report_a_version_is_older_than_any_minimum() {
    let sandbox = Sandbox::new("no-version").both_tools();
    let output = sandbox.sisho_with(&["check"], "", &[("FAKE_NO_VERSION", "twrit")]);
    let text = stdout(&output);

    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ twrit version"), "{text}");
    assert!(
        text.contains("`twrit --version` did not give a version, so twrit is older than sisho needs (0.2.0 or newer): update twrit"),
        "{text}"
    );
    assert!(text.contains("twrit: version unknown"), "{text}");
}

#[test]
fn a_tool_exactly_at_the_minimum_or_above_it_is_fine() {
    let sandbox = Sandbox::new("new-enough").both_tools();
    for version in ["0.4.0", "0.4.1", "0.10.0", "1.0.0"] {
        let output = sandbox.sisho_with(&["check"], "", &[("FAKE_VERSION_TOMET", version)]);
        assert_eq!(code(&output), 0, "{version}: {}", stdout(&output));
    }
}

#[test]
fn a_tool_that_is_not_installed_is_not_also_a_version_problem() {
    let sandbox = Sandbox::new("missing-no-extra").tool("tomet");
    let text = stdout(&sandbox.sisho(&["check"], ""));
    assert!(text.contains("✗ twrit check"), "{text}");
    assert!(!text.contains("twrit version"), "{text}");
}

#[test]
fn json_says_what_each_tool_needs_and_lists_a_version_problem_as_a_step() {
    let sandbox = Sandbox::new("json-needs").both_tools();
    let output = sandbox.sisho_with(&["check", "--json"], "", &[("FAKE_VERSION_TWRIT", "0.1.5")]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    // Each tool has a minimum of its own: tomet's moved with its conversions.
    assert_eq!(json["tools"][0]["needs"], "0.4.0");
    assert_eq!(json["tools"][1]["needs"], "0.2.0");
    assert_eq!(json["tools"][1]["version"], "twrit 0.1.5");
    assert_eq!(json["ok"], false);
    assert_eq!(json["steps"][0]["name"], "twrit version");
    assert_eq!(json["steps"][0]["status"], "failed");
}
