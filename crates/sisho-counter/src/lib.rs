//! The operating dashboard and counter for sisho.
//!
//! Two-tier design:
//! - **Board**: Status of tools, extensions, integrity check outcomes, and broken links.
//! - **Counter**: Action prompt bar to run check, fix, check-links, or open desk.

pub mod model;
pub mod screen;
pub mod ui;

#[cfg(test)]
mod render_tests;

use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

pub use model::{ExtensionItem, Model};
pub use screen::Screen;

/// Run the interactive counter TUI.
pub fn run(dir: PathBuf, extensions: Vec<ExtensionItem>) -> anyhow::Result<()> {
    let mut screen = Screen::enter()?;
    let mut model = Model::new(dir, extensions);

    loop {
        model.poll_background();
        screen.terminal.draw(|f| ui::draw(f, &model))?;

        if event::poll(Duration::from_millis(80))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };

            let is_ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'));
            if is_ctrl_c {
                break;
            }

            if model.confirm_fix {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        model.run_fixes();
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        model.confirm_fix = false;
                        model.message = Some("Fix cancelled.".to_string());
                    }
                    _ => {}
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    break;
                }
                KeyCode::Char('c') => {
                    model.run_checks();
                }
                KeyCode::Char('f') => {
                    if model.is_fixing || model.is_checking {
                        model.message = Some("Operations already in progress.".to_string());
                    } else {
                        model.confirm_fix = true;
                    }
                }
                KeyCode::Char('l') => {
                    model.run_check_links();
                }
                KeyCode::Char('d') => {
                    let has_desk = model
                        .extensions
                        .iter()
                        .any(|e| e.word == "desk" && e.is_active);
                    if has_desk {
                        let dir_arg = model.dir.clone();
                        screen.execute_handover(|| {
                            let _ = sisho_run::Step::new("desk", "sisho-desk")
                                .arg(&dir_arg)
                                .run_inherited();
                        })?;
                        model.refresh_tools();
                        model.run_checks();
                    } else {
                        model.message =
                            Some("sisho-desk extension is not installed on PATH".to_string());
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    model.move_selection_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    model.move_selection_down();
                }
                _ => {}
            }
        }
    }

    Ok(())
}
