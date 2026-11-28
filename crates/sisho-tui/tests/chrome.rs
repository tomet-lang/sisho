//! Unit and `TestBackend` rendering tests for `sisho-tui`.

use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
    widgets::Paragraph,
};
use sisho_tui::{
    FooterBar, HeaderBar, HelpRow, HelpSection, KeyHint, ModalFrame, ShellLayout, StatusKind,
    StatusLine, TabItem, Theme, centered_rect, draw_help_modal,
};

fn render_with(width: u16, height: u16, draw_fn: impl FnOnce(&mut ratatui::Frame)) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("create test terminal");
    terminal.draw(draw_fn).expect("draw frame");
    terminal.backend().buffer().clone()
}

fn row_text(buffer: &Buffer, y: u16) -> String {
    let area = buffer.area();
    (0..area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect::<String>()
}

fn buffer_text(buffer: &Buffer) -> String {
    (0..buffer.area().height)
        .map(|y| row_text(buffer, y))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn theme_defaults_and_styles_match_sisho_palette() {
    let theme = Theme::default();
    assert_eq!(theme, Theme::sisho());
    assert_eq!(theme.primary, Color::Rgb(95, 130, 255));
    assert_eq!(theme.secondary, Color::Rgb(140, 165, 255));
    assert_eq!(theme.active_tab_fg, Color::Black);
    assert_eq!(theme.inactive_tab_fg, Color::Gray);
    assert_eq!(theme.border_focused, Color::Rgb(95, 130, 255));
    assert_eq!(theme.border_unfocused, Color::DarkGray);
    assert_eq!(theme.muted, Color::DarkGray);
    assert_eq!(theme.key_hint, Color::Yellow);
    assert_eq!(theme.info, Color::Rgb(95, 130, 255));
    assert_eq!(theme.success, Color::Green);
    assert_eq!(theme.warning, Color::Yellow);
    assert_eq!(theme.error, Color::Red);

    assert_eq!(theme.focused_border_style().fg, Some(theme.border_focused));
    assert_eq!(
        theme.unfocused_border_style().fg,
        Some(theme.border_unfocused)
    );
    let active = theme.active_tab_style();
    assert_eq!(active.fg, Some(Color::Black));
    assert_eq!(active.bg, Some(Color::Rgb(95, 130, 255)));
    assert!(active.add_modifier.contains(Modifier::BOLD));

    let inactive = theme.inactive_tab_style();
    assert_eq!(inactive.fg, Some(Color::Gray));

    let key = theme.key_span("[q]");
    assert_eq!(key.style.fg, Some(Color::Yellow));
    assert!(key.style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn theme_pane_block_distinguishes_focused_and_unfocused() {
    let theme = Theme::sisho();
    let buffer = render_with(40, 8, |frame| {
        let left = Rect::new(0, 0, 20, 8);
        let right = Rect::new(20, 0, 20, 8);
        frame.render_widget(theme.pane_block(" Notes ", true), left);
        frame.render_widget(theme.pane_block(" Detail ", false), right);
    });

    // Top-left corner of focused block uses primary border color.
    assert_eq!(buffer[(0, 0)].fg, theme.border_focused);
    // Top-left corner of unfocused block uses unfocused border color.
    assert_eq!(buffer[(20, 0)].fg, theme.border_unfocused);

    let top_row = row_text(&buffer, 0);
    assert!(top_row.contains(" Notes "));
    assert!(top_row.contains(" Detail "));
}

#[test]
fn shell_layout_splits_header_body_footer() {
    let area = Rect::new(0, 0, 100, 30);
    let shell = ShellLayout::split(area);

    assert_eq!(shell.header, Rect::new(0, 0, 100, 3));
    assert_eq!(shell.body, Rect::new(0, 3, 100, 25));
    assert_eq!(shell.footer, Rect::new(0, 28, 100, 2));

    // Even on a minimal height screen, body preserves Min(10).
    let compact = ShellLayout::split(Rect::new(0, 0, 80, 15));
    assert_eq!(compact.header.height, 3);
    assert_eq!(compact.body.height, 10);
    assert_eq!(compact.footer.height, 2);
}

#[test]
fn header_bar_renders_tabs_badges_and_context() {
    let theme = Theme::sisho();
    let buffer = render_with(100, 3, |frame| {
        let area = frame.area();
        HeaderBar::new("sisho-history")
            .context("/vault/notes")
            .tabs([
                TabItem::new("1", "Notes", true),
                TabItem::new("2", "Trash", false).badge(4),
                TabItem::new("3", "Moments", false).badge(12),
            ])
            .render(frame, area, &theme);
    });

    let content = row_text(&buffer, 1);
    assert!(content.contains("sisho-history"));
    assert!(content.contains("· /vault/notes"));
    assert!(content.contains("[1] Notes"));
    assert!(content.contains("[2] Trash (4)"));
    assert!(content.contains("[3] Moments (12)"));

    // Border uses unfocused border style.
    assert_eq!(buffer[(0, 0)].fg, theme.border_unfocused);

    // Active tab cell carries primary background and black bold foreground.
    let active_x = content.find("[1] Notes").expect("active tab in header") as u16;
    assert_eq!(buffer[(active_x, 1)].bg, theme.primary);
    assert_eq!(buffer[(active_x, 1)].fg, theme.active_tab_fg);
}

#[test]
fn header_bar_renders_cleanly_with_zero_tabs_and_no_context() {
    let theme = Theme::sisho();
    let buffer = render_with(40, 3, |frame| {
        HeaderBar::new("sisho-desk").render(frame, frame.area(), &theme);
    });

    let content = row_text(&buffer, 1);
    assert!(content.contains("sisho-desk"));
    assert!(!content.contains('·'));
    assert!(!content.contains('['));
}

#[test]
fn footer_bar_renders_structured_hints_and_status_variants() {
    let theme = Theme::sisho();

    let hints = [
        KeyHint::new("1-3", "Views"),
        KeyHint::new("?", "Help"),
        KeyHint::new("q", "Quit"),
    ];

    let info_buf = render_with(80, 2, |frame| {
        FooterBar::new()
            .hints(&hints)
            .summary("Should be overridden")
            .status(Some(StatusLine::new(StatusKind::Info, "Loading notes")))
            .render(frame, frame.area(), &theme);
    });
    assert!(row_text(&info_buf, 0).contains("[1-3] Views · [?] Help · [q] Quit"));
    let line1 = row_text(&info_buf, 1);
    assert!(line1.starts_with("ℹ Loading notes"));
    assert_eq!(info_buf[(0, 1)].fg, theme.info);

    let success_buf = render_with(80, 2, |frame| {
        FooterBar::new()
            .hint_text("[q] Quit")
            .status(Some(StatusLine::success("Saved")))
            .render(frame, frame.area(), &theme);
    });
    assert!(row_text(&success_buf, 0).starts_with("[q] Quit"));
    assert!(row_text(&success_buf, 1).starts_with("✓ Saved"));
    assert_eq!(success_buf[(0, 1)].fg, theme.success);

    let error_buf = render_with(80, 2, |frame| {
        FooterBar::new()
            .status(Some(StatusLine::error("Failed to restore")))
            .render(frame, frame.area(), &theme);
    });
    assert!(row_text(&error_buf, 1).starts_with("! Failed to restore"));
    assert_eq!(error_buf[(0, 1)].fg, theme.error);
    assert!(error_buf[(0, 1)].modifier.contains(Modifier::BOLD));
}

#[test]
fn footer_bar_falls_back_to_muted_summary_when_no_status() {
    let theme = Theme::sisho();
    let buffer = render_with(60, 2, |frame| {
        FooterBar::new()
            .hint_text("[?] Help · [q] Quit")
            .summary("Ready · 12 notes")
            .render(frame, frame.area(), &theme);
    });

    let summary_row = row_text(&buffer, 1);
    assert!(summary_row.starts_with("Ready · 12 notes"));
    assert_eq!(buffer[(0, 1)].fg, theme.muted);
}

#[test]
fn centered_rect_centers_and_clamps_percentages() {
    let screen = Rect::new(0, 0, 100, 50);
    let modal = centered_rect(60, 40, screen);
    assert_eq!(modal, Rect::new(20, 15, 60, 20));

    let full = centered_rect(150, 120, screen);
    assert_eq!(full, screen);
}

#[test]
fn modal_frame_clears_background_and_returns_inner_rect() {
    let theme = Theme::sisho();
    let mut returned_inner = Rect::default();
    let buffer = render_with(60, 20, |frame| {
        let area = frame.area();
        // Paint background noise that Clear must wipe inside the modal.
        let bg_line = "X".repeat(area.width as usize);
        let bg = vec![bg_line.as_str(); area.height as usize].join("\n");
        frame.render_widget(Paragraph::new(bg), area);

        returned_inner = ModalFrame::new(" Confirm ", 50, 50)
            .border_color(theme.warning)
            .render(frame, area, &theme);
        frame.render_widget(Paragraph::new("Proceed?"), returned_inner);
    });

    assert_eq!(returned_inner, Rect::new(16, 6, 28, 8));
    // Border uses overridden color.
    assert_eq!(buffer[(15, 5)].fg, theme.warning);
    let text = buffer_text(&buffer);
    assert!(text.contains(" Confirm "));
    assert!(text.contains("Proceed?"));
    // Interior of modal should no longer contain background 'X'.
    assert!(!row_text(&buffer, 8).contains("XXXXProceed?"));
}

#[test]
fn draw_help_modal_renders_sections_rows_and_footer_hint() {
    let theme = Theme::sisho();
    let sections = [
        HelpSection::new(
            "Navigation",
            [
                HelpRow::new("j / k", "Move selection"),
                HelpRow::new("Tab", "Cycle pane focus"),
            ],
        ),
        HelpSection::new("Actions", [HelpRow::new("q", "Quit application")]),
    ];

    let buffer = render_with(90, 24, |frame| {
        draw_help_modal(
            frame,
            frame.area(),
            " Help — sisho ",
            &sections,
            Some("Press [?] or [Esc] to close."),
            &theme,
        );
    });

    let rendered = buffer_text(&buffer);
    assert!(rendered.contains(" Help — sisho "));
    assert!(rendered.contains("Navigation"));
    assert!(rendered.contains("j / k"));
    assert!(rendered.contains("Move selection"));
    assert!(rendered.contains("Tab"));
    assert!(rendered.contains("Cycle pane focus"));
    assert!(rendered.contains("Actions"));
    assert!(rendered.contains("Quit application"));
    assert!(rendered.contains("Press [?] or [Esc] to close."));
}
