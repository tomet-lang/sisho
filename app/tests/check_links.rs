//! `sisho check-links` over documents the test writes into a temporary
//! directory. The link cache goes to a temporary directory too, by pointing
//! the cache lookup there, so the test never touches the user's own.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

struct Tree {
    base: PathBuf,
    vault: PathBuf,
}

impl Tree {
    fn new(name: &str) -> Self {
        let base =
            std::env::temp_dir().join(format!("sisho-app-links-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let vault = base.join("vault");
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(base.join("cache")).unwrap();
        Self { base, vault }
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.vault.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn sisho(&self, args: &[&str]) -> Output {
        let cache = self.base.join("cache");
        Command::new(env!("CARGO_BIN_EXE_sisho"))
            .arg("check-links")
            .args(args)
            .current_dir(&self.vault)
            .env("XDG_CACHE_HOME", &cache)
            .env("HOME", &cache)
            .output()
            .unwrap()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
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

const GOOD: &str = "@meta{}\n\n@link(real.txt)\n[ fine ]\n";
const BAD: &str = "@meta{}\n\n@link(real.txt)\n[ fine ]\n\n@link(missing.txt)\n[ nowhere ]\n";

#[test]
fn a_vault_with_no_broken_link_exits_0() {
    let tree = Tree::new("clean");
    tree.write("real.txt", "x");
    tree.write("a.tmt", GOOD);

    let output = tree.sisho(&["."]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "1 file(s) scanned, 1 link(s) checked, 0 broken\n"
    );
}

#[test]
fn a_broken_link_exits_1_and_says_where() {
    let tree = Tree::new("broken");
    tree.write("real.txt", "x");
    tree.write("a.tmt", BAD);

    let output = tree.sisho(&["."]);
    assert_eq!(code(&output), 1);
    let text = stdout(&output);
    assert!(
        text.starts_with("1 file(s) scanned, 2 link(s) checked, 1 broken\n"),
        "{text}"
    );
    assert!(
        text.contains("a.tmt:6:1: broken link -> missing.txt"),
        "{text}"
    );
}

#[test]
fn a_document_that_cannot_be_read_also_exits_1() {
    let tree = Tree::new("unreadable");
    tree.write("real.txt", "x");
    tree.write("a.tmt", GOOD);
    tree.write("broken.tmt", "@broken(((");

    let output = tree.sisho(&["."]);
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("broken.tmt"),
        "{}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("0 broken"), "{}", stdout(&output));
}

#[test]
fn json_has_the_counts_the_broken_links_and_the_unreadable_documents() {
    let tree = Tree::new("json");
    tree.write("real.txt", "x");
    tree.write("a.tmt", BAD);
    tree.write("broken.tmt", "@broken(((");

    let output = tree.sisho(&[".", "--json"]);
    assert_eq!(code(&output), 1);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    // A document that could not be read was still looked at, so it is counted.
    assert_eq!(json["files_scanned"], 2);
    assert_eq!(json["links_checked"], 2);
    assert_eq!(json["broken"][0]["target"], "missing.txt");
    assert_eq!(json["broken"][0]["line"], 6);
    assert_eq!(json["broken"][0]["column"], 1);
    assert!(
        json["errors"][0]["source"]
            .as_str()
            .unwrap()
            .ends_with("broken.tmt")
    );
}

#[test]
fn a_path_that_does_not_exist_exits_1_with_a_message() {
    let tree = Tree::new("absent");
    let output = tree.sisho(&["nope"]);
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).starts_with("sisho: path 'nope' does not exist"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn the_default_path_is_the_working_directory() {
    let tree = Tree::new("default");
    tree.write("real.txt", "x");
    tree.write("a.tmt", GOOD);

    let output = tree.sisho(&[]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).starts_with("1 file(s) scanned"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn the_cache_goes_to_the_cache_directory_not_into_the_vault() {
    let tree = Tree::new("cache-place");
    tree.write("real.txt", "x");
    tree.write("a.tmt", GOOD);

    assert_eq!(code(&tree.sisho(&["."])), 0);
    let in_vault: Vec<_> = fs::read_dir(&tree.vault)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        in_vault.iter().all(|name| !name.contains("sqlite")),
        "{in_vault:?}"
    );
    assert!(
        fs::read_dir(tree.base.join("cache"))
            .unwrap()
            .next()
            .is_some()
    );
}
