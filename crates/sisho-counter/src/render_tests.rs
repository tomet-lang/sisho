use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use sisho_run::{Outcome, Status};
use sisho_tools::{Verdict, Version};

use crate::model::{ExtensionItem, Model, ToolStatus};
use crate::ui;

fn create_mock_model() -> Model {
    let (sender, receiver) = std::sync::mpsc::channel();
    Model {
        dir: PathBuf::from("."),
        tools: vec![
            ToolStatus {
                name: "tomet",
                version: Some("0.2.0".to_string()),
                verdict: Verdict::Fine(Version::new(0, 2, 0)),
            },
            ToolStatus {
                name: "twrit",
                version: Some("0.2.0".to_string()),
                verdict: Verdict::Fine(Version::new(0, 2, 0)),
            },
            ToolStatus {
                name: "tandr",
                version: None,
                verdict: Verdict::Unreported {
                    needs: Version::new(0, 2, 0),
                },
            },
        ],
        extensions: vec![ExtensionItem {
            word: "desk".to_string(),
            path: PathBuf::from("/path/to/sisho-desk"),
            is_active: true,
            note: None,
        }],
        check_outcomes: vec![
            Outcome {
                name: "tomet check".to_string(),
                command_line: "tomet check .".to_string(),
                status: Status::Passed,
                stdout: "all passed\n".to_string(),
                stderr: String::new(),
                duration: std::time::Duration::from_millis(15),
            },
            Outcome {
                name: "twrit check".to_string(),
                command_line: "twrit check .".to_string(),
                status: Status::Failed { code: 1 },
                stdout: String::new(),
                stderr: "rule error in spec\n".to_string(),
                duration: std::time::Duration::from_millis(90),
            },
        ],
        links_report: None,
        selected_step: 0,
        confirm_fix: false,
        message: None,
        is_checking: false,
        is_checking_links: false,
        is_fixing: false,
        tick: 0,
        sender,
        receiver,
    }
}

#[test]
fn renders_normal_board_and_counter() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let model = create_mock_model();
    terminal.draw(|f| ui::draw(f, &model)).unwrap();

    let view = terminal.backend().to_string();
    assert!(view.contains("Sisho Board (Project Status)"), "{view}");
    assert!(view.contains("Tools & Extensions"), "{view}");
    assert!(view.contains("tomet: 0.2.0 (fine)"), "{view}");
    assert!(view.contains("• desk"), "{view}");
    assert!(view.contains("/path/to/sisho-desk"), "{view}");
    assert!(view.contains("Integrity & Checks"), "{view}");
    assert!(view.contains("tomet check"), "{view}");
    assert!(view.contains("Counter (Actions)"), "{view}");
    assert!(view.contains("[c] Check"), "{view}");
    assert!(view.contains("[f] Fix"), "{view}");
}

#[test]
fn renders_confirm_fix_prompt() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = create_mock_model();
    model.confirm_fix = true;
    terminal.draw(|f| ui::draw(f, &model)).unwrap();

    let view = terminal.backend().to_string();
    assert!(
        view.contains("Apply fixes across the project? (writes files)"),
        "{view}"
    );
    assert!(view.contains("[y] Yes, apply fixes"), "{view}");
    assert!(view.contains("[n/Esc] Cancel"), "{view}");
}

#[test]
fn renders_broken_links_list() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = create_mock_model();
    model.links_report = Some(sisho_vault::links::Report {
        files_scanned: 10,
        links_checked: 42,
        broken: vec![sisho_vault::links::BrokenLink {
            source: PathBuf::from("docs/intro.tmt"),
            target: "missing.tmt".to_string(),
            at: Some(sisho_vault::links::Position {
                line: 15,
                column: 1,
            }),
        }],
        unreadable: Vec::new(),
    });

    terminal.draw(|f| ui::draw(f, &model)).unwrap();

    let view = terminal.backend().to_string();
    assert!(view.contains("Broken Links (1)"), "{view}");
    assert!(view.contains("docs/intro.tmt:15 -> missing.tmt"), "{view}");
}

#[test]
fn renders_in_progress_checks_and_links() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = create_mock_model();
    model.is_checking = true;
    model.is_checking_links = true;
    model.check_outcomes.clear();
    model.links_report = None;
    model.message = Some("Running checks...".to_string());

    terminal.draw(|f| ui::draw(f, &model)).unwrap();

    let view = terminal.backend().to_string();
    assert!(view.contains("Integrity & Checks (running...)"), "{view}");
    assert!(view.contains("Running checks in background..."), "{view}");
    assert!(view.contains("Broken Links (scanning...)"), "{view}");
    assert!(view.contains("Scanning links in background..."), "{view}");
    assert!(view.contains("status: ⠋ Running checks..."), "{view}");
}

#[test]
fn renders_in_progress_fixing() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = create_mock_model();
    model.is_fixing = true;
    model.check_outcomes.clear();
    model.message = Some("Applying fixes...".to_string());

    terminal.draw(|f| ui::draw(f, &model)).unwrap();

    let view = terminal.backend().to_string();
    assert!(view.contains("Integrity & Checks (fixing...)"), "{view}");
    assert!(view.contains("Applying fixes in background..."), "{view}");
    assert!(view.contains("status: ⠋ Applying fixes..."), "{view}");
}
