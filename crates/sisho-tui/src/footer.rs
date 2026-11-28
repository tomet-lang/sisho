//! Two-line bottom footer bar showing key hints and status or summary text.

use std::borrow::Cow;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::theme::Theme;

/// Severity level for a footer status message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    /// Informational feedback (`ℹ ` prefix).
    Info,
    /// Successful operation feedback (`✓ ` prefix).
    Success,
    /// Error feedback (`! ` prefix).
    Error,
}

/// Status message displayed on the second line of [`FooterBar`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine<'a> {
    /// Severity level determining the prefix icon and color.
    pub kind: StatusKind,
    /// Message text displayed after the prefix icon.
    pub text: Cow<'a, str>,
}

impl<'a> StatusLine<'a> {
    /// Creates a status line with the given `kind` and `text`.
    pub fn new(kind: StatusKind, text: impl Into<Cow<'a, str>>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }

    /// Creates an informational status line.
    pub fn info(text: impl Into<Cow<'a, str>>) -> Self {
        Self::new(StatusKind::Info, text)
    }

    /// Creates a success status line.
    pub fn success(text: impl Into<Cow<'a, str>>) -> Self {
        Self::new(StatusKind::Success, text)
    }

    /// Creates an error status line.
    pub fn error(text: impl Into<Cow<'a, str>>) -> Self {
        Self::new(StatusKind::Error, text)
    }
}

/// A single keybinding hint rendered on the first line of [`FooterBar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyHint<'a> {
    /// Key or shortcut label (e.g. `"q"`, `"1-3"`, `"Enter"`).
    pub key: &'a str,
    /// Short action description shown next to the key.
    pub label: &'a str,
}

impl<'a> KeyHint<'a> {
    /// Creates a keybinding hint pair.
    pub const fn new(key: &'a str, label: &'a str) -> Self {
        Self { key, label }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum HintSource<'a> {
    #[default]
    Empty,
    Structured(&'a [KeyHint<'a>]),
    Text(Cow<'a, str>),
}

/// Two-line footer widget rendering keybinding hints above a status or summary line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FooterBar<'a> {
    hints: HintSource<'a>,
    status: Option<StatusLine<'a>>,
    summary: Option<Cow<'a, str>>,
}

impl<'a> FooterBar<'a> {
    /// Creates an empty footer bar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets structured keybinding hints rendered on the first line.
    pub fn hints(mut self, hints: &'a [KeyHint<'a>]) -> Self {
        self.hints = HintSource::Structured(hints);
        self
    }

    /// Sets raw muted hint text rendered on the first line.
    pub fn hint_text(mut self, text: impl Into<Cow<'a, str>>) -> Self {
        self.hints = HintSource::Text(text.into());
        self
    }

    /// Sets an optional status message shown on the second line, overriding `summary`.
    pub fn status(mut self, status: Option<StatusLine<'a>>) -> Self {
        self.status = status;
        self
    }

    /// Sets a muted summary string shown on the second line when no status is active.
    pub fn summary(mut self, summary: impl Into<Cow<'a, str>>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Renders the two-line footer into `area` using `theme`.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let hint_line = match &self.hints {
            HintSource::Empty => Line::default(),
            HintSource::Text(text) => Line::from(Span::styled(
                text.as_ref(),
                Style::default().fg(theme.muted),
            )),
            HintSource::Structured(hints) => {
                let mut spans = Vec::with_capacity(hints.len() * 3);
                for (idx, hint) in hints.iter().enumerate() {
                    if idx > 0 {
                        spans.push(Span::styled(" · ", Style::default().fg(theme.muted)));
                    }
                    spans.push(theme.key_span(format!("[{}]", hint.key)));
                    spans.push(Span::styled(
                        format!(" {}", hint.label),
                        Style::default().fg(theme.muted),
                    ));
                }
                Line::from(spans)
            }
        };

        let second_line = match (&self.status, &self.summary) {
            (Some(status), _) => {
                let (prefix, style) = match status.kind {
                    StatusKind::Info => ("ℹ ", Style::default().fg(theme.info)),
                    StatusKind::Success => ("✓ ", Style::default().fg(theme.success)),
                    StatusKind::Error => (
                        "! ",
                        Style::default()
                            .fg(theme.error)
                            .add_modifier(Modifier::BOLD),
                    ),
                };
                Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(status.text.as_ref(), style),
                ])
            }
            (None, Some(summary)) => Line::from(Span::styled(
                summary.as_ref(),
                Style::default().fg(theme.muted),
            )),
            (None, None) => Line::default(),
        };

        frame.render_widget(Paragraph::new(vec![hint_line, second_line]), area);
    }
}
