//! `sisho new` against a vault the test builds in a temporary directory, with
//! the real `tomet` libraries behind it.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A vault in a fresh temporary directory, removed when dropped.
struct Vault(PathBuf);

impl Vault {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sisho-app-new-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn with_rfc(name: &str) -> Self {
        let vault = Self::new(name);
        vault.write(
            "default.config.tmt",
            "@kind(config)\n@config{\n  blueprints: list(\".tomet/blueprints/rfc.blueprint.tmt\")\n}\n",
        );
        vault.write(
            ".tomet/blueprints/rfc.blueprint.tmt",
            "@kind(blueprint)\n@blueprint(rfc)\n@meta{\n  title: ${vars.title}\n  author: ${vars.author}\n}\n\n=[ ${vars.title} ]\n",
        );
        vault
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Run `sisho new <args>` with the vault as the working directory.
    fn sisho_new(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sisho"))
            .arg("new")
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Vault {
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

const FULL: &[&str] = &["-b", "rfc", "--var", "title=Hello", "--var", "author=Me"];

fn with(extra: &[&'static str]) -> Vec<&'static str> {
    FULL.iter().chain(extra).copied().collect()
}

#[test]
fn a_document_is_made_from_the_named_blueprint() {
    let vault = Vault::with_rfc("create");
    let output = vault.sisho_new(&with(&["rfcs/1.tmt"]));

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).starts_with("Created "),
        "{}",
        stdout(&output)
    );
    assert!(stdout(&output).trim_end().ends_with("rfcs/1.tmt"));
    let made = fs::read_to_string(vault.0.join("rfcs/1.tmt")).unwrap();
    assert!(made.contains("@kind(rfc)"), "{made}");
    assert!(made.contains("Hello"), "{made}");
    assert!(made.contains("Me"), "{made}");
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
}

#[test]
fn list_shows_the_declared_blueprints_and_creates_nothing() {
    let vault = Vault::with_rfc("list");
    let output = vault.sisho_new(&["--list"]);

    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    assert!(text.starts_with("Declared blueprints:\n"), "{text}");
    assert!(text.contains("  - rfc "), "{text}");
    assert!(!vault.0.join("document.tmt").exists());
}

#[test]
fn a_vault_with_no_blueprints_says_so() {
    let vault = Vault::new("none");
    let output = vault.sisho_new(&["--list"]);

    assert_eq!(code(&output), 0);
    assert!(
        stdout(&output).contains("This vault declares no blueprints."),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_existing_file_is_refused_with_exit_1_and_kept_unless_forced() {
    let vault = Vault::with_rfc("exists");
    assert_eq!(code(&vault.sisho_new(&with(&["rfcs/1.tmt"]))), 0);

    let refused = vault.sisho_new(&with(&["rfcs/1.tmt"]));
    assert_eq!(code(&refused), 1);
    assert!(
        stderr(&refused).starts_with("sisho: "),
        "{}",
        stderr(&refused)
    );
    assert!(
        stderr(&refused).contains("already exists"),
        "{}",
        stderr(&refused)
    );

    let forced = vault.sisho_new(&with(&["--force", "rfcs/1.tmt"]));
    assert_eq!(code(&forced), 0, "{}", stderr(&forced));
}

#[test]
fn an_unknown_blueprint_is_refused_and_names_the_declared_ones() {
    let vault = Vault::with_rfc("unknown");
    let output = vault.sisho_new(&["-b", "nope", "x.tmt"]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("blueprint 'nope' not found. Declared: rfc"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_variable_without_a_value_is_reported_but_the_document_is_still_made() {
    let vault = Vault::with_rfc("unfilled");
    let output = vault.sisho_new(&["-b", "rfc", "--var", "title=T", "rfcs/2.tmt"]);

    assert_eq!(code(&output), 0);
    assert!(vault.0.join("rfcs/2.tmt").is_file());
    let text = stderr(&output);
    assert!(text.contains("rfc.blueprint.tmt:5: warning:"), "{text}");
    assert!(text.contains("1 expression left as written in"), "{text}");
}

#[test]
fn a_declared_blueprint_that_is_missing_is_a_warning_not_a_failure() {
    let vault = Vault::new("rotted");
    vault.write(
        "default.config.tmt",
        "@kind(config)\n@config{\n  blueprints: list(\".tomet/blueprints/gone.blueprint.tmt\")\n}\n",
    );
    let output = vault.sisho_new(&["--list"]);

    assert_eq!(code(&output), 0);
    assert!(
        stderr(&output).starts_with("warning: declared blueprint does not exist"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_variable_that_is_not_key_value_is_a_usage_error() {
    let vault = Vault::with_rfc("bad-var");
    let output = vault.sisho_new(&["--var", "novalue", "x.tmt"]);

    assert_eq!(code(&output), 2);
    assert!(
        stderr(&output).contains("no `=` found"),
        "{}",
        stderr(&output)
    );
}
