//! `sisho export` over documents the test writes into a temporary directory.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sisho-app-export-{}-{name}", std::process::id()));
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

    fn sisho(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sisho"))
            .arg("export")
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
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

const README: &str =
    "@config(export: { type: commonmark, path: \"out/readme.md\" })\n\n=[ Readme ]\n\nHello\n";

fn named_vault(name: &str) -> Dir {
    let dir = Dir::new(name);
    dir.write("default.config.tmt", "@kind(config)\n");
    dir.write("docs/readme.tmt", README);
    dir
}

#[test]
fn a_directory_is_exported_to_the_paths_its_documents_declare() {
    let dir = named_vault("dir");
    let output = dir.sisho(&["."]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("readme.tmt -> "), "{text}");
    assert!(text.contains("out/readme.md"), "{text}");
    assert!(
        text.contains("Batch export finished: 2 file(s) processed."),
        "{text}"
    );
    assert!(dir.read("out/readme.md").contains("# Readme"));
}

#[test]
fn check_passes_on_fresh_outputs_and_fails_on_a_hand_edited_one() {
    let dir = named_vault("check");
    assert_eq!(code(&dir.sisho(&["."])), 0);

    let fresh = dir.sisho(&[".", "--check"]);
    assert_eq!(code(&fresh), 0, "{}", stderr(&fresh));
    assert_eq!(stdout(&fresh), "All exports are up to date.\n");

    dir.write("out/readme.md", "# Hand edited\n");
    let stale = dir.sisho(&[".", "--check"]);
    assert_eq!(code(&stale), 1);
    assert!(
        stdout(&stale).contains("Stale export: "),
        "{}",
        stdout(&stale)
    );
    assert!(
        stderr(&stale)
            .contains("sisho: 1 export(s) differ from their source; re-run without --check"),
        "{}",
        stderr(&stale)
    );
    assert_eq!(
        dir.read("out/readme.md"),
        "# Hand edited\n",
        "--check writes nothing"
    );
}

#[test]
fn a_missing_output_fails_the_check() {
    let dir = named_vault("missing");
    let output = dir.sisho(&[".", "--check"]);

    assert_eq!(code(&output), 1);
    assert!(
        stdout(&output).contains("Stale export: "),
        "{}",
        stdout(&output)
    );
    assert!(!dir.0.join("out/readme.md").exists());
}

#[test]
fn a_single_file_with_no_destination_prints_to_standard_output() {
    let dir = named_vault("print");
    dir.write("docs/loose.tmt", "=[ Loose ]\n");
    let output = dir.sisho(&["docs/loose.tmt"]);

    assert_eq!(code(&output), 0);
    assert!(stdout(&output).contains("# Loose"), "{}", stdout(&output));
}

#[test]
fn one_broken_document_fails_the_run_but_not_the_others() {
    let dir = named_vault("broken");
    dir.write("docs/broken.tmt", "=[ unterminated\n");
    let output = dir.sisho(&["."]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("Error exporting "),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("sisho: 1 of 3 file(s) could not be exported"),
        "{}",
        stderr(&output)
    );
    assert!(
        dir.0.join("out/readme.md").is_file(),
        "the others are still exported"
    );
}

#[test]
fn a_document_that_cannot_be_read_fails_the_check_too() {
    let dir = named_vault("check-broken");
    dir.write("docs/broken.tmt", "=[ unterminated\n");
    assert_eq!(code(&dir.sisho(&["docs/readme.tmt"])), 0);

    let output = dir.sisho(&[".", "--check"]);
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("could not be checked"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_single_broken_document_names_the_line_and_column() {
    let dir = named_vault("single-broken");
    dir.write("docs/broken.tmt", "=[ unterminated\n");
    let output = dir.sisho(&["docs/broken.tmt"]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("--> docs/broken.tmt:"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn type_all_with_an_output_directory_writes_every_format() {
    let dir = named_vault("all");
    dir.write("docs/loose.tmt", "=[ Loose ]\n");
    let output = dir.sisho(&["docs/loose.tmt", "-t", "all", "-o", "alt"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(dir.0.join("alt/loose.md").is_file());
    assert!(dir.0.join("alt/loose.html").is_file());
}

#[test]
fn a_path_that_does_not_exist_exits_1() {
    let dir = named_vault("absent");
    let output = dir.sisho(&["nope"]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("sisho: path 'nope' does not exist"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_directory_with_no_documents_says_so() {
    let dir = Dir::new("none");
    let output = dir.sisho(&["."]);

    assert_eq!(code(&output), 0);
    assert!(
        stdout(&output).starts_with("No .tmt or .tmt files found in"),
        "{}",
        stdout(&output)
    );
}
