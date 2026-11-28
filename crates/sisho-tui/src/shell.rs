//! Top-level vertical layout splitting a screen into header, body, and footer regions.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Standard three-row chrome layout shared by `sisho` TUI screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellLayout {
    /// Top header bar area (height 3).
    pub header: Rect,
    /// Main content area (minimum height 10).
    pub body: Rect,
    /// Bottom two-line footer area (height 2).
    pub footer: Rect,
}

impl ShellLayout {
    /// Splits `area` vertically into header (`Length(3)`), body (`Min(10)`), and footer (`Length(2)`).
    pub fn split(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(area);

        Self {
            header: chunks[0],
            body: chunks[1],
            footer: chunks[2],
        }
    }
}
