//! `sisho convert` over documents the test writes into a temporary directory.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sisho-app-convert-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.0.join(rel)).unwrap()
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sisho"));
        command.arg("convert").args(args).current_dir(&self.0);
        command
    }

    fn sisho(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    /// Run with `input` on standard input.
    fn sisho_with_input(&self, args: &[&str], input: &str) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap()
}

const TMT: &str = "=[ Hello Convert ]\n\nA paragraph.\n";
const MD: &str = "# Hello Convert\n\nA paragraph.\n";

fn dir_with_documents(name: &str) -> Dir {
    let dir = Dir::new(name);
    dir.write("doc.tmt", TMT);
    dir.write("note.md", MD);
    dir
}

#[test]
fn a_document_is_written_to_standard_output_as_exactly_the_file_would_be() {
    let dir = dir_with_documents("same-bytes");

    let printed = dir.sisho(&["doc.tmt", "--to", "md"]);
    assert_eq!(code(&printed), 0, "{}", stderr(&printed));
    let text = stdout(&printed);
    assert!(text.contains("# Hello Convert"), "{text}");

    let written = dir.sisho(&["doc.tmt", "--to", "md", "-o", "doc.md"]);
    assert_eq!(code(&written), 0);
    assert_eq!(
        dir.read("doc.md"),
        text,
        "no newline is added on the way to the terminal"
    );
    assert!(
        stdout(&written).starts_with("Converted doc.tmt -> doc.md"),
        "{}",
        stdout(&written)
    );
}

#[test]
fn every_target_is_available() {
    let dir = dir_with_documents("targets");
    for (name, needle) in [
        ("commonmark", "# Hello Convert"),
        ("md", "# Hello Convert"),
        ("html", "<html"),
        ("typst", "Hello Convert"),
        ("pandoc", "pandoc-api-version"),
    ] {
        let output = dir.sisho(&["doc.tmt", "--to", name]);
        assert_eq!(code(&output), 0, "--to {name}: {}", stderr(&output));
        assert!(
            stdout(&output).contains(needle),
            "--to {name}: {}",
            stdout(&output)
        );
    }
}

#[test]
fn html_takes_its_options() {
    let dir = dir_with_documents("html");

    let page = stdout(&dir.sisho(&["doc.tmt", "--to", "html", "--lang", "en"]));
    assert!(page.contains("lang=\"en\""), "{page}");

    let body = stdout(&dir.sisho(&["doc.tmt", "--to", "html", "--body"]));
    assert!(!body.contains("<html"), "{body}");
    assert!(body.contains("Hello Convert"), "{body}");
}

#[test]
fn the_pandoc_bridge_works_through_standard_input_and_output() {
    let dir = dir_with_documents("bridge");

    let json = stdout(&dir.sisho(&["doc.tmt", "--to", "pandoc"]));
    let back = dir.sisho_with_input(&["-", "--from", "pandoc"], &json);

    assert_eq!(code(&back), 0, "{}", stderr(&back));
    assert!(stdout(&back).contains("Hello Convert"), "{}", stdout(&back));
    assert!(stdout(&back).contains("A paragraph."), "{}", stdout(&back));
}

#[test]
fn markdown_can_be_read_from_standard_input_too() {
    let dir = Dir::new("stdin-md");
    let output = dir.sisho_with_input(&["-", "--from", "md"], MD);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).contains("Hello Convert"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn markdown_becomes_tomet_in_place_and_the_original_can_be_removed() {
    let dir = dir_with_documents("migrate");

    let kept = dir.sisho(&["note.md", "--from", "md", "--in-place"]);
    assert_eq!(code(&kept), 0, "{}", stderr(&kept));
    assert!(dir.0.join("note.tmt").is_file());
    assert!(
        dir.0.join("note.md").exists(),
        "the original stays by default"
    );

    let moved = dir.sisho(&["note.md", "--from", "md", "--in-place", "--remove-original"]);
    assert_eq!(code(&moved), 0);
    assert!(!dir.0.join("note.md").exists());
}

#[test]
fn a_dry_run_checks_and_writes_nothing() {
    let dir = dir_with_documents("dry");
    let output = dir.sisho(&["note.md", "--from", "md", "--dry-run"]);

    assert_eq!(code(&output), 0);
    assert_eq!(stdout(&output), "OK (dry-run): note.md\n");
    assert!(!dir.0.join("note.tmt").exists());
}

#[test]
fn a_directory_is_converted_into_files_and_summarised() {
    let dir = Dir::new("tree");
    dir.write("md/one.md", MD);
    dir.write("md/sub/two.md", MD);

    let output = dir.sisho(&["md", "--from", "md", "--out", "out"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "Conversion finished: 2 total, 2 succeeded, 0 failed\n"
    );
    assert!(dir.0.join("out/one.tmt").is_file());
    assert!(dir.0.join("out/sub/two.tmt").is_file());
}

#[test]
fn a_directory_of_tomet_is_converted_to_a_format_too() {
    let dir = Dir::new("tree-to");
    dir.write("docs/a.tmt", TMT);
    dir.write("docs/sub/b.tmt", TMT);

    let output = dir.sisho(&["docs", "--to", "html", "--out", "site"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(dir.0.join("site/a.html").is_file());
    assert!(dir.0.join("site/sub/b.html").is_file());
}

#[test]
fn a_directory_needs_somewhere_to_go() {
    let dir = Dir::new("tree-nowhere");
    dir.write("md/one.md", MD);

    let output = dir.sisho(&["md", "--from", "md"]);
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("pass --out <DIR> or --in-place"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn one_file_that_fails_fails_the_run_and_the_others_are_still_converted() {
    let dir = Dir::new("tree-failure");
    dir.write("docs/good.tmt", TMT);
    dir.write("docs/broken.tmt", "=[ unterminated\n");

    let output = dir.sisho(&["docs", "--to", "md", "--out", "out"]);
    assert_eq!(code(&output), 1);
    assert!(
        stdout(&output).contains("2 total, 1 succeeded, 1 failed"),
        "{}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains("Errors encountered (1):"),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("sisho: 1 file(s) failed conversion"),
        "{}",
        stderr(&output)
    );
    assert!(dir.0.join("out/good.md").is_file());
}

#[test]
fn a_path_that_does_not_exist_is_named() {
    let dir = Dir::new("absent");
    let output = dir.sisho(&["nope.tmt", "--to", "md"]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("sisho: path 'nope.tmt' does not exist"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_parse_error_names_the_line_and_column() {
    let dir = Dir::new("parse-error");
    dir.write("broken.tmt", "=[ unterminated\n");
    let output = dir.sisho(&["broken.tmt", "--to", "md"]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("--> broken.tmt:"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_command_line_that_means_nothing_is_a_usage_error() {
    let dir = dir_with_documents("usage");
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
        let output = dir.sisho(args);
        assert_eq!(code(&output), 2, "{args:?}: {}", stderr(&output));
    }
    assert!(dir.0.join("note.md").exists());
    assert!(dir.read("doc.tmt") == TMT);
}

#[test]
fn the_unknown_format_message_lists_the_known_ones() {
    let dir = dir_with_documents("unknown-format");
    let output = dir.sisho(&["doc.tmt", "--to", "docx"]);

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

#[test]
fn standard_input_cannot_be_converted_in_place() {
    let dir = Dir::new("stdin-in-place");
    let output = dir.sisho_with_input(&["-", "--from", "md", "-i"], MD);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("standard input is not a file"),
        "{}",
        stderr(&output)
    );
}
