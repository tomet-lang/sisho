//! A parsed document, written out in each of the formats `tomet` can produce.
//!
//! Every function takes a document whose `${...}` expressions have already
//! been resolved (see `Vault::prepare`), so none of the converters needs to
//! know expressions exist.

use anyhow::Result;
use tomet_ast::Document;

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

/// A standalone HTML page. The title is the document's `@meta` title, or
/// `fallback_title` (the file name) without one. `advanced` numbers the
/// headings and gives them slugs.
pub fn html_page(doc: &Document, fallback_title: &str, advanced: bool) -> String {
    let title = meta_title(doc).unwrap_or_else(|| fallback_title.to_string());
    let options = tomet_html::RenderOptions {
        number_headings: advanced,
        auto_slug_headings: advanced,
        lang: None,
        ..Default::default()
    };
    tomet_html::render_page_with(doc, &title, &options)
}
