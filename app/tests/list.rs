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
    assert_eq!(
        out_str.trim(),
        format!("desk      {} [untrusted]", ext_path.display())
    );
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
        .env("SISHO_TRUST_FILE", sb1.trust_file())
        .output()
        .unwrap();

    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    // Alphabetical sort: alpha before beta
    assert!(lines[0].starts_with("alpha "));
    assert!(lines[0].contains(&path_alpha.display().to_string()));
    assert!(lines[0].ends_with("[untrusted]"));
    assert!(lines[1].starts_with("beta "));
    assert!(lines[1].contains(&path_beta.display().to_string()));
    assert!(lines[1].ends_with("[untrusted]"));
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
        .env("SISHO_TRUST_FILE", sb1.trust_file())
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
        .env("SISHO_TRUST_FILE", sb1.trust_file())
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
        format!("desk      {} [untrusted]", ext_path.display())
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
fn list_verbose_displays_extension_metadata_when_allowed() {
    let sandbox = Sandbox::new("list-verbose-metadata");
    let script = r#"#!/bin/sh
if [ "$1" = "--sisho-extension-info" ]; then
    echo '{"schema_version":1,"word":"desk","version":"0.2.0","about":"Desktop interface for Tomet vaults","capabilities":{"tui":true,"interactive":true}}'
    exit 0
fi
echo "desk"
"#;
    let ext_path = make_script(&sandbox.dir, "sisho-desk", script);
    let sandbox = sandbox.trust_extension("desk");

    let output = sandbox.sisho(&["--list", "-v"], "");
    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        format!("desk      {} [allowed]", ext_path.display())
    );
    assert_eq!(
        lines[1],
        "          v0.2.0  Desktop interface for Tomet vaults  [tui, interactive]"
    );
}

#[test]
fn list_verbose_never_executes_untrusted_or_denied_extensions() {
    let sandbox = Sandbox::new("list-verbose-gate");
    let marker = sandbox.dir.join("info-executed");
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"--sisho-extension-info\" ]; then\n\
             : > \"{}\"\n\
             echo '{{\"schema_version\":1,\"word\":\"desk\",\"version\":\"0.2.0\",\"about\":\"Desktop interface\",\"capabilities\":{{\"tui\":true}}}}'\n\
             exit 0\n\
         fi\n",
        marker.display()
    );
    let ext_path = make_script(&sandbox.dir, "sisho-desk", &script);

    // 1. Untrusted: `sisho --list -v` must NOT run `--sisho-extension-info`
    let out_untrusted = sandbox.sisho(&["--list", "-v"], "");
    assert_eq!(code(&out_untrusted), 0);
    assert!(
        !marker.exists(),
        "--sisho-extension-info must never run on an untrusted extension"
    );
    assert_eq!(
        stdout(&out_untrusted).trim(),
        format!("desk      {} [untrusted]", ext_path.display())
    );

    // 2. Denied: `sisho --list -v` must STILL NOT run `--sisho-extension-info`
    assert_eq!(code(&sandbox.sisho(&["--deny", "desk"], "")), 0);
    let out_denied = sandbox.sisho(&["--list", "-v"], "");
    assert_eq!(code(&out_denied), 0);
    assert!(
        !marker.exists(),
        "--sisho-extension-info must never run on a denied extension"
    );
    assert_eq!(
        stdout(&out_denied).trim(),
        format!("desk      {} [denied]", ext_path.display())
    );

    // 3. Allowed: `sisho --list -v` now queries `--sisho-extension-info`
    assert_eq!(code(&sandbox.sisho(&["--allow", "desk"], "")), 0);
    let out_allowed = sandbox.sisho(&["--list", "-v"], "");
    assert_eq!(code(&out_allowed), 0);
    assert!(
        marker.exists(),
        "--sisho-extension-info must run once extension is allowed"
    );
    let text_allowed = stdout(&out_allowed);
    let lines: Vec<&str> = text_allowed.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        format!("desk      {} [allowed]", ext_path.display())
    );
}

#[test]
fn list_json_outputs_structured_entries_with_canonical_path_and_trust() {
    let sandbox = Sandbox::new("list-json");
    let script = r#"#!/bin/sh
if [ "$1" = "--sisho-extension-info" ]; then
    echo '{"schema_version":1,"word":"desk","version":"0.2.0","about":"Desktop interface","capabilities":{"tui":true}}'
    exit 0
fi
"#;
    let ext_path = make_script(&sandbox.dir, "sisho-desk", script);
    let canonical = std::fs::canonicalize(&ext_path).unwrap();

    // Untrusted JSON with -v: info must be null
    let out_untrusted = sandbox.sisho(&["--list", "-v", "--json"], "");
    assert_eq!(code(&out_untrusted), 0);
    let parsed_untrusted: serde_json::Value =
        serde_json::from_str(&stdout(&out_untrusted)).unwrap();
    let arr = parsed_untrusted.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["word"], "desk");
    assert_eq!(arr[0]["path"], ext_path.to_str().unwrap());
    assert_eq!(arr[0]["canonical_path"], canonical.to_str().unwrap());
    assert_eq!(arr[0]["status"], "active");
    assert!(arr[0]["shadowed_by"].is_null());
    assert_eq!(arr[0]["trust"], "untrusted");
    assert!(arr[0]["info"].is_null());

    // After allowing: info is populated when -v is passed
    assert_eq!(code(&sandbox.sisho(&["--allow", "desk"], "")), 0);
    let out_allowed = sandbox.sisho(&["--list", "-v", "--json"], "");
    assert_eq!(code(&out_allowed), 0);
    let parsed_allowed: serde_json::Value = serde_json::from_str(&stdout(&out_allowed)).unwrap();
    let arr_allowed = parsed_allowed.as_array().unwrap();
    assert_eq!(arr_allowed[0]["trust"], "allowed");
    assert_eq!(arr_allowed[0]["info"]["version"], "0.2.0");
    assert_eq!(arr_allowed[0]["info"]["about"], "Desktop interface");
}

#[test]
fn list_verbose_fallback_when_not_supporting_info() {
    let sandbox = Sandbox::new("list-verbose-fallback");
    let script = "#!/bin/sh\nexit 1\n";
    let ext_path = make_script(&sandbox.dir, "sisho-desk", script);
    let sandbox = sandbox.trust_extension("desk");

    let output = sandbox.sisho(&["--list", "--verbose"], "");
    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(
        lines[0],
        format!("desk      {} [allowed]", ext_path.display())
    );
}

#[test]
fn list_verbose_times_out_and_discards_output_when_hanging() {
    let sandbox = Sandbox::new("list-verbose-timeout");
    let script = "#!/bin/sh\nwhile :; do :; done\n";
    let ext_path = make_script(&sandbox.dir, "sisho-desk", script);
    let sandbox = sandbox.trust_extension("desk");

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
    assert_eq!(
        lines[0],
        format!("desk      {} [allowed]", ext_path.display())
    );
}

#[test]
fn list_verbose_passes_sisho_config_and_data_dirs_to_extension_info() {
    let sandbox = Sandbox::new("list-verbose-dirs");
    let captured = sandbox.dir.join("captured-dirs");
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"--sisho-extension-info\" ]; then\n\
             printf '%s\\n%s\\n' \"$SISHO_CONFIG_DIR\" \"$SISHO_DATA_DIR\" > \"{}\"\n\
             echo '{{\"schema_version\":1,\"word\":\"desk\",\"version\":\"0.2.0\",\"about\":\"Desktop interface\"}}'\n\
             exit 0\n\
         fi\n",
        captured.display()
    );
    make_script(&sandbox.dir, "sisho-desk", &script);
    let sandbox = sandbox.trust_extension("desk");

    let xdg_cfg = sandbox.dir.join("xdg-cfg");
    let xdg_data = sandbox.dir.join("xdg-data");
    let output = sandbox.sisho_with(
        &["--list", "-v"],
        "",
        &[
            ("XDG_CONFIG_HOME", xdg_cfg.to_str().unwrap()),
            ("XDG_DATA_HOME", xdg_data.to_str().unwrap()),
        ],
    );
    assert_eq!(code(&output), 0);
    let recorded = fs::read_to_string(&captured).unwrap();
    let lines: Vec<&str> = recorded.lines().collect();
    assert_eq!(
        lines,
        vec![
            xdg_cfg.join("sisho/desk").to_str().unwrap(),
            xdg_data.join("sisho/desk").to_str().unwrap(),
        ]
    );
}
