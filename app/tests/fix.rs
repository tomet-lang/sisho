//! `sisho fix`, against fake tools.
#![cfg(unix)]

mod common;

use common::{Sandbox, code, stdout};
use serde_json::Value;

#[test]
fn fix_runs_in_the_fixed_order_and_keeps_going_after_a_failure() {
    let sandbox = Sandbox::new("fix").all_tools();

    let text = stdout(&sandbox.sisho(&["fix"], ""));
    // Regenerate, then export what was regenerated, then format the result.
    let generate = text.find("✓ twrit check --fix").expect(&text);
    let export = text.find("✓ tomet export ").expect(&text);
    let format = text.find("✓ tomet format -i").expect(&text);
    assert!(generate < export && export < format, "{text}");
    assert!(text.contains("all 3 steps passed"), "{text}");

    let output = sandbox.sisho(&["fix"], "twrit:check");
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ twrit check --fix"), "{text}");
    assert!(text.contains("✓ tomet export"), "{text}");
    assert!(text.contains("✓ tomet format -i"), "{text}");
}

#[test]
fn fix_has_a_json_form_too() {
    let sandbox = Sandbox::new("fix-json").all_tools();
    let output = sandbox.sisho(&["fix", "--json"], "");
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["steps"][0]["name"], "twrit check --fix");
    assert_eq!(json["steps"][1]["name"], "tomet export");
    assert_eq!(json["steps"][2]["name"], "tomet format -i");
}

#[test]
fn fix_never_runs_refactor() {
    // Migrating a document re-prints it from its syntax tree, which has no
    // comments. That is the language's own `refactor`, in `tomet`, and it is
    // something to ask for by name; `fix` does not run it.
    let sandbox = Sandbox::new("no-refactor").all_tools();
    let output = sandbox.sisho(&["fix", "--verbose"], "");
    assert!(!stdout(&output).contains("refactor"), "{}", stdout(&output));
}

#[test]
fn fix_writes_nothing_with_a_tool_older_than_sisho_works_with() {
    let sandbox = Sandbox::new("fix-old").all_tools();
    let output = sandbox.sisho_with(&["fix"], "", &[("FAKE_VERSION_TWRIT", "0.1.0")]);
    let text = stdout(&output);

    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ twrit version"), "{text}");
    assert!(text.contains("nothing was changed"), "{text}");
    // None of the steps that write was run.
    assert!(!text.contains("twrit check --fix"), "{text}");
    assert!(!text.contains("tomet export"), "{text}");
    assert!(!text.contains("tomet format -i"), "{text}");
}

#[test]
fn fix_with_a_tool_older_than_sisho_works_with_has_a_json_form_too() {
    let sandbox = Sandbox::new("fix-old-json").all_tools();
    let output = sandbox.sisho_with(&["fix", "--json"], "", &[("FAKE_NO_VERSION", "tomet")]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(json["ok"], false);
    assert_eq!(json["steps"].as_array().unwrap().len(), 1);
    assert_eq!(json["steps"][0]["name"], "tomet version");
}
