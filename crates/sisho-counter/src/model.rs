use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use sisho_run::{Outcome, run_parallel_streaming};
use sisho_tools::{Tool, Verdict, check_steps, fix_steps};
use sisho_vault::links::{BrokenLink, Report, check as check_links};

/// An extension presented on the Board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionItem {
    pub word: String,
    pub path: PathBuf,
    pub is_active: bool,
    pub note: Option<String>,
}

/// Status of a core tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: &'static str,
    pub version: Option<String>,
    pub verdict: Verdict,
}

/// A slot representing a check step in progress or completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepSlot {
    Running { name: String },
    Done(Outcome),
}

impl StepSlot {
    pub fn name(&self) -> &str {
        match self {
            Self::Running { name } => name,
            Self::Done(outcome) => &outcome.name,
        }
    }

    pub fn outcome(&self) -> Option<&Outcome> {
        match self {
            Self::Done(outcome) => Some(outcome),
            Self::Running { .. } => None,
        }
    }
}

/// The currently focused pane on the Board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    #[default]
    Checks,
    BrokenLinks,
    OutputPreview,
}

impl FocusedPane {
    pub fn next(self) -> Self {
        match self {
            Self::Checks => Self::BrokenLinks,
            Self::BrokenLinks => Self::OutputPreview,
            Self::OutputPreview => Self::Checks,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Checks => Self::OutputPreview,
            Self::BrokenLinks => Self::Checks,
            Self::OutputPreview => Self::BrokenLinks,
        }
    }
}

/// Messages sent from background threads to the model.
pub enum WorkerMessage {
    CheckStepDone { index: usize, outcome: Outcome },
    ChecksDone(Vec<Outcome>),
    LinksDone(Result<Report, String>),
    FixStepDone { index: usize, outcome: Outcome },
    FixesDone(Vec<Outcome>),
}

/// The state of the counter and board.
pub struct Model {
    pub dir: PathBuf,
    pub tools: Vec<ToolStatus>,
    pub extensions: Vec<ExtensionItem>,
    pub check_slots: Vec<StepSlot>,
    pub check_outcomes: Vec<Outcome>,
    pub links_report: Option<Report>,
    pub selected_step: usize,
    pub selected_broken_link: usize,
    pub preview_scroll: usize,
    pub focused_pane: FocusedPane,
    pub confirm_fix: bool,
    pub message: Option<String>,
    pub is_checking: bool,
    pub is_checking_links: bool,
    pub is_fixing: bool,
    pub tick: usize,
    pub sender: Sender<WorkerMessage>,
    pub receiver: Receiver<WorkerMessage>,
}

impl Model {
    pub fn new(dir: PathBuf, extensions: Vec<ExtensionItem>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let steps = check_steps(&dir);
        let check_slots = steps
            .iter()
            .map(|s| StepSlot::Running {
                name: s.name().to_string(),
            })
            .collect();

        let mut model = Self {
            dir,
            tools: Vec::new(),
            extensions,
            check_slots,
            check_outcomes: Vec::new(),
            links_report: None,
            selected_step: 0,
            selected_broken_link: 0,
            preview_scroll: 0,
            focused_pane: FocusedPane::default(),
            confirm_fix: false,
            message: Some("Running checks...".to_string()),
            is_checking: false,
            is_checking_links: false,
            is_fixing: false,
            tick: 0,
            sender,
            receiver,
        };
        model.refresh_tools();
        model.run_check_links();
        model.run_checks();
        model
    }

    /// Refresh tools information and version status synchronously.
    pub fn refresh_tools(&mut self) {
        let mut statuses = Vec::new();
        for tool in Tool::ALL {
            let step = tool.version_step();
            let outcome = step.run();
            let reported = outcome
                .is_pass()
                .then(|| outcome.stdout.lines().next())
                .flatten();
            let verdict = tool.judge(reported);
            let version = reported.map(|s| s.to_string());
            statuses.push(ToolStatus {
                name: tool.name(),
                version,
                verdict,
            });
        }
        self.tools = statuses;
    }

    /// Run the check steps suite across the project in a background thread.
    pub fn run_checks(&mut self) {
        if self.is_checking || self.is_fixing {
            return;
        }
        self.is_checking = true;
        let steps = check_steps(&self.dir);
        self.check_slots = steps
            .iter()
            .map(|s| StepSlot::Running {
                name: s.name().to_string(),
            })
            .collect();
        self.check_outcomes.clear();
        self.preview_scroll = 0;
        self.message = Some("Running checks...".to_string());

        let dir = self.dir.clone();
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let steps = check_steps(&dir);
            let jobs = steps.iter().map(sisho_run::Job::step).collect();
            let sender_clone = sender.clone();
            let outcomes = run_parallel_streaming(jobs, move |index, outcome| {
                let _ = sender_clone.send(WorkerMessage::CheckStepDone { index, outcome });
            });
            let _ = sender.send(WorkerMessage::ChecksDone(outcomes));
        });
    }

    /// Run the fix steps suite across the project in a background thread.
    pub fn run_fixes(&mut self) {
        if self.is_fixing || self.is_checking {
            return;
        }
        self.is_fixing = true;
        self.confirm_fix = false;
        let steps = fix_steps(&self.dir);
        self.check_slots = steps
            .iter()
            .map(|s| StepSlot::Running {
                name: s.name().to_string(),
            })
            .collect();
        self.check_outcomes.clear();
        self.preview_scroll = 0;
        self.message = Some("Applying fixes...".to_string());

        let dir = self.dir.clone();
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let steps = fix_steps(&dir);
            let mut outcomes = Vec::new();
            for (index, step) in steps.into_iter().enumerate() {
                let outcome = step.run();
                let _ = sender.send(WorkerMessage::FixStepDone {
                    index,
                    outcome: outcome.clone(),
                });
                outcomes.push(outcome);
            }
            let _ = sender.send(WorkerMessage::FixesDone(outcomes));
        });
    }

    /// Run check-links verification on the project directory in a background thread.
    pub fn run_check_links(&mut self) {
        if self.is_checking_links {
            return;
        }
        self.is_checking_links = true;
        let dir = self.dir.clone();
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let cache_path = sisho_vault::links::default_cache_path();
            let res = check_links(&dir, &cache_path).map_err(|e| e.to_string());
            let _ = sender.send(WorkerMessage::LinksDone(res));
        });
    }

    /// Poll any messages from background worker threads and update model state.
    pub fn poll_background(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                WorkerMessage::CheckStepDone { index, outcome } => {
                    if index < self.check_slots.len() {
                        self.check_slots[index] = StepSlot::Done(outcome);
                    }
                }
                WorkerMessage::ChecksDone(outcomes) => {
                    self.check_outcomes = outcomes.clone();
                    self.check_slots = outcomes.into_iter().map(StepSlot::Done).collect();
                    self.is_checking = false;
                    if self.selected_step >= self.check_slots.len() {
                        self.selected_step = 0;
                    }
                    self.message = Some("Checks completed.".to_string());
                }
                WorkerMessage::FixStepDone { index, outcome } => {
                    if index < self.check_slots.len() {
                        self.check_slots[index] = StepSlot::Done(outcome);
                    }
                }
                WorkerMessage::FixesDone(outcomes) => {
                    self.check_outcomes = outcomes.clone();
                    self.check_slots = outcomes.into_iter().map(StepSlot::Done).collect();
                    self.is_fixing = false;
                    if self.selected_step >= self.check_slots.len() {
                        self.selected_step = 0;
                    }
                    self.message = Some("Fixes applied. Run check to verify.".to_string());
                }
                WorkerMessage::LinksDone(res) => {
                    self.is_checking_links = false;
                    match res {
                        Ok(report) => {
                            self.links_report = Some(report);
                            if self.selected_broken_link >= self.broken_links().len() {
                                self.selected_broken_link = 0;
                            }
                        }
                        Err(err) => {
                            self.message = Some(format!("check-links error: {err}"));
                        }
                    }
                }
            }
        }
    }

    pub fn broken_links(&self) -> &[BrokenLink] {
        self.links_report
            .as_ref()
            .map(|r| r.broken.as_slice())
            .unwrap_or(&[])
    }

    pub fn next_pane(&mut self) {
        self.focused_pane = self.focused_pane.next();
    }

    pub fn prev_pane(&mut self) {
        self.focused_pane = self.focused_pane.prev();
    }

    pub fn move_selection_up(&mut self) {
        match self.focused_pane {
            FocusedPane::Checks => self.move_check_selection_up(),
            FocusedPane::BrokenLinks => self.move_link_selection_up(),
            FocusedPane::OutputPreview => self.scroll_preview_up(1),
        }
    }

    pub fn move_selection_down(&mut self) {
        match self.focused_pane {
            FocusedPane::Checks => self.move_check_selection_down(),
            FocusedPane::BrokenLinks => self.move_link_selection_down(),
            FocusedPane::OutputPreview => self.scroll_preview_down(1),
        }
    }

    pub fn move_check_selection_up(&mut self) {
        if self.selected_step > 0 {
            self.selected_step -= 1;
            self.preview_scroll = 0;
        }
    }

    pub fn move_check_selection_down(&mut self) {
        if !self.check_slots.is_empty() && self.selected_step + 1 < self.check_slots.len() {
            self.selected_step += 1;
            self.preview_scroll = 0;
        }
    }

    pub fn move_link_selection_up(&mut self) {
        if self.selected_broken_link > 0 {
            self.selected_broken_link -= 1;
        }
    }

    pub fn move_link_selection_down(&mut self) {
        let count = self.broken_links().len();
        if count > 0 && self.selected_broken_link + 1 < count {
            self.selected_broken_link += 1;
        }
    }

    pub fn scroll_preview_up(&mut self, lines: usize) {
        self.preview_scroll = self.preview_scroll.saturating_sub(lines);
    }

    pub fn scroll_preview_down(&mut self, lines: usize) {
        self.preview_scroll = self.preview_scroll.saturating_add(lines);
    }

    pub fn selected_outcome(&self) -> Option<&Outcome> {
        self.check_slots
            .get(self.selected_step)
            .and_then(StepSlot::outcome)
    }
}
