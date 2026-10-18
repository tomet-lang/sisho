//! A parsed document, written out in each of the formats `tomet` can produce.
//!
//! Every function that renders takes a document whose `${...}` expressions
//! have already been resolved (see [`prepared`]), so none of the converters
//! needs to know expressions exist.

use std::path::Path;

use anyhow::{Result, anyhow};
use tomet_ast::Document;
use tomet_load::Vault;

use crate::diagnostic;

/// Parse `src`, the text of the document `path`, in `vault`, and resolve its
/// `${...}` expressions.
///
/// The vault settles what a `${macro.…}` or a `${self.…}` means, so every
/// command that renders a document agrees about it. A parse error comes back
/// shown with its line and a pointer at the column.
pub fn prepared(vault: &Vault, path: &Path, src: &str) -> Result<Document> {
    let (mut doc, _bindings) = vault
        .parse(src)
        .map_err(|e| anyhow!("{}", diagnostic::parse_error(path, src, &e)))?;
    vault.prepare(&mut doc, path);
    Ok(doc)
}

/// The `title` string of the document's `@meta` block, if it has one. An
/// HTML page prefers it to the file name.
pub fn meta_title(doc: &Document) -> Option<String> {
    let meta = tomet_semantics::document_meta(doc)?;
    let tomet_ast::Value::Map(map) = meta else {
        return None;
    };
    map.iter().find_map(|(key, value)| {
        if key != "title" {
            return None;
        }
        match value {
            tomet_ast::Value::String(s) => Some(s.clone()),
            _ => None,
        }
    })
}

pub fn commonmark(doc: &Document) -> String {
    tomet_markdown::to_markdown(doc)
}

pub fn typst(doc: &Document) -> String {
    tomet_typst::to_typst(doc)
}

/// Pandoc's AST as JSON, not a rendering: `pandoc -f json` turns it into
/// whatever format is actually wanted.
pub fn pandoc_json(doc: &Document) -> Result<String> {
    Ok(serde_json::to_string(&tomet_pandoc::to_pandoc(doc))?)
}

/// How HTML is written.
#[derive(Debug, Clone, Default)]
pub struct Html {
    /// Number headings by nesting level and give them slugs.
    pub advanced: bool,
    /// `<html lang="...">` for the page shell.
    pub lang: Option<String>,
    /// Just the rendered body, with no `<html>` / `<head>` shell, for
    /// templating into a page of your own. The shell is what carries `lang`
    /// and the title, so both are ignored in this mode.
    pub body: bool,
}

/// HTML: a standalone page, or the body alone. A page is titled with the
/// document's `@meta` title, or `fallback_title` (the file name) without one.
pub fn html(doc: &Document, fallback_title: &str, how: &Html) -> String {
    let options = tomet_html::RenderOptions {
        number_headings: how.advanced,
        auto_slug_headings: how.advanced,
        lang: how.lang.clone(),
        ..Default::default()
    };
    if how.body {
        return tomet_html::render_body_with(doc, &options);
    }
    let title = meta_title(doc).unwrap_or_else(|| fallback_title.to_string());
    tomet_html::render_page_with(doc, &title, &options)
}
