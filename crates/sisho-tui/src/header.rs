//! Top header bar widget with application name, optional context, and tab strip.

use std::borrow::Cow;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::theme::Theme;

/// A single selectable view tab rendered inside [`HeaderBar`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem<'a> {
    /// Shortcut key shown in brackets before the label.
    pub key: &'a str,
    /// Human-readable tab label.
    pub label: Cow<'a, str>,
    /// Whether this tab is currently active.
    pub active: bool,
    /// Optional numeric count displayed as `(N)` after the label.
    pub badge: Option<usize>,
}

impl<'a> TabItem<'a> {
    /// Creates a tab item without a badge count.
    pub fn new(key: &'a str, label: impl Into<Cow<'a, str>>, active: bool) -> Self {
        Self {
            key,
            label: label.into(),
            active,
            badge: None,
        }
    }

    /// Attaches a numeric badge count rendered as `(count)` after the tab label.
    pub fn badge(mut self, count: usize) -> Self {
        self.badge = Some(count);
        self
    }
}

/// Top chrome bar displaying the application title, optional context, and view tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderBar<'a> {
    app_name: &'a str,
    context: Option<Cow<'a, str>>,
    tabs: Vec<TabItem<'a>>,
}

impl<'a> HeaderBar<'a> {
    /// Creates a header bar for `app_name` with no context and no tabs.
    pub fn new(app_name: &'a str) -> Self {
        Self {
            app_name,
            context: None,
            tabs: Vec::new(),
        }
    }

    /// Sets an optional context string rendered as `· <context>` beside the app name.
    pub fn context(mut self, context: impl Into<Cow<'a, str>>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Populates the tab strip rendered after the header title and context.
    pub fn tabs(mut self, tabs: impl IntoIterator<Item = TabItem<'a>>) -> Self {
        self.tabs = tabs.into_iter().collect();
        self
    }

    /// Renders the header bar into `area` using `theme`.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut spans = Vec::with_capacity(2 + self.tabs.len() * 2);
        spans.push(Span::styled(
            format!(" {} ", self.app_name),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

        if let Some(context) = &self.context {
            let suffix = if self.tabs.is_empty() { "" } else { "   " };
            spans.push(Span::styled(
                format!("· {context}{suffix}"),
                Style::default().fg(theme.muted),
            ));
        }

        for (idx, tab) in self.tabs.iter().enumerate() {
            if idx > 0 {
                spans.push(Span::raw(" "));
            }
            let text = match tab.badge {
                Some(count) => format!(" [{}] {} ({count}) ", tab.key, tab.label),
                None => format!(" [{}] {} ", tab.key, tab.label),
            };
            let style = if tab.active {
                theme.active_tab_style()
            } else {
                theme.inactive_tab_style()
            };
            spans.push(Span::styled(text, style));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.unfocused_border_style());
        let paragraph = Paragraph::new(Line::from(spans)).block(block);
        frame.render_widget(paragraph, area);
    }
}
