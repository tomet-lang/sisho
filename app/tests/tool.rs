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
    let sandbox = Sandbox::new("ext-args")
        .tool("sisho-desk")
        .trust_extension("desk");
    let output = sandbox.sisho(&["desk", "inspect", "--verbose", "-c", "foo.tmt"], "");
    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "args: inspect --verbose -c foo.tmt\n");
}

#[test]
fn an_extension_exit_code_is_kept() {
    let sandbox = Sandbox::new("ext-code")
        .tool("sisho-desk")
        .trust_extension("desk");
    assert_eq!(code(&sandbox.sisho(&["desk", "exit-42"], "")), 42);

    let output = sandbox.sisho(&["desk", "fail"], "sisho-desk:fail");
    assert_eq!(code(&output), 1);
    assert!(stderr(&output).contains("boom from sisho-desk fail"));
}

#[test]
fn an_extension_hands_over_standard_input() {
    let sandbox = Sandbox::new("ext-stdin")
        .tool("sisho-desk")
        .trust_extension("desk");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["desk", "read-stdin"])
        .env_clear()
        .env("PATH", &sandbox.dir)
        .env("SISHO_TRUST_FILE", sandbox.trust_file())
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
fn untrusted_extension_is_blocked_with_exit_126_and_never_executed() {
    let sandbox = Sandbox::new("ext-untrusted");
    let marker = sandbox.dir.join("ran-marker");
    let script = format!(
        "#!/bin/sh\n: > \"{}\"\necho \"should not run\"\n",
        marker.display()
    );
    let ext_path = sandbox.dir.join("sisho-desk");
    std::fs::write(&ext_path, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&ext_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let output = sandbox.sisho(&["desk", "arg1"], "");
    assert_eq!(code(&output), 126);
    assert_eq!(stdout(&output), "");
    assert!(!marker.exists(), "untrusted extension must never execute");
    let err = stderr(&output);
    assert!(
        err.contains("is not trusted (untrusted); run 'sisho --allow desk' to allow it"),
        "{err}"
    );
}

#[test]
fn allow_deny_and_untrust_lifecycle_controls_extension_execution() {
    let sandbox = Sandbox::new("ext-lifecycle").tool("sisho-desk");
    let canonical = std::fs::canonicalize(sandbox.dir.join("sisho-desk")).unwrap();

    // Initially untrusted -> blocked with 126
    let out_initial = sandbox.sisho(&["desk", "hello"], "");
    assert_eq!(code(&out_initial), 126);

    // Allow by word
    let out_allow = sandbox.sisho(&["--allow", "desk"], "");
    assert_eq!(code(&out_allow), 0);
    assert_eq!(
        stdout(&out_allow).trim(),
        format!("Allowed sisho-desk ({})", canonical.display())
    );

    // Verify trust.json contains canonical path -> "allowed"
    let trust_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(sandbox.trust_file()).unwrap()).unwrap();
    assert_eq!(trust_json["schema_version"], 1);
    assert_eq!(
        trust_json["binaries"][canonical.to_str().unwrap()],
        "allowed"
    );

    // Now execution succeeds
    let out_run = sandbox.sisho(&["desk", "hello"], "");
    assert_eq!(code(&out_run), 0);
    assert_eq!(stdout(&out_run), "args: hello\n");

    // Deny by word -> blocked with 126 and reports (denied)
    let out_deny = sandbox.sisho(&["--deny", "desk"], "");
    assert_eq!(code(&out_deny), 0);
    assert_eq!(
        stdout(&out_deny).trim(),
        format!("Denied sisho-desk ({})", canonical.display())
    );
    let out_denied_run = sandbox.sisho(&["desk", "hello"], "");
    assert_eq!(code(&out_denied_run), 126);
    assert!(
        stderr(&out_denied_run)
            .contains("is not trusted (denied); run 'sisho --allow desk' to allow it")
    );

    // Untrust by word -> resets to untrusted and removes from binaries map
    let out_untrust = sandbox.sisho(&["--untrust", "desk"], "");
    assert_eq!(code(&out_untrust), 0);
    assert_eq!(
        stdout(&out_untrust).trim(),
        format!(
            "Reset trust for sisho-desk ({}) to untrusted",
            canonical.display()
        )
    );
    let trust_json_after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(sandbox.trust_file()).unwrap()).unwrap();
    assert!(
        trust_json_after["binaries"]
            .as_object()
            .unwrap()
            .get(canonical.to_str().unwrap())
            .is_none()
    );

    let out_untrusted_run = sandbox.sisho(&["desk", "hello"], "");
    assert_eq!(code(&out_untrusted_run), 126);
    assert!(
        stderr(&out_untrusted_run)
            .contains("is not trusted (untrusted); run 'sisho --allow desk' to allow it")
    );
}

#[test]
fn trust_is_bound_to_canonical_path_not_just_extension_name() {
    let sb1 = Sandbox::new("ext-canon-1").tool("sisho-desk");
    let sb2 = Sandbox::new("ext-canon-2").tool("sisho-desk");
    let trust_file = sb1.trust_file();

    // Allow sisho-desk in sb1
    let out_allow = sb1.sisho(&["--allow", "desk"], "");
    assert_eq!(code(&out_allow), 0);

    // Running with PATH=sb2 but using the same trust.json must block sb2's sisho-desk!
    let out_sb2 = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["desk", "hello"])
        .env_clear()
        .env("PATH", &sb2.dir)
        .env("SISHO_TRUST_FILE", &trust_file)
        .output()
        .unwrap();
    assert_eq!(code(&out_sb2), 126);
    assert_eq!(stdout(&out_sb2), "");
    assert!(stderr(&out_sb2).contains("is not trusted (untrusted)"));
}

#[test]
fn allow_deny_untrust_reject_missing_targets_and_invalid_flag_combinations() {
    let sandbox = Sandbox::new("ext-flag-errors").tool("sisho-desk");

    // Missing extension target exits 2
    let out_missing = sandbox.sisho(&["--allow", "nonexistent"], "");
    assert_eq!(code(&out_missing), 2);
    assert!(stderr(&out_missing).contains("not found on PATH"));

    // Mutually exclusive flags exit 2
    assert_eq!(
        code(&sandbox.sisho(&["--allow", "desk", "--deny", "desk"], "")),
        2
    );
    assert_eq!(code(&sandbox.sisho(&["--allow", "desk", "--list"], "")), 2);
    assert_eq!(code(&sandbox.sisho(&["--allow", "desk", "check"], "")), 2);
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

#[test]
fn allowed_extension_receives_sisho_config_and_data_dirs() {
    let sandbox = Sandbox::new("ext-dirs");
    let script = "#!/bin/sh\nprintf '%s\\n%s\\n' \"$SISHO_CONFIG_DIR\" \"$SISHO_DATA_DIR\"\n";
    let ext_path = sandbox.dir.join("sisho-echo");
    std::fs::write(&ext_path, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&ext_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let sandbox = sandbox.trust_extension("echo");
    let home = sandbox.dir.join("home");
    let xdg_cfg = sandbox.dir.join("xdg-cfg");
    let xdg_data = sandbox.dir.join("xdg-data");

    // 1. With HOME fallback (and parent SISHO_*_DIR ignored):
    let out_home = sandbox.sisho_with(
        &["echo"],
        "",
        &[
            ("HOME", home.to_str().unwrap()),
            ("SISHO_CONFIG_DIR", "/stale/other-ext/cfg"),
            ("SISHO_DATA_DIR", "/stale/other-ext/data"),
        ],
    );
    assert_eq!(code(&out_home), 0);
    let out_home_stdout = stdout(&out_home);
    let home_lines: Vec<&str> = out_home_stdout.lines().collect();
    assert_eq!(
        home_lines,
        vec![
            home.join(".config/sisho/echo").to_str().unwrap(),
            home.join(".local/share/sisho/echo").to_str().unwrap(),
        ]
    );
    assert!(home_lines[0].ends_with("/sisho/echo"));
    assert!(home_lines[1].ends_with("/sisho/echo"));

    // 2. With XDG_CONFIG_HOME and XDG_DATA_HOME:
    let out_xdg = sandbox.sisho_with(
        &["echo"],
        "",
        &[
            ("HOME", home.to_str().unwrap()),
            ("XDG_CONFIG_HOME", xdg_cfg.to_str().unwrap()),
            ("XDG_DATA_HOME", xdg_data.to_str().unwrap()),
        ],
    );
    assert_eq!(code(&out_xdg), 0);
    let out_xdg_stdout = stdout(&out_xdg);
    let xdg_lines: Vec<&str> = out_xdg_stdout.lines().collect();
    assert_eq!(
        xdg_lines,
        vec![
            xdg_cfg.join("sisho/echo").to_str().unwrap(),
            xdg_data.join("sisho/echo").to_str().unwrap(),
        ]
    );
}
