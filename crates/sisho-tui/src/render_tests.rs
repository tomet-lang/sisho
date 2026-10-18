//! What each tab shows, drawn into an in-memory terminal. No real terminal is
//! needed: the buffer the widgets are drawn into is read back as text.

use std::fs;
use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app::{ActiveTab, App};
use crate::ui;

/// A vault in a fresh temporary place, removed when dropped.
struct Vault(PathBuf);

impl Vault {
    fn new(name: &str) -> Self {
        // The time, not the process id: `process-door` keeps the whole
        // `process` module out of every crate but `sisho-run`.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("sisho-tui-{nanos}-{name}"));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn with_documents(name: &str) -> Self {
        let vault = Self::new(name);
        vault.write("default.config.tmt", "@kind(config)\n");
        vault.write("notes/alpha.tmt", "@kind(note)\n\n=[ Alpha ]\n\nFirst.\n");
        vault.write("legacy/old.md", "# Legacy\n\nAn old page.\n");
        vault
    }
}

impl Drop for Vault {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The screen as text, one line per row.
fn screen_of(width: u16, height: u16, draw: impl FnOnce(&mut ratatui::Frame)) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(draw).unwrap();
    let buffer = terminal.backend().buffer();
    buffer
        .content()
        .chunks(usize::from(width))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn app_on(vault: &Vault) -> App {
    App::new(vault.0.clone(), None)
}

fn draw_tab(app: &mut App, tab: ActiveTab, width: u16, height: u16) -> String {
    app.active_tab = tab;
    app.refresh_status();
    screen_of(width, height, |frame| ui::draw(frame, app))
}

#[test]
fn the_explorer_lists_the_files_of_the_vault() {
    let vault = Vault::with_documents("explorer");
    let mut app = app_on(&vault);
    let screen = draw_tab(&mut app, ActiveTab::Explorer, 120, 40);

    assert!(screen.contains("Tomet Workbench TUI"), "{screen}");
    assert!(screen.contains("Workspace File Explorer"), "{screen}");
    assert!(screen.contains("alpha.tmt"), "{screen}");
    assert!(screen.contains("old.md"), "{screen}");
}

#[test]
fn every_tab_has_its_own_panes() {
    let vault = Vault::with_documents("tabs");
    let mut app = app_on(&vault);

    for (tab, title) in [
        (ActiveTab::Explorer, "Workspace File Explorer"),
        (ActiveTab::Migration, "Migration Tree (.md -> .tmt)"),
        (ActiveTab::BatchMeta, "Tomet Files (.tmt)"),
        (ActiveTab::StructuralGrep, "Structural Search Matches"),
    ] {
        let screen = draw_tab(&mut app, tab, 120, 40);
        assert!(screen.contains(title), "{title:?} is missing:\n{screen}");
        // The tab bar is on every screen.
        assert!(screen.contains("1. Workspace Explorer"), "{screen}");
        assert!(screen.contains("Quick Actions & Controls"), "{screen}");
    }
}

#[test]
fn the_migration_tab_finds_the_markdown_to_migrate() {
    let vault = Vault::with_documents("migration");
    let mut app = app_on(&vault);
    let screen = draw_tab(&mut app, ActiveTab::Migration, 120, 40);

    assert!(screen.contains("old.md"), "{screen}");
}

#[test]
fn the_batch_tab_lists_the_tomet_documents() {
    let vault = Vault::with_documents("batch");
    let mut app = app_on(&vault);
    let screen = draw_tab(&mut app, ActiveTab::BatchMeta, 120, 40);

    assert!(screen.contains("alpha.tmt"), "{screen}");
    assert!(screen.contains("default.config.tmt"), "{screen}");
}

#[test]
fn an_empty_vault_still_draws_every_tab() {
    let vault = Vault::new("empty");
    let mut app = app_on(&vault);

    for tab in [
        ActiveTab::Explorer,
        ActiveTab::Migration,
        ActiveTab::BatchMeta,
        ActiveTab::StructuralGrep,
    ] {
        let screen = draw_tab(&mut app, tab, 120, 40);
        assert!(screen.contains("Quick Actions & Controls"), "{screen}");
    }
}

#[test]
fn a_small_terminal_does_not_panic() {
    let vault = Vault::with_documents("small");
    let mut app = app_on(&vault);

    for (width, height) in [(1, 1), (10, 4), (40, 10), (80, 24)] {
        for tab in [
            ActiveTab::Explorer,
            ActiveTab::Migration,
            ActiveTab::BatchMeta,
            ActiveTab::StructuralGrep,
        ] {
            draw_tab(&mut app, tab, width, height);
        }
    }
}

#[test]
fn the_loading_screen_says_what_it_is_doing() {
    let screen = screen_of(80, 12, |frame| {
        ui::draw_loading(frame, 0, "Scanning workspace...")
    });
    assert!(screen.contains("Scanning workspace..."), "{screen}");
}
