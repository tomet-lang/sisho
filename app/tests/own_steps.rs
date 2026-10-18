//! The steps `sisho` does itself inside `check` and `fix`: `export` and
//! `refactor --check`. The tool steps are fake programs that always pass, so
//! what is under test is `sisho`'s own part and the order it is run in.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A project directory with fake `tomet` and `twrit` programs in it, removed
/// when dropped. The project is also the working directory and the `PATH`.
struct Project(PathBuf);

impl Project {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sisho-app-own-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let project = Self(dir);
        for tool in ["tomet", "twrit"] {
            project.write(
                tool,
                &format!(
                    "#!/bin/sh\ncase \"$1\" in --version) echo \"{tool} 9.9.9\";; esac\nexit 0\n"
                ),
            );
            let path = project.0.join(tool);
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        project.write("default.config.tmt", "@kind(config)\n");
        project
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.0.join(rel)).unwrap()
    }

    fn sisho(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sisho"))
            .args(args)
            .current_dir(&self.0)
            .env_clear()
            .env("PATH", &self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap()
}

const WITH_EXPORT: &str =
    "@config(export: { type: commonmark, path: \"out/readme.md\" })\n\n=[ Readme ]\n\nHello\n";
const OLD_SPELLING: &str = "@meta{\n  type: note\n}\n";

#[test]
fn check_runs_the_tools_then_its_own_checks_and_all_pass_on_a_clean_project() {
    let project = Project::new("clean");
    let output = project.sisho(&["check"]);
    let text = stdout(&output);

    assert_eq!(code(&output), 0, "{text}");
    let order = [
        "✓ tomet check",
        "✓ tomet format --check",
        "✓ twrit check",
        "✓ sisho export --check",
        "✓ sisho refactor --check",
    ];
    let mut at = 0;
    for step in order {
        let found = text[at..]
            .find(step)
            .unwrap_or_else(|| panic!("{step} out of order:\n{text}"));
        at += found + step.len();
    }
    assert!(text.contains("all 5 steps passed"), "{text}");
}

#[test]
fn a_stale_export_fails_the_check_and_names_the_file() {
    let project = Project::new("stale");
    project.write("docs/readme.tmt", WITH_EXPORT);

    let output = project.sisho(&["check"]);
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ sisho export --check"), "{text}");
    assert!(text.contains("Stale export:"), "{text}");
    assert!(text.contains("✓ sisho refactor --check"), "{text}");
    assert!(
        !project.0.join("out/readme.md").exists(),
        "check writes nothing"
    );
}

#[test]
fn a_document_in_the_old_spelling_fails_the_check_and_points_at_the_commands() {
    let project = Project::new("old");
    project.write("old.tmt", OLD_SPELLING);

    let output = project.sisho(&["check"]);
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ sisho refactor --check"), "{text}");
    assert!(text.contains("Needs refactor (1 change(s)):"), "{text}");
    assert!(
        text.contains("hint: `sisho refactor` shows what would change"),
        "{text}"
    );
    assert!(text.contains("drops comments"), "{text}");
    assert_eq!(project.read("old.tmt"), OLD_SPELLING);
}

#[test]
fn a_document_that_cannot_be_read_fails_both_of_sishos_checks() {
    let project = Project::new("unreadable");
    project.write("broken.tmt", "=[ unterminated\n");

    let output = project.sisho(&["check"]);
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    // Each check says it did not look at the document, rather than passing.
    assert!(text.contains("✗ sisho export --check"), "{text}");
    assert!(text.contains("✗ sisho refactor --check"), "{text}");
    assert!(text.contains("could not be checked"), "{text}");
}

#[test]
fn json_names_the_own_steps_with_a_command_line() {
    let project = Project::new("json");
    let output = project.sisho(&["check", "--json"]);
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let steps = json["steps"].as_array().unwrap();
    assert_eq!(steps[3]["name"], "sisho export --check");
    assert_eq!(steps[3]["command"], "sisho export --check .");
    assert_eq!(steps[3]["status"], "passed");
    assert_eq!(steps[4]["name"], "sisho refactor --check");
}

#[test]
fn fix_exports_between_the_tools_and_then_check_passes() {
    let project = Project::new("fix");
    project.write("docs/readme.tmt", WITH_EXPORT);

    let output = project.sisho(&["fix"]);
    let text = stdout(&output);
    assert_eq!(code(&output), 0, "{text}");
    let generate = text.find("✓ twrit check --fix").unwrap();
    let export = text.find("✓ sisho export ").unwrap();
    let format = text.find("✓ tomet format -i").unwrap();
    assert!(generate < export && export < format, "{text}");

    assert!(project.read("out/readme.md").contains("# Readme"));
    assert_eq!(code(&project.sisho(&["check"])), 0);
}

/// `refactor -i` re-prints a document from its syntax tree, which has no
/// comments, so `fix` leaves it to be asked for by name.
#[test]
fn fix_does_not_rewrite_documents_in_the_old_spelling() {
    let project = Project::new("fix-no-refactor");
    project.write("old.tmt", OLD_SPELLING);

    let output = project.sisho(&["fix"]);
    assert_eq!(code(&output), 0, "{}", stdout(&output));
    assert!(!stdout(&output).contains("refactor"), "{}", stdout(&output));
    assert_eq!(project.read("old.tmt"), OLD_SPELLING);
}
