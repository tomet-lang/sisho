//! `sisho <tool> ...`, against fake tools.
#![cfg(unix)]

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use common::{Sandbox, code, stderr, stdout};

#[test]
fn a_tool_name_runs_the_tool_with_the_rest_of_the_command_line() {
    let sandbox = Sandbox::new("pass-through").all_tools();
    let output = sandbox.sisho(&["tomet", "format", "-i", "."], "");
    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "args: format -i .\n");
}

#[test]
fn flags_after_a_tool_name_belong_to_the_tool_not_to_sisho() {
    let sandbox = Sandbox::new("flags").all_tools();
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
    let sandbox = Sandbox::new("exit-code").all_tools();
    assert_eq!(code(&sandbox.sisho(&["tomet", "exit-3"], "")), 3);
    assert_eq!(code(&sandbox.sisho(&["twrit", "exit-0"], "")), 0);

    let output = sandbox.sisho(&["tomet", "format"], "tomet:format");
    assert_eq!(code(&output), 1);
    assert!(stderr(&output).contains("boom from tomet format"));
}

#[test]
fn it_hands_over_standard_input() {
    let sandbox = Sandbox::new("stdin").all_tools();
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
    let sandbox = Sandbox::new("unknown").all_tools();
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
    let sandbox = Sandbox::new("usage").all_tools();
    assert_eq!(code(&sandbox.sisho(&["no-such-verb"], "")), 2);
}

#[test]
fn the_help_names_every_tool() {
    let sandbox = Sandbox::new("help").all_tools();
    let text = stdout(&sandbox.sisho(&["--help"], ""));
    assert!(text.contains("Tools: tomet, twrit"), "{text}");
    assert!(
        text.contains(
            "Extensions: a program named `sisho-<word>` on PATH runs as `sisho <word>`, for example `sisho desk`."
        ),
        "{text}"
    );
}

#[test]
fn an_extension_runs_as_sisho_word_with_arguments_and_flags_unchanged() {
    let sandbox = Sandbox::new("ext-args").tool("sisho-desk");
    let output = sandbox.sisho(&["desk", "inspect", "--verbose", "-c", "foo.tmt"], "");
    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "args: inspect --verbose -c foo.tmt\n");
}

#[test]
fn an_extension_exit_code_is_kept() {
    let sandbox = Sandbox::new("ext-code").tool("sisho-desk");
    assert_eq!(code(&sandbox.sisho(&["desk", "exit-42"], "")), 42);

    let output = sandbox.sisho(&["desk", "fail"], "sisho-desk:fail");
    assert_eq!(code(&output), 1);
    assert!(stderr(&output).contains("boom from sisho-desk fail"));
}

#[test]
fn an_extension_hands_over_standard_input() {
    let sandbox = Sandbox::new("ext-stdin").tool("sisho-desk");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["desk", "read-stdin"])
        .env_clear()
        .env("PATH", &sandbox.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"hello desk\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(stdout(&output), "stdin: hello desk\n");
}

#[test]
fn an_unknown_word_with_no_extension_on_path_exits_2_and_explains_extensions() {
    let sandbox = Sandbox::new("ext-unknown").all_tools();
    let output = sandbox.sisho(&["desk"], "");
    assert_eq!(code(&output), 2);
    let text = stderr(&output);
    assert!(
        text.contains("`desk` is neither a command nor a tool"),
        "{text}"
    );
    assert!(
        text.contains("an extension is a program named `sisho-<word>` on PATH"),
        "{text}"
    );
}

#[test]
fn built_in_commands_and_known_programs_win_over_extensions() {
    let sandbox = Sandbox::new("ext-precedence")
        .tool("sisho-tomet")
        .tool("sisho-twrit")
        .tool("sisho-check");

    // `sisho tomet` must look for `tomet`, not fallback to `sisho-tomet`.
    // Since `tomet` is not on PATH, it must exit 127 with the missing tool error,
    // and `sisho-tomet` must never run (stdout is empty).
    let output = sandbox.sisho(&["tomet", "format", "."], "");
    assert_eq!(code(&output), 127);
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("tomet is not on PATH"));

    // `sisho twrit` must similarly look for `twrit`, not `sisho-twrit`.
    let output = sandbox.sisho(&["twrit", "check", "."], "");
    assert_eq!(code(&output), 127);
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("twrit is not on PATH"));

    // `sisho check` is a built-in command parsed by clap; it must run sisho's
    // check pipeline (reporting missing tools and exiting 1), never `sisho-check`.
    let output = sandbox.sisho(&["check", "."], "");
    assert_eq!(code(&output), 1);
    let text = stdout(&output);
    assert!(text.contains("not found on PATH"), "{text}");
    assert!(!text.starts_with("args: "), "{text}");
}

#[test]
fn invalid_extension_words_are_usage_errors() {
    let sandbox = Sandbox::new("ext-invalid").all_tools().tool("sisho-desk");

    // Paths, leading hyphens, and upper case must be usage errors (exit 2), never running anything
    assert_eq!(code(&sandbox.sisho(&["../x"], "")), 2);
    assert_eq!(code(&sandbox.sisho(&["a/b"], "")), 2);
    assert_eq!(code(&sandbox.sisho(&["Desk"], "")), 2);
    assert_eq!(code(&sandbox.sisho(&["-desk"], "")), 2);
}
