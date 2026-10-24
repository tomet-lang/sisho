//! `sisho convert`, against a fake `tomet`: which command it runs, with what.
//!
//! The converting is `tomet`'s and is tested there. What `sisho` owns is the
//! verb: naming the formats, refusing what means nothing, handing the terminal
//! over, and taking on the exit code.
#![cfg(unix)]

mod common;

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

use common::{Sandbox, code, stderr, stdout};

/// Replace the fake `tomet` with a script of the test's own.
fn tomet_script(sandbox: &Sandbox, body: &str) {
    let path = sandbox.dir.join("tomet");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn each_target_is_the_tomet_command_that_converts_to_it() {
    let sandbox = Sandbox::new("convert-targets").tool("tomet");
    for (format, command) in [
        ("md", "to-md"),
        ("commonmark", "to-md"),
        ("MD", "to-md"),
        ("html", "html"),
        ("typst", "to-typst"),
        ("pandoc", "to-pandoc"),
    ] {
        let output = sandbox.sisho(&["convert", "doc.tmt", "--to", format], "");
        assert_eq!(code(&output), 0, "{format}: {}", stderr(&output));
        assert_eq!(stdout(&output), format!("args: {command} doc.tmt\n"));
    }
}

#[test]
fn each_source_is_the_tomet_command_that_converts_from_it() {
    let sandbox = Sandbox::new("convert-sources").tool("tomet");
    for (format, command) in [
        ("md", "from-md"),
        ("commonmark", "from-md"),
        ("pandoc", "from-pandoc"),
    ] {
        let output = sandbox.sisho(&["convert", "in", "--from", format], "");
        assert_eq!(code(&output), 0, "{format}: {}", stderr(&output));
        assert_eq!(stdout(&output), format!("args: {command} in\n"));
    }
}

#[test]
fn where_the_result_goes_is_left_to_tomet() {
    let sandbox = Sandbox::new("convert-destination").tool("tomet");
    let cases: &[(&[&str], &str)] = &[
        (
            &["doc.tmt", "--to", "md", "-o", "out"],
            "to-md --out out doc.tmt",
        ),
        (
            &["src", "--to", "typst", "--in-place", "--dry-run"],
            "to-typst --in-place --dry-run src",
        ),
        (
            &["src", "--from", "md", "-i", "--remove-original"],
            "from-md --in-place --remove-original src",
        ),
        (
            &["src", "--from", "md", "--out", "o", "--remove-original"],
            "from-md --out o --remove-original src",
        ),
    ];
    for (args, expected) in cases {
        let mut full = vec!["convert"];
        full.extend_from_slice(args);
        let output = sandbox.sisho(&full, "");
        assert_eq!(code(&output), 0, "{args:?}: {}", stderr(&output));
        assert_eq!(stdout(&output), format!("args: {expected}\n"), "{args:?}");
    }
}

#[test]
fn html_takes_its_options() {
    let sandbox = Sandbox::new("convert-html").tool("tomet");
    let output = sandbox.sisho(
        &[
            "convert",
            "doc.tmt",
            "--to",
            "html",
            "--advanced",
            "--lang",
            "en",
            "--body",
        ],
        "",
    );
    assert_eq!(
        stdout(&output),
        "args: html --advanced --body --lang en doc.tmt\n"
    );
}

#[test]
fn a_file_that_starts_with_a_dash_is_a_file() {
    let sandbox = Sandbox::new("convert-dash").tool("tomet");
    let output = sandbox.sisho(&["convert", "--to", "md", "--", "-x.tmt"], "");
    assert_eq!(stdout(&output), "args: to-md -- -x.tmt\n");
}

/// `pandoc -t json x.docx | sisho convert - --from pandoc | ...`: the pipes are
/// `tomet`'s own, not read and written again by `sisho`.
#[test]
fn standard_input_and_output_are_tomets_own() {
    let sandbox = Sandbox::new("convert-stdin");
    tomet_script(&sandbox, "read line; echo \"$1 read: $line\"");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["convert", "-", "--from", "pandoc"])
        .env_clear()
        .env("PATH", &sandbox.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"hello\n").unwrap();
    let output = child.wait_with_output().unwrap();

    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "from-pandoc read: hello\n");
}

#[test]
fn what_tomet_says_and_the_code_it_exits_with_are_sishos() {
    let sandbox = Sandbox::new("convert-failure").tool("tomet");
    let output = sandbox.sisho(&["convert", "doc.tmt", "--to", "md"], "tomet:to-md");
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("boom from tomet to-md doc.tmt"),
        "{}",
        stderr(&output)
    );

    tomet_script(&sandbox, "exit 3");
    let output = sandbox.sisho(&["convert", "doc.tmt", "--to", "md"], "");
    assert_eq!(code(&output), 3);
}

#[test]
fn a_tomet_that_is_not_installed_is_said_so() {
    let sandbox = Sandbox::new("convert-no-tomet");
    let output = sandbox.sisho(&["convert", "doc.tmt", "--to", "md"], "");
    assert_eq!(code(&output), 127);
    assert!(
        stderr(&output).contains("tomet is not on PATH"),
        "{}",
        stderr(&output)
    );
}

/// Refused before `tomet` is started: with no `tomet` on `PATH`, a command that
/// had been started would exit 127.
#[test]
fn a_command_line_that_means_nothing_is_a_usage_error() {
    let sandbox = Sandbox::new("convert-usage");
    let cases: &[&[&str]] = &[
        &["doc.tmt"],                                          // neither --to nor --from
        &["doc.tmt", "--to", "md", "--from", "md"],            // both
        &["doc.tmt", "--to", "docx"],                          // unknown format
        &["note.md", "--from", "html"],                        // unknown source
        &["doc.tmt", "--to", "md", "--lang", "en"],            // an html option
        &["note.md", "--from", "md", "--body"],                // an html option
        &["doc.tmt", "--to", "md", "--remove-original", "-i"], // would delete Tomet
        &["note.md", "--from", "md", "--remove-original"],     // nothing replaces it
        &["note.md", "--from", "md", "-i", "-o", "x.tmt"],     // two destinations
    ];
    for args in cases {
        let mut full = vec!["convert"];
        full.extend_from_slice(args);
        let output = sandbox.sisho(&full, "");
        assert_eq!(code(&output), 2, "{args:?}: {}", stderr(&output));
    }
}

#[test]
fn the_unknown_format_message_lists_the_known_ones() {
    let sandbox = Sandbox::new("convert-unknown-format");
    let output = sandbox.sisho(&["convert", "doc.tmt", "--to", "docx"], "");

    assert!(
        stderr(&output).contains("unknown format 'docx' for --to"),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("commonmark (md), html, typst, pandoc"),
        "{}",
        stderr(&output)
    );
}
