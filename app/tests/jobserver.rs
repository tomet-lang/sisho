//! Concurrency and jobserver integration tests for `sisho`.
#![cfg(unix)]

mod common;

use common::{Sandbox, code, stderr, stdout};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn install_concurrency_tracking_tool(dir: &Path, name: &str, shared_dir: &Path) {
    let script = format!(
        r#"#!/bin/sh
export PATH="/run/current-system/sw/bin:/usr/bin:/bin:$PATH"

# Record whether the tool inherited the jobserver authentication flag
case "$CARGO_MAKEFLAGS $MAKEFLAGS" in
  *--jobserver-auth=*)
    echo "auth: {name} $*" >> "{shared}/jobserver_auth"
    ;;
  *)
    echo "no-auth: {name} $*" >> "{shared}/jobserver_auth"
    ;;
esac

case "$1" in
  --version)
    echo "{name} 9.9.9"
    exit 0
    ;;
  *)
    file="{shared}/active_$$"
    : > "$file"
    set -- "{shared}"/active_*
    if [ -f "$1" ]; then
      count=$#
    else
      count=0
    fi
    echo "$count" >> "{shared}/counts"
    # Sleep long enough for concurrent siblings to overlap reliably
    sleep 0.2
    rm -f "$file"
    echo "args: $*"
    exit 0
    ;;
esac
"#,
        shared = shared_dir.display()
    );
    let path = dir.join(name);
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn concurrency_ceiling_holds_for_jobs_flag() {
    for limit in [1, 2, 4] {
        let sandbox = Sandbox::new(&format!("jobs-{limit}"));
        let shared_dir = sandbox.dir.join("shared");
        fs::create_dir_all(&shared_dir).unwrap();

        install_concurrency_tracking_tool(&sandbox.dir, "tomet", &shared_dir);
        install_concurrency_tracking_tool(&sandbox.dir, "twrit", &shared_dir);

        let output = sandbox.sisho(&["-j", &limit.to_string(), "check"], "");
        let text = stdout(&output);
        assert_eq!(code(&output), 0, "{text}");
        assert!(text.contains("all 4 steps passed"), "{text}");

        let counts_content = fs::read_to_string(shared_dir.join("counts")).unwrap_or_default();
        let max_concurrent: usize = counts_content
            .lines()
            .filter_map(|l| l.trim().parse::<usize>().ok())
            .max()
            .unwrap_or(0);

        // Capacity is reached: exactly min(limit, 4) since 4 steps run
        assert_eq!(
            max_concurrent,
            limit.min(4),
            "for -j {limit}, observed max concurrency was {max_concurrent}"
        );

        // Prove children received the jobserver environment
        let auth_content =
            fs::read_to_string(shared_dir.join("jobserver_auth")).unwrap_or_default();
        let auth_steps: Vec<_> = auth_content
            .lines()
            .filter(|l| l.starts_with("auth: ") && !l.contains("--version"))
            .collect();
        // 4 check steps run
        assert_eq!(
            auth_steps.len(),
            4,
            "all 4 child check steps must receive jobserver auth, got: {auth_content}"
        );
        assert!(
            !auth_content.contains("no-auth:"),
            "no step should lack jobserver auth, got: {auth_content}"
        );
    }
}

#[test]
fn inherited_jobserver_is_honored_and_wins_over_jobs_flag() {
    let sandbox = Sandbox::new("inherited");
    let shared_dir = sandbox.dir.join("shared");
    fs::create_dir_all(&shared_dir).unwrap();

    install_concurrency_tracking_tool(&sandbox.dir, "tomet", &shared_dir);
    install_concurrency_tracking_tool(&sandbox.dir, "twrit", &shared_dir);

    // Limit 2 = 1 token in pipe + 1 implicit token
    let client = jobserver::Client::new(1).unwrap();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sisho"));
    cmd.args(["-j", "4", "check"]) // -j 4 is ignored; inherited wins
        .current_dir(&sandbox.dir)
        .env_clear()
        .env("PATH", &sandbox.dir)
        .env("FAKE_FAIL", "");
    client.configure(&mut cmd);

    let output = cmd.output().unwrap();
    let text = stdout(&output);
    assert_eq!(code(&output), 0, "{text}");
    assert!(text.contains("all 4 steps passed"), "{text}");

    let counts_content = fs::read_to_string(shared_dir.join("counts")).unwrap_or_default();
    let max_concurrent: usize = counts_content
        .lines()
        .filter_map(|l| l.trim().parse::<usize>().ok())
        .max()
        .unwrap_or(0);

    assert_eq!(
        max_concurrent, 2,
        "inherited jobserver with limit 2 must constrain concurrency to exactly 2, observed {max_concurrent}"
    );

    // Children received jobserver environment
    let auth_content = fs::read_to_string(shared_dir.join("jobserver_auth")).unwrap_or_default();
    assert!(
        auth_content.lines().all(|l| l.starts_with("auth: ")),
        "all invocations should inherit jobserver: {auth_content}"
    );
}

#[test]
fn passthrough_receives_jobserver_environment() {
    let sandbox = Sandbox::new("passthrough");
    let shared_dir = sandbox.dir.join("shared");
    fs::create_dir_all(&shared_dir).unwrap();

    install_concurrency_tracking_tool(&sandbox.dir, "tomet", &shared_dir);
    install_concurrency_tracking_tool(&sandbox.dir, "twrit", &shared_dir);

    let output = sandbox.sisho(&["tomet", "format", "-i", "."], "");
    assert_eq!(code(&output), 0, "{}", stdout(&output));

    let auth_content = fs::read_to_string(shared_dir.join("jobserver_auth")).unwrap_or_default();
    assert!(
        auth_content.contains("auth: tomet format -i ."),
        "passthrough step via run_inherited must receive jobserver auth, got: {auth_content}"
    );
}

#[test]
fn broken_jobserver_prints_warning_and_falls_back() {
    let sandbox = Sandbox::new("broken").both_tools();
    let output = sandbox.sisho_with(&["check"], "", &[("MAKEFLAGS", "--jobserver-auth=998,999")]);
    let stderr_text = stderr(&output);
    assert_eq!(code(&output), 0, "{stderr_text}");

    let warning_lines: Vec<_> = stderr_text
        .lines()
        .filter(|l| l.starts_with("warning: failed to connect to jobserver"))
        .collect();
    assert_eq!(
        warning_lines.len(),
        1,
        "must print exactly one warning line, got: {stderr_text}"
    );
}

#[test]
fn inherited_jobserver_does_not_leak_tokens() {
    let sandbox = Sandbox::new("leak").both_tools();

    // Client with 1 pipe token
    let client = jobserver::Client::new(1).unwrap();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sisho"));
    cmd.args(["check"])
        .current_dir(&sandbox.dir)
        .env_clear()
        .env("PATH", &sandbox.dir)
        .env("FAKE_FAIL", "");
    client.configure(&mut cmd);

    let output = cmd.output().unwrap();
    assert_eq!(code(&output), 0, "{}", stdout(&output));

    let mut tokens = Vec::new();
    while client.available().unwrap_or(0) > 0 {
        tokens.push(client.acquire().unwrap());
    }
    assert_eq!(
        tokens.len(),
        1,
        "pipe should have exactly 1 token after run, but had {}",
        tokens.len()
    );
}

#[test]
fn j_zero_is_rejected_as_usage_error() {
    let sandbox = Sandbox::new("j-zero").both_tools();
    let output = sandbox.sisho(&["-j", "0", "check"], "");
    assert_eq!(code(&output), 2);
    let stderr_text = stderr(&output);
    assert!(
        stderr_text.contains("number would be zero for non-zero type")
            || stderr_text.contains("invalid value '0'"),
        "{stderr_text}"
    );
}
