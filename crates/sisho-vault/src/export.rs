//! `sisho export`: write documents out in the formats their `@config` asks
//! for, and check that what was written is still what the source renders to.
//!
//! A document declares its outputs in `@config(export: { type: ..., path: ...
//! })`. A path it declares means the same thing as it does in a link: bare is
//! measured from the project root (the directory holding the nearest
//! `default.config.tmt`), `./` and `../` from the document's own directory. It
//! used to be measured from the process's working directory, so the output
//! landed in a different place depending on where you stood, and `--check`
//! only agreed with itself from one of them.
//!
//! The work returns what was done, document by document, as data. With
//! [`Options::check`] nothing is written: a generated file that disagrees with
//! its source is [`Action::Stale`], a failure with a mechanical fix, which
//! beats a rule asking people to remember not to hand-edit it.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tomet_load::Vault;
use tomet_semantics::{ExportType, document_config};

use crate::render;

/// What to export, and how.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options<'a> {
    /// Export this format instead of what the documents declare: a format
    /// name, or `all` for every declared format (or CommonMark and HTML if a
    /// document declares none).
    pub only_type: Option<&'a str>,
    /// Write here instead of where the documents declare. A directory takes
    /// one file per format named after the document; a document sweep keeps
    /// the sub-directory structure under it.
    pub out: Option<&'a Path>,
    /// Number HTML headings by nesting level and give them slugs.
    pub advanced: bool,
    /// Compare each output with what the source renders to now, and write
    /// nothing.
    pub check: bool,
}

/// What happened to one output of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The output was written to `dest`.
    Wrote { dest: PathBuf },
    /// Checking: `dest` differs from what the source renders to now, or is
    /// missing. A missing file counts as stale: the source says a file should
    /// be there, so its absence is exactly what this is for.
    Stale { dest: PathBuf },
    /// Checking: `dest` is what the source renders to now.
    Current { dest: PathBuf },
    /// The document declares no destination and none was given, so the
    /// output is for the standard output.
    Printed { text: String },
}

/// What was done for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub source: PathBuf,
    /// The outputs handled, in order. Present even when the document then
    /// failed: what was written before the failure is on disk.
    pub actions: Vec<Action>,
    /// Why the document could not be exported (or checked) any further.
    pub error: Option<String>,
}

/// Everything a run did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Whether the target was a directory.
    pub directory: bool,
    /// The documents, in the order they were handled. Empty for a directory
    /// that holds none.
    pub documents: Vec<Document>,
}

impl Run {
    /// The outputs that are out of date, in order. Only a check has any.
    pub fn stale(&self) -> Vec<&Path> {
        self.documents
            .iter()
            .flat_map(|document| &document.actions)
            .filter_map(|action| match action {
                Action::Stale { dest } => Some(dest.as_path()),
                _ => None,
            })
            .collect()
    }

    /// How many documents could not be exported (or checked).
    pub fn failed(&self) -> usize {
        self.documents
            .iter()
            .filter(|document| document.error.is_some())
            .count()
    }
}

/// Export `target` (a document, or a directory of them).
///
/// `Err` means the run could not start: the path does not exist. A document
/// that fails does not stop the others; it is in [`Document::error`].
pub fn export(target: &Path, options: Options<'_>) -> Result<Run> {
    export_with(target, options, |_| {})
}

/// [`export`], telling `on_document` about each document as soon as it is
/// done, so a long run can show progress.
pub fn export_with(
    target: &Path,
    options: Options<'_>,
    mut on_document: impl FnMut(&Document),
) -> Result<Run> {
    if target.is_file() {
        let project_root = project_root_for(target);
        let vault = Vault::discover(target);
        let settings = Settings {
            options,
            project_root: &project_root,
            vault: &vault,
        };
        let document = export_one(&settings, target, options.out);
        on_document(&document);
        Ok(Run {
            directory: false,
            documents: vec![document],
        })
    } else if target.is_dir() {
        Ok(export_directory(target, options, &mut on_document))
    } else {
        Err(anyhow!("path '{}' does not exist", target.display()))
    }
}

/// Where a declared output path is measured from: the directory holding the
/// nearest `default.config.tmt`, which is what declares where the vault
/// begins.
///
/// Falls back to the target's own directory when there is no config to find,
/// which keeps a loose file exporting beside itself rather than into
/// whatever directory the shell happened to be in.
fn project_root_for(target: &Path) -> PathBuf {
    if let Some((_, _, root)) = tomet_load_config::find_config_file(target) {
        return root;
    }
    if target.is_dir() {
        return target.to_path_buf();
    }
    target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// What stays the same for every document of one run.
struct Settings<'a> {
    options: Options<'a>,
    project_root: &'a Path,
    vault: &'a Vault,
}

fn export_directory(
    dir: &Path,
    options: Options<'_>,
    on_document: &mut impl FnMut(&Document),
) -> Run {
    let files = tomet_indexer::collect_tm_files(dir);
    let project_root = project_root_for(dir);
    // Discovered once for the whole run; the metadata table inside it is
    // filled by whichever document asks first.
    let vault = Vault::discover(dir);
    let settings = Settings {
        options,
        project_root: &project_root,
        vault: &vault,
    };

    let mut documents = Vec::with_capacity(files.len());
    for file in &files {
        let out = options.out.map(|out_dir| match file.strip_prefix(dir) {
            Ok(rel) => out_dir.join(rel.parent().unwrap_or_else(|| Path::new(""))),
            Err(_) => out_dir.to_path_buf(),
        });
        let document = export_one(&settings, file, out.as_deref());
        on_document(&document);
        documents.push(document);
    }
    Run {
        directory: true,
        documents,
    }
}

fn export_one(settings: &Settings<'_>, file: &Path, out: Option<&Path>) -> Document {
    let mut actions = Vec::new();
    let error = export_into(settings, file, out, &mut actions)
        .err()
        .map(|e| e.to_string());
    Document {
        source: file.to_path_buf(),
        actions,
        error,
    }
}

fn export_into(
    settings: &Settings<'_>,
    file: &Path,
    override_out: Option<&Path>,
    actions: &mut Vec<Action>,
) -> Result<()> {
    let Options {
        only_type,
        advanced,
        check,
        ..
    } = settings.options;

    let src = fs::read_to_string(file)?;
    // Every `${...}` is resolved before anything renders, so every converter
    // is handed its value and none of them needs to know expressions exist.
    let doc = render::prepared(settings.vault, file, &src)?;

    let config = document_config(&doc);

    let targets = if let Some(requested) = only_type {
        if requested.eq_ignore_ascii_case("all") {
            if !config.export_type.is_empty() {
                config.export_type.clone()
            } else {
                vec![ExportType::CommonMark, ExportType::Html]
            }
        } else {
            vec![ExportType::parse(requested)]
        }
    } else if !config.export_type.is_empty() {
        config.export_type.clone()
    } else {
        vec![ExportType::CommonMark]
    };

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    for target in &targets {
        let ext = match target {
            ExportType::CommonMark => "md",
            ExportType::Html => "html",
            ExportType::Typst => "typ",
            ExportType::Pandoc => "json",
            ExportType::Custom(s) => s.as_str(),
        };

        let dest = if let Some(out) = override_out {
            if out.is_dir() || targets.len() > 1 || file.is_dir() {
                Some(out.join(format!("{stem}.{ext}")))
            } else {
                Some(out.to_path_buf())
            }
        } else if let Some(declared) = config.export_path_for(target) {
            Some(tomet_indexer::resolve_document_relative(
                file,
                declared,
                settings.project_root,
            ))
        } else if targets.len() == 1 {
            None
        } else {
            Some(PathBuf::from(format!("{stem}.{ext}")))
        };

        let rendered = match target {
            ExportType::CommonMark => render::commonmark(&doc),
            ExportType::Pandoc => render::pandoc_json(&doc)?,
            ExportType::Html => {
                let fallback = file.file_stem().and_then(|s| s.to_str()).unwrap_or("Tomet");
                render::html(
                    &doc,
                    fallback,
                    &render::Html {
                        advanced,
                        ..render::Html::default()
                    },
                )
            }
            ExportType::Typst => render::typst(&doc),
            ExportType::Custom(format) => {
                return Err(anyhow!("unsupported export type: {format}"));
            }
        };

        match dest {
            Some(dest) if check => {
                let current = fs::read_to_string(&dest).unwrap_or_default();
                actions.push(if current == rendered {
                    Action::Current { dest }
                } else {
                    Action::Stale { dest }
                });
            }
            Some(dest) => {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&dest, &rendered)?;
                actions.push(Action::Wrote { dest });
            }
            None if !check => actions.push(Action::Printed { text: rendered }),
            None => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
            let dir = std::env::temp_dir().join(format!("sisho-export-{nanos}-{name}"));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, rel: &str, text: &str) -> PathBuf {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            path
        }

        fn path(&self, rel: &str) -> PathBuf {
            self.0.join(rel)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const CHECK: Options<'static> = Options {
        only_type: None,
        out: None,
        advanced: false,
        check: true,
    };

    fn write_mode() -> Options<'static> {
        Options::default()
    }

    /// A bare declared path is measured from the project root -- the
    /// directory holding `default.config.tmt` -- and not from wherever the
    /// process happens to be standing.
    ///
    /// The source sits in a subdirectory and declares plain `out.md`.
    /// Landing beside the source, or in the test runner's own working
    /// directory, are the two ways this used to go wrong; both are asserted
    /// against.
    #[test]
    fn a_bare_export_path_is_measured_from_the_project_root() {
        let root = Dir::new("root");
        root.write("default.config.tmt", "@kind(config)\n");
        let src = root.write(
            "sub/doc.tmt",
            "@config(export: { type: commonmark, path: \"out.md\" })\n\n#[ Title ]\n",
        );

        let run = export(&src, write_mode()).unwrap();
        assert_eq!(run.failed(), 0, "{run:?}");

        assert!(root.path("out.md").is_file(), "output belongs at the root");
        assert!(
            !root.path("sub/out.md").exists(),
            "output must not land beside its source"
        );
        assert!(
            !Path::new("out.md").exists(),
            "output must not land in the working directory"
        );

        // And `--check` agrees with the same file, which is what makes it
        // usable as a guard from anywhere.
        let checked = export(&src, CHECK).unwrap();
        assert!(checked.stale().is_empty(), "{checked:?}");
    }

    /// `--check` has to fail before it is worth running. Exports once,
    /// confirms clean, edits the output by hand, confirms it is caught.
    #[test]
    fn check_catches_a_hand_edited_export() {
        let dir = Dir::new("hand-edit");
        let out = dir.path("doc.md");
        let src = dir.write(
            "doc.tmt",
            &format!(
                "@config(export: {{ type: commonmark, path: \"{}\" }})\n\n#[ Title ]\n",
                out.display()
            ),
        );

        export(&src, write_mode()).unwrap();
        assert!(export(&src, CHECK).unwrap().stale().is_empty());

        fs::write(&out, "# Hand edited\n").unwrap();
        assert_eq!(export(&src, CHECK).unwrap().stale(), [out.as_path()]);
    }

    #[test]
    fn a_missing_output_is_stale() {
        let dir = Dir::new("missing");
        let src = dir.write(
            "doc.tmt",
            "@config(export: { type: commonmark, path: \"doc.md\" })\n\n#[ Title ]\n",
        );
        let stale = export(&src, CHECK).unwrap();
        assert_eq!(stale.stale().len(), 1);
    }

    #[test]
    fn checking_writes_nothing() {
        let dir = Dir::new("check-writes-nothing");
        let src = dir.write(
            "doc.tmt",
            "@config(export: { type: commonmark, path: \"doc.md\" })\n\n#[ Title ]\n",
        );
        export(&src, CHECK).unwrap();
        assert!(!dir.path("doc.md").exists());
    }

    /// A directory with one document that cannot be exported, and one that can.
    fn dir_with_a_broken_document(name: &str) -> Dir {
        let dir = Dir::new(name);
        dir.write("default.config.tmt", "@kind(config)\n");
        dir.write(
            "good.tmt",
            "@config(export: { type: commonmark, path: \"good.md\" })\n\n#[ Good ]\n",
        );
        dir.write("broken.tmt", "=[ unterminated\n");
        dir
    }

    /// A document that could not be exported is a document the run did not
    /// do, and the run has to be able to say so. The rest is still done: one
    /// bad document does not hide the others.
    #[test]
    fn one_broken_document_is_reported_and_the_others_are_still_exported() {
        let dir = dir_with_a_broken_document("run");
        let run = export(&dir.0, write_mode()).unwrap();

        assert_eq!(run.failed(), 1);
        // The config file is a `.tmt` document too, so a sweep handles it.
        assert_eq!(run.documents.len(), 3);
        assert!(dir.path("good.md").is_file());
        let broken = run
            .documents
            .iter()
            .find(|d| d.source.ends_with("broken.tmt"))
            .unwrap();
        assert!(broken.error.as_ref().unwrap().contains("error:"));
    }

    /// A guard that cannot read a document has not checked it. Passing there
    /// is the same as saying it is up to date.
    #[test]
    fn a_document_that_could_not_be_checked_is_counted() {
        let dir = dir_with_a_broken_document("check");
        export(&dir.path("good.tmt"), write_mode()).unwrap();

        let run = export(&dir.0, CHECK).unwrap();
        assert_eq!(run.failed(), 1);
        assert!(run.stale().is_empty());
    }

    #[test]
    fn a_directory_of_good_documents_still_passes() {
        let dir = dir_with_a_broken_document("good");
        fs::remove_file(dir.path("broken.tmt")).unwrap();

        let run = export(&dir.0, write_mode()).unwrap();
        assert_eq!(run.failed(), 0);
        let checked = export(&dir.0, CHECK).unwrap();
        assert_eq!(checked.failed(), 0);
        assert!(checked.stale().is_empty());
    }

    #[test]
    fn a_directory_with_no_documents_is_a_run_with_none() {
        let dir = Dir::new("empty");
        let run = export(&dir.0, write_mode()).unwrap();
        assert!(run.directory);
        assert!(run.documents.is_empty());
    }

    #[test]
    fn a_path_that_does_not_exist_is_an_error() {
        let dir = Dir::new("absent");
        let error = export(&dir.path("nope"), write_mode()).unwrap_err();
        assert!(error.to_string().contains("does not exist"), "{error}");
    }

    #[test]
    fn a_document_with_no_destination_is_printed() {
        let dir = Dir::new("print");
        let src = dir.write("doc.tmt", "=[ Hello Export ]\n");

        let run = export(&src, write_mode()).unwrap();
        match &run.documents[0].actions[..] {
            [Action::Printed { text }] => assert!(text.contains("# Hello Export"), "{text}"),
            other => panic!("{other:?}"),
        }
        // Checking has nothing to compare it with, and prints nothing.
        assert!(export(&src, CHECK).unwrap().documents[0].actions.is_empty());
    }

    #[test]
    fn all_exports_every_declared_format_beside_the_source_without_a_path() {
        let dir = Dir::new("all");
        let src = dir.write("doc.tmt", "=[ Hello ]\n");

        let out = dir.path("out");
        let run = export(
            &src,
            Options {
                only_type: Some("all"),
                out: Some(&out),
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(run.failed(), 0, "{run:?}");
        assert!(out.join("doc.md").is_file());
        assert!(out.join("doc.html").is_file());
    }

    #[test]
    fn an_unsupported_type_fails_that_document() {
        let dir = Dir::new("custom");
        let src = dir.write(
            "doc.tmt",
            "@config(export: { type: weird, path: \"x.weird\" })\n\n=[ T ]\n",
        );
        let run = export(&src, write_mode()).unwrap();
        assert_eq!(
            run.documents[0].error.as_deref(),
            Some("unsupported export type: weird")
        );
    }

    #[test]
    fn export_single_file_with_config_writes_output_file() {
        let dir = Dir::new("single");
        let out = dir.path("test_out.md");
        let src = dir.write(
            "test_doc.tmt",
            &format!(
                "@config(\n  export: {{\n    type: commonmark\n    path: \"{}\"\n  }}\n)\n=[ Hello Export ]\n",
                out.display()
            ),
        );

        let run = export(&src, write_mode()).unwrap();
        assert_eq!(run.failed(), 0, "{run:?}");
        assert!(fs::read_to_string(&out).unwrap().contains("# Hello Export"));
    }
}
