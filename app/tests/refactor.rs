//! `sisho refactor` over documents the test writes into a temporary directory.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sisho-app-refactor-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, rel: &str, text: &str) -> PathBuf {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.0.join(rel)).unwrap()
    }

    fn sisho(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sisho"))
            .arg("refactor")
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

const OLD: &str = "@meta{\n  type: note\n}\n";
const CLEAN: &str = "@kind(note)\n\n=[ Title ]\n";

fn dirty_and_clean(name: &str) -> Dir {
    let dir = Dir::new(name);
    dir.write("old.tmt", OLD);
    dir.write("clean.tmt", CLEAN);
    dir
}

#[test]
fn by_default_it_shows_what_would_change_and_writes_nothing() {
    let dir = dirty_and_clean("dry");
    let output = dir.sisho(&["."]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("==> ./old.tmt (1 changes) <=="), "{text}");
    assert!(text.contains("@kind(note)"), "{text}");
    assert!(
        text.contains("Dry-run: 1 of 2 file(s) would be modified."),
        "{text}"
    );
    assert_eq!(dir.read("old.tmt"), OLD);
}

#[test]
fn a_single_file_prints_its_new_source() {
    let dir = dirty_and_clean("single");
    let output = dir.sisho(&["old.tmt"]);

    assert_eq!(code(&output), 0);
    assert!(
        stdout(&output).contains("@kind(note)"),
        "{}",
        stdout(&output)
    );
    assert!(!stdout(&output).contains("Dry-run"), "{}", stdout(&output));
}

#[test]
fn check_exits_1_and_names_what_needs_changing() {
    let dir = dirty_and_clean("check-dirty");
    let output = dir.sisho(&["--check"]);

    assert_eq!(code(&output), 1);
    assert!(
        stdout(&output).contains("Needs refactor (1 change(s)): ./old.tmt"),
        "{}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains("sisho: 1 of 2 file(s) need refactoring"),
        "{}",
        stderr(&output)
    );
    assert_eq!(dir.read("old.tmt"), OLD, "--check writes nothing");
}

#[test]
fn check_on_a_clean_tree_exits_0() {
    let dir = Dir::new("check-clean");
    dir.write("clean.tmt", CLEAN);
    let output = dir.sisho(&["--check"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(stdout(&output), "All 1 file(s) are up to date.\n");
}

#[test]
fn check_fails_on_a_file_it_could_not_parse() {
    let dir = Dir::new("check-unparseable");
    dir.write("fine.tmt", CLEAN);
    dir.write("broken.tmt", "=[ unterminated\n");
    let output = dir.sisho(&["--check"]);

    assert_eq!(code(&output), 1);
    let text = stderr(&output);
    assert!(text.contains("Could not check ./broken.tmt"), "{text}");
    assert!(
        text.contains("sisho: 1 file(s) could not be checked"),
        "{text}"
    );
}

#[test]
fn in_place_writes_the_changed_documents() {
    let dir = dirty_and_clean("in-place");
    let output = dir.sisho(&["-i", "."]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(stdout(&output), "Refactored 1 of 2 file(s).\n");
    assert!(dir.read("old.tmt").contains("@kind(note)"));
    assert_eq!(
        dir.read("clean.tmt"),
        CLEAN,
        "an unchanged file is left as it was"
    );
}

#[test]
fn check_and_in_place_cannot_be_combined() {
    let dir = dirty_and_clean("conflict");
    let output = dir.sisho(&["--check", "-i"]);

    assert_eq!(code(&output), 2);
    assert_eq!(dir.read("old.tmt"), OLD);
}

#[test]
fn an_explicit_file_the_config_excludes_is_skipped_until_forced() {
    let dir = Dir::new("unswept");
    dir.write(
        "default.config.tmt",
        "@kind(config)\n@config{ workspace: { unswept: list(\"frozen\") } }\n",
    );
    dir.write("frozen/f.tmt", OLD);

    let skipped = dir.sisho(&["-i", "frozen/f.tmt"]);
    assert_eq!(code(&skipped), 0);
    assert!(
        stderr(&skipped).contains("skipped "),
        "{}",
        stderr(&skipped)
    );
    assert_eq!(dir.read("frozen/f.tmt"), OLD, "left alone");

    let forced = dir.sisho(&["-i", "--force", "frozen/f.tmt"]);
    assert_eq!(code(&forced), 0, "{}", stderr(&forced));
    assert!(
        dir.read("frozen/f.tmt").contains("@kind(note)"),
        "--force reaches it"
    );
}
