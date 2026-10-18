//! `sisho refactor`: migrate documents away from spellings the language has
//! moved off.
//!
//! Two passes exist: URLs that match a `@config` macro pattern become
//! `$macro(...)` invocations, and `@meta{type: ...}` becomes `@kind(...)`.
//! A sweep reads every document under a path and works out what each pass
//! would change; nothing is written until [`Sweep::save`].
//!
//! Writing is lossy. The documents are re-printed from the syntax tree, which
//! carries no comments, so look at [`Sweep::changed`] before saving.

use std::path::{Path, PathBuf};

use anyhow::Result;
use tomet_workspace::{RefactorOptions, RefactorReport, refactor_workspace, save_file_diffs};

/// Which passes to run, and over what.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Run only the URL-to-macro pass.
    pub url_macros: bool,
    /// Run only the `@meta` type to `@kind` pass.
    pub meta_kind: bool,
    /// Also reach files and directories the project config excludes
    /// (`workspace.ignore`, `workspace.unswept`). Without it, a file named
    /// explicitly that the config excludes is skipped, as it would be in a
    /// sweep of its directory.
    pub force: bool,
}

impl Options {
    fn to_tomet(self) -> RefactorOptions {
        // Naming neither pass means both.
        let (url_to_macros, meta_type_to_kind) = if self.url_macros || self.meta_kind {
            (self.url_macros, self.meta_kind)
        } else {
            (true, true)
        };
        RefactorOptions {
            url_to_macros,
            meta_type_to_kind,
            force: self.force,
        }
    }
}

/// A document the sweep looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub path: PathBuf,
    /// How many edits the passes made. Zero means the document is already in
    /// the current spelling.
    pub changes: usize,
    /// The document as it would be written.
    pub modified_src: String,
}

impl File {
    pub fn is_changed(&self) -> bool {
        self.changes > 0
    }
}

/// A document the sweep could not read or parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unchecked {
    pub path: PathBuf,
    pub error: String,
}

/// What a sweep found.
pub struct Sweep {
    /// Every document that was looked at, changed or not.
    pub files: Vec<File>,
    /// Documents that could not be looked at.
    ///
    /// A document the sweep could not read is not a document it checked, and
    /// it is the likeliest one to still carry the spelling being migrated
    /// away from.
    pub unchecked: Vec<Unchecked>,
    /// Files named explicitly that the config excludes, left alone because
    /// [`Options::force`] was off.
    pub skipped: Vec<PathBuf>,
    report: RefactorReport,
}

impl Sweep {
    /// The documents the passes would change.
    pub fn changed(&self) -> Vec<&File> {
        self.files.iter().filter(|file| file.is_changed()).collect()
    }

    /// Whether every document was looked at and none needs changing. A
    /// document that could not be looked at keeps this false.
    pub fn is_clean(&self) -> bool {
        self.unchecked.is_empty() && self.files.iter().all(|file| !file.is_changed())
    }

    /// Write the changed documents back. Returns how many were written.
    pub fn save(&mut self) -> Result<usize> {
        save_file_diffs(&mut self.report.diffs)
    }
}

/// Look at every document under `path` (a document, or a directory of them).
///
/// `Err` means the sweep itself could not run. A document that could not be
/// parsed does not stop it; it is in [`Sweep::unchecked`].
pub fn sweep(path: &Path, options: Options) -> Result<Sweep> {
    let report = refactor_workspace(path, &options.to_tomet())?;
    Ok(Sweep {
        files: report
            .diffs
            .iter()
            .map(|diff| File {
                path: diff.path.clone(),
                changes: diff.changes_count,
                modified_src: diff.modified_src.clone(),
            })
            .collect(),
        unchecked: report
            .errors
            .iter()
            .map(|(path, error)| Unchecked {
                path: path.clone(),
                error: error.clone(),
            })
            .collect(),
        skipped: report.skipped.clone(),
        report,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A directory in a fresh temporary place, removed when dropped.
    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Self {
            // The time, not the process id: `process-door` keeps the whole
            // `process` module out of every crate but `sisho-run`.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("sisho-refactor-{nanos}-{name}"));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, rel: &str, text: &str) -> PathBuf {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            path
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const OLD: &str = r#"@version(1.0)
@meta{
  type: note
  title: "My Title"
}
@config{
  macros: {
    gh: "https://github.com/${1}"
  }
}

- @link("https://github.com/tomet/tomet")[Repo]
"#;

    #[test]
    fn saving_rewrites_both_spellings() {
        let dir = Dir::new("save");
        let file = dir.write("doc.tmt", OLD);

        let mut sweep = sweep(&file, Options::default()).unwrap();
        assert_eq!(sweep.changed().len(), 1);
        assert!(!sweep.is_clean());
        assert_eq!(sweep.save().unwrap(), 1);

        let refactored = fs::read_to_string(&file).unwrap();
        assert!(refactored.contains("@kind(note)"), "{refactored}");
        assert!(
            refactored.contains("$macro.gh(\"tomet/tomet\")"),
            "{refactored}"
        );
    }

    #[test]
    fn looking_writes_nothing() {
        let dir = Dir::new("look");
        let file = dir.write("doc.tmt", OLD);

        let sweep = sweep(&file, Options::default()).unwrap();
        assert!(sweep.changed()[0].modified_src.contains("@kind(note)"));
        assert_eq!(fs::read_to_string(&file).unwrap(), OLD);
    }

    #[test]
    fn naming_a_pass_runs_only_that_pass() {
        let dir = Dir::new("one-pass");
        let file = dir.write("doc.tmt", OLD);

        let sweep = sweep(
            &file,
            Options {
                meta_kind: true,
                ..Options::default()
            },
        )
        .unwrap();
        let modified = &sweep.changed()[0].modified_src;
        assert!(modified.contains("@kind(note)"), "{modified}");
        assert!(!modified.contains("$macro"), "{modified}");
    }

    #[test]
    fn a_document_already_in_the_current_spelling_is_clean() {
        let dir = Dir::new("clean");
        let file = dir.write("doc.tmt", "@kind(note)\n\n=[ Title ]\n");

        let sweep = sweep(&file, Options::default()).unwrap();
        assert!(sweep.is_clean());
        assert_eq!(sweep.files.len(), 1);
        assert!(sweep.changed().is_empty());
    }

    /// A file that does not parse is a file the sweep did not check, and a
    /// check has to say so by not calling the tree clean. It used to warn and
    /// pass, which reported a clean tree containing a file nobody had looked
    /// at.
    #[test]
    fn a_document_that_cannot_be_parsed_keeps_the_tree_from_being_clean() {
        let dir = Dir::new("unparseable");
        dir.write("fine.tmt", "@kind(note)\n\n=[ Title ]\n");
        dir.write("broken.tmt", "=[ unterminated\n");

        let sweep = sweep(&dir.0, Options::default()).unwrap();
        assert_eq!(sweep.unchecked.len(), 1, "{:?}", sweep.unchecked);
        assert!(sweep.unchecked[0].path.ends_with("broken.tmt"));
        assert!(!sweep.is_clean());
    }

    /// `refactor -i` rewrites files from the syntax tree, so it must not be
    /// pointed at the frozen corpus just because the path was spelled out.
    #[test]
    fn an_explicit_file_the_config_leaves_alone_is_skipped_until_forced() {
        let vault = Dir::new("unswept");
        vault.write(
            "default.config.tmt",
            "@kind(config)\n@config{ workspace: { unswept: list(\"tests/fixtures\") } }\n",
        );
        let old = "@meta{\n  type: note\n}\n";
        let frozen = vault.write("tests/fixtures/f.tmt", old);

        let mut left_alone = sweep(&frozen, Options::default()).unwrap();
        assert_eq!(left_alone.skipped.len(), 1);
        left_alone.save().unwrap();
        assert_eq!(fs::read_to_string(&frozen).unwrap(), old, "left alone");

        let mut forced = sweep(
            &frozen,
            Options {
                force: true,
                ..Options::default()
            },
        )
        .unwrap();
        assert!(forced.skipped.is_empty());
        forced.save().unwrap();
        assert_ne!(
            fs::read_to_string(&frozen).unwrap(),
            old,
            "--force reaches it"
        );
    }
}
