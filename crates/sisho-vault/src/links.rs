//! `sisho check-links`: find the links in a vault that point at nothing.
//!
//! Every `@file`, `@dir`, `@embed` and `@link` that names a local target is
//! checked against the disk. The check is incremental: what it learned about a
//! document is kept in an SQLite cache keyed by the document's modification
//! time, so checking a large vault again does not parse what has not changed.
//! The cache lives in the user's cache directory, never in the vault.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tomet_config::PrinterConfig;
use tomet_links::{BrokenLink as TometBrokenLink, LinkCache, check_vault};
use tomet_load_config::find_config_file;

/// A line and a column, both counted from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// A link whose target does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLink {
    /// The document the link is written in.
    pub source: PathBuf,
    /// The target as written.
    pub target: String,
    /// Where the link is in `source`. `None` when the file can no longer be
    /// read.
    pub at: Option<Position>,
}

/// A document that could not be looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub source: PathBuf,
    pub error: String,
}

/// What a run found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub files_scanned: usize,
    pub links_checked: usize,
    pub broken: Vec<BrokenLink>,
    pub unreadable: Vec<Unreadable>,
}

impl Report {
    /// Whether the run found something the person has to deal with: a link
    /// that is broken, or a document that could not be looked at.
    ///
    /// The second counts. A document that could not be read has links nobody
    /// checked, and a run that exits 0 over it says they are fine -- the same
    /// reason `check` and `refactor --check` fail on a file they could not
    /// read.
    pub fn has_problems(&self) -> bool {
        !self.broken.is_empty() || !self.unreadable.is_empty()
    }
}

/// Where the link cache lives unless told otherwise: in the user's cache
/// directory, one database for every vault on the machine.
pub fn default_cache_path() -> PathBuf {
    tomet_links::default_cache_path()
}

/// Check the links under `path` (a document, or a directory of them), keeping
/// what was learned in the cache at `cache_path`.
///
/// `Err` means the check itself could not run; a broken link is not an error,
/// it is in the [`Report`].
pub fn check(path: &Path, cache_path: &Path) -> Result<Report> {
    if !path.exists() {
        return Err(anyhow!("path '{}' does not exist", path.display()));
    }

    let (config, _, config_root) = find_config_file(path).unwrap_or((
        PrinterConfig::default(),
        path.to_path_buf(),
        path.to_path_buf(),
    ));

    let mut cache = LinkCache::open(cache_path)?;
    let report = check_vault(path, &config, &config_root, &config_root, &mut cache);

    // One database is shared by every vault on the machine, so rows for
    // vaults that have since moved or been deleted would otherwise sit there
    // forever. Nothing reads them, but nothing removes them either.
    let _ = cache.prune_missing();

    let positions = locate(&report.broken);
    Ok(Report {
        files_scanned: report.files_scanned,
        links_checked: report.links_checked,
        broken: report
            .broken
            .into_iter()
            .zip(positions)
            .map(|(link, at)| BrokenLink {
                source: link.source,
                target: link.target,
                at,
            })
            .collect(),
        unreadable: report
            .errors
            .into_iter()
            .map(|(source, error)| Unreadable { source, error })
            .collect(),
    })
}

/// The line and column of each broken link. A link carries a byte span into
/// its source file, so each file is read once and indexed once, however many
/// of its links are broken. `None` when the file can no longer be read.
fn locate(broken: &[TometBrokenLink]) -> Vec<Option<Position>> {
    let mut located = vec![None; broken.len()];
    let mut by_file: BTreeMap<&Path, Vec<usize>> = BTreeMap::new();
    for (i, link) in broken.iter().enumerate() {
        by_file.entry(link.source.as_path()).or_default().push(i);
    }
    for (path, links) in by_file {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let index = tomet_lexer::LineIndex::new(&src);
        for i in links {
            let at = index.line_col(broken[i].span.start);
            located[i] = Some(Position {
                line: at.line,
                column: at.column,
            });
        }
    }
    located
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A directory of documents in a fresh temporary place, removed when
    /// dropped. Its cache database sits beside it, not in the user's.
    struct Tree {
        root: PathBuf,
        cache: PathBuf,
    }

    impl Tree {
        fn new(name: &str) -> Self {
            // The time, not the process id: `process-door` keeps the whole
            // `process` module out of every crate but `sisho-run`.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let base = std::env::temp_dir().join(format!("sisho-links-{nanos}-{name}"));
            let root = base.join("vault");
            fs::create_dir_all(&root).unwrap();
            Self {
                cache: base.join("links.sqlite3"),
                root,
            }
        }

        fn write(&self, rel: &str, text: &str) {
            let path = self.root.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }

        fn check(&self) -> Report {
            check(&self.root, &self.cache).unwrap()
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.root.parent().unwrap());
        }
    }

    #[test]
    fn a_clean_report_has_no_problems() {
        assert!(!Report::default().has_problems());
    }

    #[test]
    fn a_broken_link_is_a_problem() {
        let report = Report {
            broken: vec![BrokenLink {
                source: PathBuf::from("a.tmt"),
                target: "missing.png".into(),
                at: None,
            }],
            ..Default::default()
        };
        assert!(report.has_problems());
    }

    /// No broken link was found, because the document that might have held
    /// one could not be read.
    #[test]
    fn a_document_that_could_not_be_read_is_a_problem() {
        let report = Report {
            unreadable: vec![Unreadable {
                source: PathBuf::from("a.tmt"),
                error: "could not read".into(),
            }],
            ..Default::default()
        };
        assert!(report.has_problems());
    }

    #[test]
    fn a_link_to_a_file_that_exists_is_fine() {
        let tree = Tree::new("fine");
        tree.write("real.txt", "hello");
        tree.write("a.tmt", "@meta{}\n\n@link(real.txt)\n[ here ]\n");

        let report = tree.check();
        assert!(!report.has_problems(), "{report:?}");
        assert_eq!(report.files_scanned, 1);
        assert_eq!(report.links_checked, 1);
    }

    #[test]
    fn a_broken_link_is_reported_with_where_it_is() {
        let tree = Tree::new("broken");
        tree.write("a.tmt", "@meta{}\n\n@link(missing.txt)\n[ nowhere ]\n");

        let report = tree.check();
        assert_eq!(report.broken.len(), 1, "{report:?}");
        let broken = &report.broken[0];
        assert_eq!(broken.target, "missing.txt");
        assert!(broken.source.ends_with("a.tmt"));
        assert_eq!(broken.at, Some(Position { line: 3, column: 1 }));
        assert!(report.has_problems());
    }

    #[test]
    fn an_external_url_is_not_checked() {
        let tree = Tree::new("external");
        tree.write(
            "a.tmt",
            "@meta{}\n\n<embed>[alt](https://example.com/pic.png)\n",
        );

        assert!(!tree.check().has_problems());
    }

    #[test]
    fn a_second_run_over_the_same_cache_gives_the_same_answer() {
        let tree = Tree::new("cached");
        tree.write("a.tmt", "@meta{}\n\n@link(missing.txt)\n[ nowhere ]\n");

        let first = tree.check();
        let second = tree.check();
        assert_eq!(first, second);
    }

    #[test]
    fn a_path_that_does_not_exist_is_an_error_not_a_report() {
        let tree = Tree::new("absent");
        let error = check(&tree.root.join("nope"), &tree.cache).unwrap_err();
        assert!(error.to_string().contains("does not exist"), "{error}");
    }
}
