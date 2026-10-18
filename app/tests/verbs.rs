//! `sisho check` and `sisho fix` against fake `tomet` and `twrit` programs on a
//! `PATH` the test controls, so no real tool is needed and every outcome can be
//! produced on demand.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

/// A directory holding fake tools, removed when dropped.
struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sisho-app-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    /// Install a fake tool called `name`. It prints `name 9.9.9` for
    /// `--version`, exits with `N` for `exit-N`, echoes the first line of its
    /// standard input for `read-stdin`, fails with a message when `FAKE_FAIL`
    /// lists `name:<first argument>`, and otherwise echoes its arguments.
    fn tool(self, name: &str) -> Self {
        let script = format!(
            "#!/bin/sh\n\
             case \"$1\" in --version) echo \"{name} 9.9.9\"; exit 0;; esac\n\
             case \"$1\" in exit-*) exit \"${{1#exit-}}\";; esac\n\
             case \"$1\" in read-stdin) read line; echo \"stdin: $line\"; exit 0;; esac\n\
             case \",$FAKE_FAIL,\" in *\",{name}:$1,\"*) echo \"boom from {name} $*\" >&2; exit 1;; esac\n\
             echo \"args: $*\"\n"
        );
        let path = self.dir.join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        self
    }

    fn both_tools(self) -> Self {
        self.tool("tomet").tool("twrit")
    }

    /// Run `sisho` with only this directory on `PATH`.
    fn sisho(&self, args: &[&str], fail: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sisho"))
            .args(args)
            .current_dir(&self.dir)
            .env_clear()
            .env("PATH", &self.dir)
            .env("FAKE_FAIL", fail)
            .output()
            .unwrap()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap()
}

#[test]
fn check_passes_when_every_tool_passes() {
    let sandbox = Sandbox::new("pass").both_tools();
    let output = sandbox.sisho(&["check"], "");
    let text = stdout(&output);
    assert_eq!(code(&output), 0, "{text}");
    assert!(text.contains("tomet 9.9.9, twrit 9.9.9"), "{text}");
    assert!(text.contains("✓ tomet check"), "{text}");
    assert!(text.contains("✓ tomet format --check"), "{text}");
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
            "twrit check",
            "sisho export --check",
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
    assert_eq!(json["steps"][2]["status"], "not-found");
    assert_eq!(json["tools"][1]["version"], Value::Null);
}

#[test]
fn fix_runs_in_the_fixed_order_and_keeps_going_after_a_failure() {
    let sandbox = Sandbox::new("fix").both_tools();

    let text = stdout(&sandbox.sisho(&["fix"], ""));
    // Regenerate, then export what was regenerated, then format the result.
    let generate = text.find("✓ twrit check --fix").expect(&text);
    let export = text.find("✓ sisho export ").expect(&text);
    let format = text.find("✓ tomet format -i").expect(&text);
    assert!(generate < export && export < format, "{text}");
    assert!(text.contains("all 3 steps passed"), "{text}");

    let output = sandbox.sisho(&["fix"], "twrit:check");
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("✗ twrit check --fix"), "{text}");
    assert!(text.contains("✓ tomet format -i"), "{text}");
}

#[test]
fn fix_has_a_json_form_too() {
    let sandbox = Sandbox::new("fix-json").both_tools();
    let output = sandbox.sisho(&["fix", "--json"], "");
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["steps"][0]["name"], "twrit check --fix");
    assert_eq!(json["steps"][1]["name"], "sisho export");
    assert_eq!(json["steps"][2]["name"], "tomet format -i");
}

#[test]
fn a_usage_error_exits_2_not_1() {
    let sandbox = Sandbox::new("usage").both_tools();
    assert_eq!(code(&sandbox.sisho(&["no-such-verb"], "")), 2);
    assert_eq!(
        code(&sandbox.sisho(&["check", "--json", "--verbose"], "")),
        2
    );
}

#[test]
fn a_tool_name_runs_the_tool_with_the_rest_of_the_command_line() {
    let sandbox = Sandbox::new("pass-through").both_tools();
    let output = sandbox.sisho(&["tomet", "format", "-i", "."], "");
    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "args: format -i .\n");
}

#[test]
fn flags_after_a_tool_name_belong_to_the_tool_not_to_sisho() {
    let sandbox = Sandbox::new("flags").both_tools();
    assert_eq!(
        stdout(&sandbox.sisho(&["tomet", "--version"], "")),
        "tomet 9.9.9\n"
    );
    // `--help` is the tool's too: sisho does not print its own help.
    assert_eq!(
        stdout(&sandbox.sisho(&["twrit", "--help"], "")),
        "args: --help\n"
    );
}

#[test]
fn the_tools_exit_code_becomes_sishos() {
    let sandbox = Sandbox::new("exit-code").both_tools();
    assert_eq!(code(&sandbox.sisho(&["tomet", "exit-3"], "")), 3);
    assert_eq!(code(&sandbox.sisho(&["twrit", "exit-0"], "")), 0);

    let output = sandbox.sisho(&["tomet", "format"], "tomet:format");
    assert_eq!(code(&output), 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("boom from tomet format"));
}

#[test]
fn a_pass_through_hands_over_standard_input() {
    use std::io::Write;
    use std::process::Stdio;

    let sandbox = Sandbox::new("stdin").both_tools();
    let mut child = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["tomet", "read-stdin"])
        .env_clear()
        .env("PATH", &sandbox.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"hello\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(stdout(&output), "stdin: hello\n");
}

#[test]
fn a_pass_through_to_a_tool_that_is_not_installed_exits_127() {
    let sandbox = Sandbox::new("pass-missing").tool("tomet");
    let output = sandbox.sisho(&["twrit", "check"], "");
    assert_eq!(code(&output), 127);
    assert!(String::from_utf8_lossy(&output.stderr).contains("twrit is not on PATH"));
}

#[test]
fn a_name_that_is_not_a_tool_is_a_usage_error_that_lists_the_tools() {
    let sandbox = Sandbox::new("unknown").both_tools();
    let output = sandbox.sisho(&["cargo", "build"], "");
    assert_eq!(code(&output), 2);
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(
        text.contains("`cargo` is neither a command nor a tool"),
        "{text}"
    );
    assert!(text.contains("tools: tomet, twrit"), "{text}");
}

#[test]
fn the_help_names_every_tool() {
    let sandbox = Sandbox::new("help").both_tools();
    let text = stdout(&sandbox.sisho(&["--help"], ""));
    assert!(text.contains("Tools: tomet, twrit"), "{text}");
}
