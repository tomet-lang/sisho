use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use sisho_run::Status;
use sisho_tools::Verdict;

use crate::model::Model;

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
            for link in broken {
                let at_str = link.at.map(|p| format!(":{}", p.line)).unwrap_or_default();
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(Color::Red)),
                    Span::styled(
                        format!("{}{at_str}", link.source.display()),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw(format!(" -> {}", link.target)),
                ])));
            }
        }
    }

    let title = if model.is_checking_links {
        " Broken Links (scanning...) ".to_string()
    } else {
        format!(" Broken Links ({}) ", broken.len())
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(list, area);
}

fn draw_right_column(f: &mut Frame, area: Rect, model: &Model) {
    // Number of check command items + top/bottom borders (2 lines).
    let cmd_count = if model.check_outcomes.is_empty() {
        sisho_tools::check_steps(&model.dir).len()
    } else {
        model.check_outcomes.len()
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
    let mut items = Vec::new();

    if model.check_outcomes.is_empty() {
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
        for (i, outcome) in model.check_outcomes.iter().enumerate() {
            let is_selected = i == model.selected_step;
            let cursor = if is_selected { "> " } else { "  " };

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
            let line = Line::from(vec![
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
            ]);

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

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(list, area);
}

fn draw_output_preview(f: &mut Frame, area: Rect, model: &Model) {
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
            for l in outcome.stdout.lines().take(15) {
                lines.push(Line::from(l));
            }
        }
        if !outcome.stderr.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "--- stderr ---",
                Style::default().fg(Color::Red),
            )));
            for l in outcome.stderr.lines().take(15) {
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

        lines
    } else {
        vec![Line::from("No step selected.")]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Step Output Preview "),
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
        lines.push(Line::from(vec![
            Span::styled("[c]", Style::default().fg(Color::Cyan)),
            Span::raw(" Check   "),
            Span::styled("[f]", Style::default().fg(Color::Cyan)),
            Span::raw(" Fix (ask)   "),
            Span::styled("[l]", Style::default().fg(Color::Cyan)),
            Span::raw(" Check Links   "),
            Span::styled("[d]", Style::default().fg(Color::Cyan)),
            Span::raw(" Open Desk   "),
            Span::styled("[↑/k ↓/j]", Style::default().fg(Color::Cyan)),
            Span::raw(" Step   "),
            Span::styled("[q/Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" Quit"),
        ]));

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
