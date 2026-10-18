//! `sisho <tool> ...`, against fake tools.
#![cfg(unix)]

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use common::{Sandbox, code, stderr, stdout};

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
    assert!(stderr(&output).contains("boom from tomet format"));
}

#[test]
fn it_hands_over_standard_input() {
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
fn a_tool_that_is_not_installed_exits_127() {
    let sandbox = Sandbox::new("pass-missing").tool("tomet");
    let output = sandbox.sisho(&["twrit", "check"], "");
    assert_eq!(code(&output), 127);
    assert!(stderr(&output).contains("twrit is not on PATH"));
}

#[test]
fn a_name_that_is_not_a_tool_is_a_usage_error_that_lists_the_tools() {
    let sandbox = Sandbox::new("unknown").both_tools();
    let output = sandbox.sisho(&["cargo", "build"], "");
    assert_eq!(code(&output), 2);
    let text = stderr(&output);
    assert!(
        text.contains("`cargo` is neither a command nor a tool"),
        "{text}"
    );
    assert!(text.contains("tools: tomet, twrit"), "{text}");
}

#[test]
fn an_unknown_verb_is_a_usage_error_too() {
    let sandbox = Sandbox::new("usage").both_tools();
    assert_eq!(code(&sandbox.sisho(&["no-such-verb"], "")), 2);
}

#[test]
fn the_help_names_every_tool() {
    let sandbox = Sandbox::new("help").both_tools();
    let text = stdout(&sandbox.sisho(&["--help"], ""));
    assert!(text.contains("Tools: tomet, twrit"), "{text}");
}
