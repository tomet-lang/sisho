//! How a parse error is shown: the message, where it is, and the line it is
//! on with a pointer at the column.

use std::path::Path;

/// A parse error of the document `path`, whose text is `src`, shown with the
/// offending line and a pointer at the column.
pub fn parse_error(path: &Path, src: &str, err: &tomet_parser::Error) -> String {
    let line_num = err.line;
    let col_num = err.column;

    let line_text = src.lines().nth(line_num.saturating_sub(1)).unwrap_or("");

    let indent = " ".repeat(col_num.saturating_sub(1));
    let line_str = line_num.to_string();
    let padding = " ".repeat(line_str.len());

    format!(
        "error: {}\n  --> {}:{}:{}\n   {}\n {} | {}\n   {}| {}^ {}",
        err.message,
        path.display(),
        line_num,
        col_num,
        "|",
        line_str,
        line_text,
        padding,
        indent,
        err.message
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parse_error_is_shown_with_its_line_and_a_pointer() {
        let err = tomet_parser::Error {
            message: "expected ']'".into(),
            line: 1,
            column: 11,
            offset: 10,
        };
        let src = "@caution[ unterminated";
        let shown = parse_error(Path::new("test.tmt"), src, &err);
        assert!(shown.contains("error: expected ']'"));
        assert!(shown.contains("--> test.tmt:1:11"));
        assert!(shown.contains("@caution[ unterminated"));
        assert!(shown.contains("^ expected ']'"));
    }
}
