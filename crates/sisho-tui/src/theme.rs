//! Color palette and widget styling primitives for `sisho` TUI applications.

use std::borrow::Cow;

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders},
};

/// Color palette and reusable widget styles shared across `sisho` TUI tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Primary brand accent used for app titles, focused accents, and active tabs.
    pub primary: Color,
    /// Secondary accent used for supporting highlights.
    pub secondary: Color,
    /// Foreground color for active tab pills.
    pub active_tab_fg: Color,
    /// Foreground color for inactive tabs and unfocused pane titles.
    pub inactive_tab_fg: Color,
    /// Border color for the focused pane.
    pub border_focused: Color,
    /// Border color for unfocused panes and chrome containers.
    pub border_unfocused: Color,
    /// Muted text color for secondary metadata, separators, and hints.
    pub muted: Color,
    /// Highlight color for keybinding labels.
    pub key_hint: Color,
    /// Color for informational status messages.
    pub info: Color,
    /// Color for success status messages.
    pub success: Color,
    /// Color for warning highlights and modal section headings.
    pub warning: Color,
    /// Color for error status messages.
    pub error: Color,
}

impl Theme {
    /// Returns the default `sisho` TUI theme.
    pub const fn sisho() -> Self {
        Self {
            primary: Color::Rgb(95, 130, 255),
            secondary: Color::Rgb(140, 165, 255),
            active_tab_fg: Color::Black,
            inactive_tab_fg: Color::Gray,
            border_focused: Color::Rgb(95, 130, 255),
            border_unfocused: Color::DarkGray,
            muted: Color::DarkGray,
            key_hint: Color::Yellow,
            info: Color::Rgb(95, 130, 255),
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
        }
    }

    /// Builds a bordered pane [`Block`] styled according to `focused`.
    pub fn pane_block(&self, title: impl Into<String>, focused: bool) -> Block<'static> {
        let border_style = if focused {
            self.focused_border_style()
        } else {
            self.unfocused_border_style()
        };
        let title_style = if focused {
            Style::default()
                .fg(self.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.inactive_tab_fg)
        };

        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title.into())
            .title_style(title_style)
    }

    /// Returns the border style for a focused pane.
    pub fn focused_border_style(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    /// Returns the border style for an unfocused pane.
    pub fn unfocused_border_style(&self) -> Style {
        Style::default().fg(self.border_unfocused)
    }

    /// Returns the style for an active header tab pill.
    pub fn active_tab_style(&self) -> Style {
        Style::default()
            .fg(self.active_tab_fg)
            .bg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Returns the style for an inactive header tab.
    pub fn inactive_tab_style(&self) -> Style {
        Style::default().fg(self.inactive_tab_fg)
    }

    /// Styles a keybinding label with the theme's key hint color and bold weight.
    pub fn key_span<'a>(&self, label: impl Into<Cow<'a, str>>) -> Span<'a> {
        Span::styled(
            label.into(),
            Style::default()
                .fg(self.key_hint)
                .add_modifier(Modifier::BOLD),
        )
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::sisho()
    }
}
