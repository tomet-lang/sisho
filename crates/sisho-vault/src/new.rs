//! `sisho new`: make a document from a blueprint.
//!
//! A blueprint is declared in the vault's config (`blueprints`) and names its
//! target with `@blueprint(<name>)`. This module finds the vault, lists the
//! blueprints it declares, and writes a new document from one of them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tomet_ast::Value;
use tomet_config::PrinterConfig;
use tomet_load_config::find_config_file;
use tomet_workspace::{create_file_from_blueprint, declaration_errors, list_blueprints};

/// The vault `new` works in: where it is, and what its config declares.
pub struct Context {
    root: PathBuf,
    config: PrinterConfig,
    warnings: Vec<String>,
}

/// A blueprint the vault declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blueprint {
    /// The name given in `@blueprint(<name>)`.
    pub name: String,
    /// The blueprint file.
    pub path: PathBuf,
}

/// A document to make.
pub struct Document<'a> {
    /// Where to write it, relative to the vault.
    pub path: &'a Path,
    /// The blueprint to use. Without one, the name of the directory the
    /// document goes in, or `default` if it goes in none.
    pub blueprint: Option<&'a str>,
    /// Overwrite the file if it exists.
    pub force: bool,
    /// Values for `${vars.<key>}` in the blueprint.
    pub vars: &'a [(String, String)],
}

/// A document that was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    /// Where the new document is.
    pub path: PathBuf,
    /// The blueprint file it was made from, which [`Unfilled::line`] counts in.
    pub blueprint: PathBuf,
    /// Expressions the blueprint has no value for. They are in the new
    /// document as written.
    pub unfilled: Vec<Unfilled>,
}

/// An expression in a blueprint that had no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unfilled {
    /// The 1-based line it is written on in the blueprint file.
    pub line: usize,
    /// One line to show, in the form `tomet check` uses for the same thing.
    pub message: String,
}

impl Context {
    /// The vault that governs `start`: the one whose config is found from
    /// `start` upward, or `start` itself with the default config if there is
    /// none.
    pub fn discover(start: &Path) -> Self {
        let (config, root) = match find_config_file(start) {
            Some((config, _, root)) => (config, root),
            None => (PrinterConfig::default(), start.to_path_buf()),
        };
        // A declared path that rotted is reported once, wherever the command
        // was going. Finding out at the document that wanted the blueprint
        // would name the wrong file.
        let warnings = declaration_errors(&root, &config);
        Self {
            root,
            config,
            warnings,
        }
    }

    /// Problems with how the vault declares its blueprints.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// The blueprints the vault declares, by name.
    pub fn blueprints(&self) -> Vec<Blueprint> {
        list_blueprints(&self.root, &self.config)
            .into_iter()
            .map(|(name, path)| Blueprint { name, path })
            .collect()
    }

    /// Write `document` from its blueprint.
    pub fn create(&self, document: &Document<'_>) -> Result<Created> {
        let name = match document.blueprint {
            Some(name) => name.to_string(),
            None => document
                .path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("default")
                .to_string(),
        };

        let vars: HashMap<String, Value> = document
            .vars
            .iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect();

        let created = create_file_from_blueprint(
            &self.root,
            &name,
            document.path,
            &vars,
            &self.config,
            document.force,
        )?;

        Ok(Created {
            path: created.path,
            blueprint: created.blueprint,
            unfilled: created
                .unfilled
                .into_iter()
                .map(|u| Unfilled {
                    line: u.line,
                    message: u.message,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A vault in a fresh temporary directory, removed when dropped.
    struct TempVault(PathBuf);

    impl TempVault {
        fn new(name: &str) -> Self {
            // Unique per run, so two test runs at once do not share a
            // directory. The time, not the process id: `process-door` keeps
            // the whole `process` module out of every crate but `sisho-run`.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("sisho-vault-{nanos}-{name}"));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, rel: &str, text: &str) {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
    }

    impl Drop for TempVault {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const CONFIG: &str =
        "@kind(config)\n@config{\n  blueprints: list(\".tomet/blueprints/rfc.blueprint.tmt\")\n}\n";
    const RFC: &str = "@kind(blueprint)\n@blueprint(rfc)\n@meta{\n  id: ${generate.uuid()}\n  title: ${vars.title}\n  author: ${vars.author}\n}\n\n=[ Motivation for ${vars.title} ]#(motivation)\n";

    fn vault_with_rfc(name: &str) -> TempVault {
        let vault = TempVault::new(name);
        vault.write("default.config.tmt", CONFIG);
        vault.write(".tomet/blueprints/rfc.blueprint.tmt", RFC);
        vault
    }

    fn vars() -> Vec<(String, String)> {
        vec![
            ("title".into(), "New Auth Protocol".into()),
            ("author".into(), "Bob".into()),
        ]
    }

    #[test]
    fn a_document_is_made_from_the_named_blueprint() {
        let vault = vault_with_rfc("named");
        let context = Context::discover(&vault.0);
        let vars = vars();

        let created = context
            .create(&Document {
                path: Path::new("rfcs/001.tmt"),
                blueprint: Some("rfc"),
                force: false,
                vars: &vars,
            })
            .unwrap();

        assert!(created.path.is_file());
        let content = fs::read_to_string(&created.path).unwrap();
        assert!(content.contains("@kind(rfc)"), "{content}");
        assert!(content.contains("New Auth Protocol"), "{content}");
        assert!(content.contains("Bob"), "{content}");
        assert!(created.unfilled.is_empty());
    }

    #[test]
    fn without_a_name_the_blueprint_is_the_directory_the_document_goes_in() {
        let vault = vault_with_rfc("by-directory");
        let context = Context::discover(&vault.0);
        let vars = vars();

        let created = context
            .create(&Document {
                path: Path::new("rfc/002.tmt"),
                blueprint: None,
                force: false,
                vars: &vars,
            })
            .unwrap();

        assert!(created.path.is_file());
    }

    #[test]
    fn an_existing_file_is_kept_unless_forced() {
        let vault = vault_with_rfc("force");
        let context = Context::discover(&vault.0);
        let vars = vars();
        let document = |force| Document {
            path: Path::new("rfcs/003.tmt"),
            blueprint: Some("rfc"),
            force,
            vars: &vars,
        };

        context.create(&document(false)).unwrap();
        assert!(context.create(&document(false)).is_err());
        assert!(context.create(&document(true)).is_ok());
    }

    #[test]
    fn a_variable_with_no_value_is_left_as_written_and_reported() {
        let vault = vault_with_rfc("unfilled");
        let context = Context::discover(&vault.0);

        let created = context
            .create(&Document {
                path: Path::new("rfcs/004.tmt"),
                blueprint: Some("rfc"),
                force: false,
                vars: &[("title".into(), "Only a title".into())],
            })
            .unwrap();

        assert_eq!(created.unfilled.len(), 1, "{:?}", created.unfilled);
        assert!(created.unfilled[0].line > 0);
    }

    #[test]
    fn the_declared_blueprints_are_listed_by_name() {
        let vault = vault_with_rfc("list");
        let context = Context::discover(&vault.0);

        let blueprints = context.blueprints();
        assert_eq!(blueprints.len(), 1);
        assert_eq!(blueprints[0].name, "rfc");
        assert!(context.warnings().is_empty());
    }

    #[test]
    fn a_blueprint_that_is_declared_but_missing_is_a_warning() {
        let vault = TempVault::new("rotted");
        vault.write("default.config.tmt", CONFIG);

        let context = Context::discover(&vault.0);
        assert_eq!(context.warnings().len(), 1, "{:?}", context.warnings());
        assert!(context.warnings()[0].contains("does not exist"));
    }

    #[test]
    fn with_no_config_the_start_directory_is_the_vault() {
        let vault = TempVault::new("no-config");
        let context = Context::discover(&vault.0);
        assert!(context.blueprints().is_empty());
        assert!(context.warnings().is_empty());
    }
}
