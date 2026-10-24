use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use sisho_run::Status;
use sisho_tools::Verdict;

use crate::model::{FocusedPane, Model, StepSlot};

/// Render the complete sisho-counter interface (Board + Counter).
pub fn draw(f: &mut Frame, model: &Model) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(4)])
        .split(f.area());

    draw_board(f, chunks[0], model);
    draw_counter(f, chunks[1], model);
}

fn draw_board(f: &mut Frame, area: Rect, model: &Model) {
    let board_block = Block::default()
        .borders(Borders::ALL)
        .title(" Sisho Board (Project Status) ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner = board_block.inner(area);
    f.render_widget(board_block, area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(inner);

    draw_left_column(f, columns[0], model);
    draw_right_column(f, columns[1], model);
}

fn draw_left_column(f: &mut Frame, area: Rect, model: &Model) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    draw_tools_and_extensions(f, rows[0], model);
    draw_broken_links(f, rows[1], model);
}

fn draw_tools_and_extensions(f: &mut Frame, area: Rect, model: &Model) {
    let mut items = Vec::new();

    // Tools
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "Core Tools:",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Yellow),
    )])));

    for tool in &model.tools {
        let (icon, icon_style) = match &tool.verdict {
            Verdict::Fine(_) => ("  ✓ ", Style::default().fg(Color::Green)),
            Verdict::TooOld { .. } => ("  ✗ ", Style::default().fg(Color::Red)),
            Verdict::Unreported { .. } => ("  - ", Style::default().fg(Color::DarkGray)),
        };

        let ver_str = tool.version.as_deref().unwrap_or("not found");
        let note = match &tool.verdict {
            Verdict::Fine(_) => " (fine)",
            Verdict::TooOld { .. } => " (older than needed)",
            Verdict::Unreported { .. } => " (missing)",
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(icon, icon_style),
            Span::styled(tool.name, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(": {ver_str}{note}")),
        ])));
    }

    // Extensions
    items.push(ListItem::new(Line::from("")));
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "Installed Extensions:",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Yellow),
    )])));

    if model.extensions.is_empty() {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "  (none found on PATH)",
            Style::default().fg(Color::DarkGray),
        )])));
    } else {
        for ext in &model.extensions {
            let (icon, style) = if ext.is_active {
                ("  • ", Style::default().fg(Color::Green))
            } else {
                ("  • ", Style::default().fg(Color::DarkGray))
            };

            let mut header_spans = vec![
                Span::styled(icon, style),
                Span::styled(&ext.word, Style::default().add_modifier(Modifier::BOLD)),
            ];
            if let Some(note) = &ext.note {
                header_spans.push(Span::raw(" "));
                header_spans.push(Span::styled(
                    format!("({note})"),
                    Style::default().fg(Color::Yellow),
                ));
            }

            items.push(ListItem::new(Line::from(header_spans)));
            items.push(ListItem::new(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    ext.path.display().to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ])));
        }
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Tools & Extensions "),
    );
    f.render_widget(list, area);
}

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner(tick: usize) -> &'static str {
    SPINNER[tick % SPINNER.len()]
}

fn draw_broken_links(f: &mut Frame, area: Rect, model: &Model) {
    let is_focused = model.focused_pane == FocusedPane::BrokenLinks;
    let broken = model.broken_links();
    let mut items = Vec::new();

    if model.is_checking_links && model.links_report.is_none() {
        let spin = spinner(model.tick);
        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("  {spin} "), Style::default().fg(Color::Cyan)),
            Span::styled(
                "Scanning links in background...",
                Style::default().fg(Color::Yellow),
            ),
        ])));
    } else {
        if let Some(report) = &model.links_report {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!(
                    "Scanned {} files ({} links checked)",
                    report.files_scanned, report.links_checked
                ),
                Style::default().fg(Color::DarkGray),
            )])));
        }

        if broken.is_empty() {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                "✓ No broken links found",
                Style::default().fg(Color::Green),
            )])));
        } else {
            // Scroll window if broken links exceed available area height inside the box.
            let available_height = area.height.saturating_sub(3) as usize;
            let (start_idx, end_idx) = if available_height > 0 && broken.len() > available_height {
                let start = if model.selected_broken_link >= available_height {
                    model.selected_broken_link + 1 - available_height
                } else {
                    0
                };
                let end = (start + available_height).min(broken.len());
                (start, end)
            } else {
                (0, broken.len())
            };

            for (i, link) in broken[start_idx..end_idx].iter().enumerate() {
                let actual_idx = start_idx + i;
                let is_selected = is_focused && actual_idx == model.selected_broken_link;
                let cursor = if is_selected { "> " } else { "  " };
                let at_str = link.at.map(|p| format!(":{}", p.line)).unwrap_or_default();
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        cursor,
                        if is_selected {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled("✗ ", Style::default().fg(Color::Red)),
                    Span::styled(
                        format!("{}{at_str}", link.source.display()),
                        if is_selected {
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                        } else {
                            Style::default().fg(Color::White)
                        },
                    ),
                    Span::raw(format!(" -> {}", link.target)),
                ])));
            }
        }
    }

    let title = if model.is_checking_links {
        " Broken Links (scanning...) ".to_string()
    } else if !broken.is_empty() && is_focused {
        format!(
            " Broken Links ({}/{}) ",
            model.selected_broken_link + 1,
            broken.len()
        )
    } else {
        format!(" Broken Links ({}) ", broken.len())
    };

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .title_style(if is_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            }),
    );
    f.render_widget(list, area);
}

fn draw_right_column(f: &mut Frame, area: Rect, model: &Model) {
    // Number of check command items + top/bottom borders (2 lines).
    let cmd_count = if model.check_slots.is_empty() {
        sisho_tools::check_steps(&model.dir).len()
    } else {
        model.check_slots.len()
    };
    let border_lines = 2;
    let height = (cmd_count as u16) + border_lines;

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Max(height), Constraint::Min(0)])
        .split(area);

    draw_checks(f, rows[0], model);
    draw_output_preview(f, rows[1], model);
}

fn draw_checks(f: &mut Frame, area: Rect, model: &Model) {
    let is_focused = model.focused_pane == FocusedPane::Checks;
    let mut items = Vec::new();

    if model.check_slots.is_empty() {
        if model.is_checking {
            let spin = spinner(model.tick);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("  {spin} "), Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Running checks in background...",
                    Style::default().fg(Color::Yellow),
                ),
            ])));
        } else if model.is_fixing {
            let spin = spinner(model.tick);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("  {spin} "), Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Applying fixes in background...",
                    Style::default().fg(Color::Yellow),
                ),
            ])));
        } else {
            items.push(ListItem::new(Line::from("No checks run yet. Press [c].")));
        }
    } else {
        for (i, slot) in model.check_slots.iter().enumerate() {
            let is_selected = is_focused && i == model.selected_step;
            let cursor = if is_selected { "> " } else { "  " };

            let line = match slot {
                StepSlot::Done(outcome) => {
                    let (status_icon, status_style) = match outcome.status {
                        Status::Passed => ("✓ ", Style::default().fg(Color::Green)),
                        Status::Failed { .. } => (
                            "✗ ",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Status::NotFound => ("- ", Style::default().fg(Color::Yellow)),
                        Status::CouldNotStart { .. } => ("! ", Style::default().fg(Color::Red)),
                        Status::Signalled { .. } => ("! ", Style::default().fg(Color::Magenta)),
                    };

                    let duration_str = format!("({:.2}s)", outcome.duration.as_secs_f64());
                    Line::from(vec![
                        Span::styled(
                            cursor,
                            if is_selected {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default()
                            },
                        ),
                        Span::styled(status_icon, status_style),
                        Span::styled(
                            &outcome.name,
                            if is_selected {
                                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                            } else {
                                Style::default()
                            },
                        ),
                        Span::raw(" "),
                        Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
                    ])
                }
                StepSlot::Running { name } => {
                    let spin = spinner(model.tick.wrapping_add(i));
                    Line::from(vec![
                        Span::styled(
                            cursor,
                            if is_selected {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default()
                            },
                        ),
                        Span::styled(format!("{spin} "), Style::default().fg(Color::Cyan)),
                        Span::styled(
                            name,
                            if is_selected {
                                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                            } else {
                                Style::default()
                            },
                        ),
                        Span::raw(" "),
                        Span::styled("(running...)", Style::default().fg(Color::Yellow)),
                    ])
                }
            };

            items.push(ListItem::new(line));
        }
    }

    let title = if model.is_checking {
        " Integrity & Checks (running...) "
    } else if model.is_fixing {
        " Integrity & Checks (fixing...) "
    } else {
        " Integrity & Checks "
    };

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .title_style(if is_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            }),
    );
    f.render_widget(list, area);
}

fn draw_output_preview(f: &mut Frame, area: Rect, model: &Model) {
    let is_focused = model.focused_pane == FocusedPane::OutputPreview;
    let mut total_lines = 0;

    let content = if let Some(outcome) = model.selected_outcome() {
        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Color::DarkGray)),
            Span::raw(&outcome.command_line),
        ]));

        if !outcome.stdout.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "--- stdout ---",
                Style::default().fg(Color::DarkGray),
            )));
            for l in outcome.stdout.lines() {
                lines.push(Line::from(l));
            }
        }
        if !outcome.stderr.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "--- stderr ---",
                Style::default().fg(Color::Red),
            )));
            for l in outcome.stderr.lines() {
                lines.push(Line::from(Span::styled(l, Style::default().fg(Color::Red))));
            }
        }

        if outcome.stdout.is_empty() && outcome.stderr.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "(no output produced)",
                Style::default().fg(Color::DarkGray),
            )));
        }

        total_lines = lines.len();
        lines
    } else {
        vec![Line::from("No step selected or step is still running.")]
    };

    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_y = model.preview_scroll.min(max_scroll);

    let title = if total_lines > visible_height {
        format!(
            " Step Output Preview (lines {}-{}/{}) ",
            scroll_y + 1,
            (scroll_y + visible_height).min(total_lines),
            total_lines
        )
    } else {
        " Step Output Preview ".to_string()
    };

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let paragraph = Paragraph::new(content)
        .scroll((scroll_y as u16, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title)
                .title_style(if is_focused {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_counter(f: &mut Frame, area: Rect, model: &Model) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Counter (Actions) ")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    if model.confirm_fix {
        lines.push(Line::from(vec![
            Span::styled(
                " Apply fixes across the project? (writes files)  ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "[y] Yes, apply fixes",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                "[n/Esc] Cancel",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        let focus_desc = match model.focused_pane {
            FocusedPane::Checks => "Checks",
            FocusedPane::BrokenLinks => "Links",
            FocusedPane::OutputPreview => "Preview",
        };

        let mut action_spans = vec![
            Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
            Span::raw(format!(" Pane:{focus_desc}   ")),
        ];

        match model.focused_pane {
            FocusedPane::BrokenLinks => {
                if !model.broken_links().is_empty() {
                    action_spans.push(Span::styled(
                        "[Enter/e]",
                        Style::default().fg(Color::Yellow),
                    ));
                    action_spans.push(Span::raw(" Edit   "));
                }
                action_spans.push(Span::styled("[d]", Style::default().fg(Color::Cyan)));
                action_spans.push(Span::raw(" Desk   "));
                action_spans.push(Span::styled("[↑/k ↓/j]", Style::default().fg(Color::Cyan)));
                action_spans.push(Span::raw(" Move   "));
            }
            FocusedPane::Checks => {
                action_spans.push(Span::styled("[↑/k ↓/j]", Style::default().fg(Color::Cyan)));
                action_spans.push(Span::raw(" Move   "));
                action_spans.push(Span::styled("[d]", Style::default().fg(Color::Cyan)));
                action_spans.push(Span::raw(" Desk   "));
            }
            FocusedPane::OutputPreview => {
                action_spans.push(Span::styled("[↑/k ↓/j]", Style::default().fg(Color::Cyan)));
                action_spans.push(Span::raw(" Scroll   "));
                action_spans.push(Span::styled("[d]", Style::default().fg(Color::Cyan)));
                action_spans.push(Span::raw(" Desk   "));
            }
        }

        action_spans.push(Span::styled("[c]", Style::default().fg(Color::Cyan)));
        action_spans.push(Span::raw(" Check   "));
        action_spans.push(Span::styled("[f]", Style::default().fg(Color::Cyan)));
        action_spans.push(Span::raw(" Fix   "));
        action_spans.push(Span::styled("[l]", Style::default().fg(Color::Cyan)));
        action_spans.push(Span::raw(" Links   "));
        action_spans.push(Span::styled("[q/Esc]", Style::default().fg(Color::Cyan)));
        action_spans.push(Span::raw(" Quit"));

        lines.push(Line::from(action_spans));

        if let Some(msg) = &model.message {
            let is_busy = model.is_checking || model.is_checking_links || model.is_fixing;
            let status_prefix = if is_busy {
                format!("status: {} ", spinner(model.tick))
            } else {
                "status: ".to_string()
            };
            lines.push(Line::from(vec![
                Span::styled(
                    status_prefix,
                    if is_busy {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(msg, Style::default().fg(Color::White)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}
