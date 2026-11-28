//! Shared Ratatui chrome, theme, and modal primitives for `sisho` TUI tools.
//!
//! This crate provides a minimal, consistent visual shell across `sisho` terminal
//! interfaces without coupling to any application state or event loop:
//! - [`theme`]: [`Theme`] palette and reusable pane/tab/keybinding styles.
//! - [`shell`]: [`ShellLayout`] standard header/body/footer vertical split.
//! - [`header`]: [`HeaderBar`] and [`TabItem`] top bar with optional context and badges.
//! - [`footer`]: [`FooterBar`], [`KeyHint`], [`StatusLine`], and [`StatusKind`] two-line footer.
//! - [`modal`]: [`centered_rect`], [`ModalFrame`], [`HelpSection`], [`HelpRow`], and [`draw_help_modal`].

pub mod footer;
pub mod header;
pub mod modal;
pub mod shell;
pub mod theme;

pub use footer::{FooterBar, KeyHint, StatusKind, StatusLine};
pub use header::{HeaderBar, TabItem};
pub use modal::{HelpRow, HelpSection, ModalFrame, centered_rect, draw_help_modal};
pub use shell::ShellLayout;
pub use theme::Theme;
