use std::path::PathBuf;
use std::time::Duration;

use sisho_run::{Outcome, Status};
use sisho_vault::links::{BrokenLink, Position, Report};

use crate::model::{ExtensionItem, FocusedPane, Model, StepSlot, WorkerMessage};

fn create_test_model() -> Model {
    let (sender, receiver) = std::sync::mpsc::channel();
    Model {
        dir: PathBuf::from("."),
        tools: Vec::new(),
        extensions: vec![ExtensionItem {
            word: "desk".to_string(),
            path: PathBuf::from("/bin/sisho-desk"),
            is_active: true,
            note: None,
        }],
        check_slots: vec![
            StepSlot::Running {
                name: "step 1".to_string(),
            },
            StepSlot::Running {
                name: "step 2".to_string(),
            },
        ],
        check_outcomes: Vec::new(),
        links_report: None,
        selected_step: 0,
        selected_broken_link: 0,
        preview_scroll: 0,
        focused_pane: FocusedPane::Checks,
        confirm_fix: false,
        message: None,
        is_checking: true,
        is_checking_links: false,
        is_fixing: false,
        tick: 0,
        sender,
        receiver,
    }
}

#[test]
fn individual_step_completion_updates_only_its_slot() {
    let mut model = create_test_model();
    assert_eq!(model.selected_outcome(), None);

    let outcome1 = Outcome {
        name: "step 1".to_string(),
        command_line: "step 1".to_string(),
        status: Status::Passed,
        stdout: "ok".to_string(),
        stderr: String::new(),
        duration: Duration::from_millis(50),
    };

    model
        .sender
        .send(WorkerMessage::CheckStepDone {
            index: 0,
            outcome: outcome1.clone(),
        })
        .unwrap();

    model.poll_background();

    // Step 0 is now Done, step 1 is still Running
    assert_eq!(model.check_slots[0], StepSlot::Done(outcome1.clone()));
    assert_eq!(
        model.check_slots[1],
        StepSlot::Running {
            name: "step 2".to_string()
        }
    );
    assert_eq!(model.selected_outcome(), Some(&outcome1));

    let outcome2 = Outcome {
        name: "step 2".to_string(),
        command_line: "step 2".to_string(),
        status: Status::Failed { code: 1 },
        stdout: String::new(),
        stderr: "fail".to_string(),
        duration: Duration::from_millis(100),
    };

    model
        .sender
        .send(WorkerMessage::CheckStepDone {
            index: 1,
            outcome: outcome2.clone(),
        })
        .unwrap();

    model.poll_background();

    assert_eq!(model.check_slots[1], StepSlot::Done(outcome2.clone()));
    model.move_check_selection_down();
    assert_eq!(model.selected_outcome(), Some(&outcome2));
}

#[test]
fn pane_focus_cycling_and_delegated_navigation() {
    let mut model = create_test_model();
    model.links_report = Some(Report {
        files_scanned: 2,
        links_checked: 2,
        broken: vec![
            BrokenLink {
                source: PathBuf::from("a.tmt"),
                target: "b.tmt".to_string(),
                at: Some(Position { line: 1, column: 1 }),
            },
            BrokenLink {
                source: PathBuf::from("c.tmt"),
                target: "d.tmt".to_string(),
                at: Some(Position { line: 2, column: 1 }),
            },
        ],
        unreadable: Vec::new(),
    });

    // In Checks pane
    assert_eq!(model.focused_pane, FocusedPane::Checks);
    assert_eq!(model.selected_step, 0);
    model.move_selection_down();
    assert_eq!(model.selected_step, 1);
    model.move_selection_up();
    assert_eq!(model.selected_step, 0);

    // Switch to BrokenLinks
    model.next_pane();
    assert_eq!(model.focused_pane, FocusedPane::BrokenLinks);
    assert_eq!(model.selected_broken_link, 0);
    model.move_selection_down();
    assert_eq!(model.selected_broken_link, 1);
    model.move_selection_down(); // clamped
    assert_eq!(model.selected_broken_link, 1);
    model.move_selection_up();
    assert_eq!(model.selected_broken_link, 0);

    // Switch to OutputPreview
    model.next_pane();
    assert_eq!(model.focused_pane, FocusedPane::OutputPreview);
    assert_eq!(model.preview_scroll, 0);
    model.move_selection_down();
    assert_eq!(model.preview_scroll, 1);
    model.scroll_preview_down(10);
    assert_eq!(model.preview_scroll, 11);
    model.scroll_preview_up(5);
    assert_eq!(model.preview_scroll, 6);

    // Switch back to Checks
    model.next_pane();
    assert_eq!(model.focused_pane, FocusedPane::Checks);

    // Moving step resets preview_scroll
    model.move_selection_down();
    assert_eq!(model.preview_scroll, 0);
}
