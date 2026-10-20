//! Integration tests for bare `sisho` invocation.
#![cfg(unix)]

mod common;

use std::process::Command;

use common::{Sandbox, code, stderr, stdout};

#[test]
fn bare_sisho_without_terminal_prints_help_and_exits_2() {
    let sandbox = Sandbox::new("bare-no-term").both_tools();
    let output = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .current_dir(&sandbox.dir)
        .env_clear()
        .env("PATH", &sandbox.dir)
        .output()
        .unwrap();

    assert_eq!(code(&output), 2);
    let out = format!("{}{}", stdout(&output), stderr(&output));
    assert!(out.contains("Usage: sisho"), "{out}");
    assert!(out.contains("check"), "{out}");
    assert!(out.contains("fix"), "{out}");
    assert!(out.contains("--list"), "{out}");
    assert!(out.contains("Tools: tomet, twrit"), "{out}");
}
