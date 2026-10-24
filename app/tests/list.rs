//! `sisho --list`, listing installed extensions.
#![cfg(unix)]

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use common::{Sandbox, code, stderr, stdout};

fn make_file(dir: &std::path::Path, name: &str, executable: bool) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, "#!/bin/sh\necho ok\n").unwrap();
    let mode = if executable { 0o755 } else { 0o644 };
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    path
}

#[test]
fn none_found_is_success_with_empty_output() {
    let sandbox = Sandbox::new("list-none").all_tools();
    let output = sandbox.sisho(&["--list"], "");
    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn lists_one_installed_extension() {
    let sandbox = Sandbox::new("list-one");
    let ext_path = make_file(&sandbox.dir, "sisho-desk", true);

    let output = sandbox.sisho(&["--list"], "");
    assert_eq!(code(&output), 0);
    let out_str = stdout(&output);
    assert_eq!(out_str.trim(), format!("desk      {}", ext_path.display()));
}

#[test]
fn lists_multiple_extensions_across_different_directories() {
    let sb1 = Sandbox::new("list-multi-1");
    let sb2 = Sandbox::new("list-multi-2");

    let path_beta = make_file(&sb1.dir, "sisho-beta", true);
    let path_alpha = make_file(&sb2.dir, "sisho-alpha", true);

    let combined_path = format!("{}:{}", sb1.dir.display(), sb2.dir.display());
    let output = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["--list"])
        .env_clear()
        .env("PATH", &combined_path)
        .output()
        .unwrap();

    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    // Alphabetical sort: alpha before beta
    assert!(lines[0].starts_with("alpha "));
    assert!(lines[0].contains(&path_alpha.display().to_string()));
    assert!(lines[1].starts_with("beta "));
    assert!(lines[1].contains(&path_beta.display().to_string()));
}

#[test]
fn ignores_unexecutable_files() {
    let sandbox = Sandbox::new("list-unexec");
    make_file(&sandbox.dir, "sisho-desk", false); // 0o644, not executable

    let output = sandbox.sisho(&["--list"], "");
    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "");
}

#[test]
fn ignores_files_with_invalid_extension_words() {
    let sandbox = Sandbox::new("list-invalid");
    make_file(&sandbox.dir, "sisho-Desk", true); // uppercase
    make_file(&sandbox.dir, "sisho--desk", true); // starts with hyphen
    make_file(&sandbox.dir, "sisho-desk.sh", true); // contains dot
    make_file(&sandbox.dir, "sisho-", true); // empty word
    make_file(&sandbox.dir, "other-desk", true); // does not start with sisho-

    let output = sandbox.sisho(&["--list"], "");
    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "");
}

#[test]
fn marks_names_shadowed_by_builtins_or_known_tools() {
    let sandbox = Sandbox::new("list-precedence");
    let path_check = make_file(&sandbox.dir, "sisho-check", true);
    let path_tomet = make_file(&sandbox.dir, "sisho-tomet", true);
    let path_desk = make_file(&sandbox.dir, "sisho-desk", true);

    let output = sandbox.sisho(&["--list"], "");
    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);

    // Sorted alphabetically: check, desk, tomet
    assert!(lines[0].starts_with("check "));
    assert!(lines[0].contains(&path_check.display().to_string()));
    assert!(lines[0].ends_with("(shadowed by builtin)"));

    assert!(lines[1].starts_with("desk "));
    assert!(lines[1].contains(&path_desk.display().to_string()));
    assert!(!lines[1].contains("(shadowed"));

    assert!(lines[2].starts_with("tomet "));
    assert!(lines[2].contains(&path_tomet.display().to_string()));
    assert!(lines[2].ends_with("(shadowed by builtin)"));
}

#[test]
fn duplicate_names_in_path_show_only_first_by_default() {
    let sb1 = Sandbox::new("list-dup-1");
    let sb2 = Sandbox::new("list-dup-2");

    let path1 = make_file(&sb1.dir, "sisho-desk", true);
    let _path2 = make_file(&sb2.dir, "sisho-desk", true);

    let combined_path = format!("{}:{}", sb1.dir.display(), sb2.dir.display());
    let output = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["--list"])
        .env_clear()
        .env("PATH", &combined_path)
        .output()
        .unwrap();

    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains(&path1.display().to_string()));
    assert!(!lines[0].contains("shadowed"));
}

#[test]
fn duplicate_names_with_all_flag_shows_hidden_duplicates() {
    let sb1 = Sandbox::new("list-all-1");
    let sb2 = Sandbox::new("list-all-2");

    let path1 = make_file(&sb1.dir, "sisho-desk", true);
    let path2 = make_file(&sb2.dir, "sisho-desk", true);

    let combined_path = format!("{}:{}", sb1.dir.display(), sb2.dir.display());
    let output = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["--list", "--all"])
        .env_clear()
        .env("PATH", &combined_path)
        .output()
        .unwrap();

    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);

    assert!(lines[0].starts_with("desk "));
    assert!(lines[0].contains(&path1.display().to_string()));
    assert!(!lines[0].contains("shadowed"));

    assert!(lines[1].starts_with("desk "));
    assert!(lines[1].contains(&path2.display().to_string()));
    let expected_shadow = format!("(shadowed by {})", path1.display());
    assert!(lines[1].contains(&expected_shadow));
}

#[test]
fn short_flag_l_works_identically() {
    let sandbox = Sandbox::new("list-short");
    let ext_path = make_file(&sandbox.dir, "sisho-desk", true);

    let output = sandbox.sisho(&["-l"], "");
    assert_eq!(code(&output), 0);
    assert_eq!(
        stdout(&output).trim(),
        format!("desk      {}", ext_path.display())
    );
}

#[test]
fn all_flag_without_list_is_rejected_as_usage_error() {
    let sandbox = Sandbox::new("list-err");
    let output = sandbox.sisho(&["--all"], "");
    assert_eq!(code(&output), 2);
    assert!(stderr(&output).contains("--list"));
}

#[test]
fn verbose_flag_without_list_is_rejected_as_usage_error() {
    let sandbox = Sandbox::new("list-v-err");
    let output = sandbox.sisho(&["-v"], "");
    assert_eq!(code(&output), 2);
    assert!(stderr(&output).contains("--list"));

    let output2 = sandbox.sisho(&["--verbose"], "");
    assert_eq!(code(&output2), 2);
    assert!(stderr(&output2).contains("--list"));
}

fn make_script(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn list_verbose_displays_extension_metadata() {
    let sandbox = Sandbox::new("list-verbose-metadata");
    let script = r#"#!/bin/sh
if [ "$1" = "--sisho-extension-info" ]; then
    echo '{"schema_version":1,"word":"desk","version":"0.2.0","about":"Desktop interface for Tomet vaults","capabilities":{"tui":true,"interactive":true}}'
    exit 0
fi
echo "desk"
"#;
    let ext_path = make_script(&sandbox.dir, "sisho-desk", script);

    let output = sandbox.sisho(&["--list", "-v"], "");
    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], format!("desk      {}", ext_path.display()));
    assert_eq!(
        lines[1],
        "          v0.2.0  Desktop interface for Tomet vaults  [tui, interactive]"
    );
}

#[test]
fn list_verbose_fallback_when_not_supporting_info() {
    let sandbox = Sandbox::new("list-verbose-fallback");
    let script = "#!/bin/sh\nexit 1\n";
    let ext_path = make_script(&sandbox.dir, "sisho-desk", script);

    let output = sandbox.sisho(&["--list", "--verbose"], "");
    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], format!("desk      {}", ext_path.display()));
}

#[test]
fn list_verbose_times_out_and_discards_output_when_hanging() {
    let sandbox = Sandbox::new("list-verbose-timeout");
    let script = "#!/bin/sh\nwhile :; do :; done\n";
    let ext_path = make_script(&sandbox.dir, "sisho-desk", script);

    let start = std::time::Instant::now();
    let output = sandbox.sisho(&["--list", "-v"], "");
    let elapsed = start.elapsed();

    assert_eq!(code(&output), 0);
    // Timeout is 500ms, so it should finish well under 2 seconds.
    assert!(elapsed < std::time::Duration::from_millis(1500));
    assert!(elapsed >= std::time::Duration::from_millis(450));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], format!("desk      {}", ext_path.display()));
}
