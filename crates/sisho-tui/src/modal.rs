//! Centered modal dialog frame and structured keybinding help overlay.

use std::borrow::Cow;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::theme::Theme;

/// Computes a centered rectangle occupying `percent_x` by `percent_y` of `area`.
///
/// Percentages above `100` are clamped to `100`.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let px = percent_x.min(100);
    let py = percent_y.min(100);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - py) / 2),
            Constraint::Percentage(py),
            Constraint::Percentage((100 - py) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - px) / 2),
            Constraint::Percentage(px),
            Constraint::Percentage((100 - px) / 2),
        ])
        .split(vertical[1])[1]
}

/// Centered, bordered modal container that clears its background and returns its inner area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalFrame<'a> {
    title: Cow<'a, str>,
    percent_x: u16,
    percent_y: u16,
    border_color: Option<Color>,
}

impl<'a> ModalFrame<'a> {
    /// Creates a modal frame with `title` sized to `percent_x` by `percent_y` of the screen.
    pub fn new(title: impl Into<Cow<'a, str>>, percent_x: u16, percent_y: u16) -> Self {
        Self {
            title: title.into(),
            percent_x,
            percent_y,
            border_color: None,
        }
    }

    /// Overrides the modal border color (defaults to `theme.primary`).
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Clears the modal area, draws the outer bordered block, and returns its inner [`Rect`].
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) -> Rect {
        let modal = centered_rect(self.percent_x, self.percent_y, area);
        frame.render_widget(Clear, modal);

        let border_color = self.border_color.unwrap_or(theme.primary);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(self.title.as_ref().to_owned());
        let inner = block.inner(modal);
        frame.render_widget(block, modal);
        inner
    }
}

/// A single keybinding entry inside a [`HelpSection`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpRow<'a> {
    /// Key or shortcut chord displayed in the left column.
    pub keys: Cow<'a, str>,
    /// Description of the command or action displayed in the right column.
    pub description: Cow<'a, str>,
}

impl<'a> HelpRow<'a> {
    /// Creates a keybinding help row.
    pub fn new(keys: impl Into<Cow<'a, str>>, description: impl Into<Cow<'a, str>>) -> Self {
        Self {
            keys: keys.into(),
            description: description.into(),
        }
    }
}

/// A titled group of [`HelpRow`] items rendered inside [`draw_help_modal`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpSection<'a> {
    /// Section heading text.
    pub title: Cow<'a, str>,
    /// Keybinding rows belonging to this section.
    pub rows: Vec<HelpRow<'a>>,
}

impl<'a> HelpSection<'a> {
    /// Creates a help section with `title` and `rows`.
    pub fn new(
        title: impl Into<Cow<'a, str>>,
        rows: impl IntoIterator<Item = HelpRow<'a>>,
    ) -> Self {
        Self {
            title: title.into(),
            rows: rows.into_iter().collect(),
        }
    }
}

/// Renders a centered keybinding help overlay from `sections` and an optional `footer_hint`.
pub fn draw_help_modal<'a>(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<Cow<'a, str>>,
    sections: &[HelpSection<'a>],
    footer_hint: Option<&str>,
    theme: &Theme,
) {
    let inner = ModalFrame::new(title, 76, 72).render(frame, area, theme);

    let mut lines = Vec::new();
    for (idx, section) in sections.iter().enumerate() {
        if idx > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            section.title.as_ref(),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )));
        for row in &section.rows {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<18}", row.keys),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(row.description.as_ref()),
            ]));
        }
    }

    if let Some(hint) = footer_hint {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            hint,
            Style::default().fg(theme.muted),
        )));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}
