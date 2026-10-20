use std::path::PathBuf;

use sisho_run::{Outcome, run_parallel};
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

/// The state of the counter and board.
pub struct Model {
    pub dir: PathBuf,
    pub tools: Vec<ToolStatus>,
    pub extensions: Vec<ExtensionItem>,
    pub check_outcomes: Vec<Outcome>,
    pub links_report: Option<Report>,
    pub selected_step: usize,
    pub confirm_fix: bool,
    pub message: Option<String>,
}

impl Model {
    pub fn new(dir: PathBuf, extensions: Vec<ExtensionItem>) -> Self {
        let mut model = Self {
            dir,
            tools: Vec::new(),
            extensions,
            check_outcomes: Vec::new(),
            links_report: None,
            selected_step: 0,
            confirm_fix: false,
            message: None,
        };
        model.refresh_tools();
        model.run_check_links();
        model.run_checks();
        model
    }

    /// Refresh tools information and version status.
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

    /// Run the check steps suite across the project.
    pub fn run_checks(&mut self) {
        let steps = check_steps(&self.dir);
        let jobs = steps.iter().map(sisho_run::Job::step).collect();
        let outcomes = run_parallel(jobs);
        self.check_outcomes = outcomes;
        if self.selected_step >= self.check_outcomes.len() {
            self.selected_step = 0;
        }
        self.message = Some("Checks completed.".to_string());
    }

    /// Run the fix steps suite across the project.
    pub fn run_fixes(&mut self) {
        let steps = fix_steps(&self.dir);
        let mut outcomes = Vec::new();
        for step in steps {
            outcomes.push(step.run());
        }
        self.check_outcomes = outcomes;
        if self.selected_step >= self.check_outcomes.len() {
            self.selected_step = 0;
        }
        self.confirm_fix = false;
        self.message = Some("Fixes applied. Run check to verify.".to_string());
    }

    /// Run check-links verification on the project directory.
    pub fn run_check_links(&mut self) {
        let cache_path = sisho_vault::links::default_cache_path();
        match check_links(&self.dir, &cache_path) {
            Ok(report) => {
                self.links_report = Some(report);
            }
            Err(err) => {
                self.message = Some(format!("check-links error: {err}"));
            }
        }
    }

    pub fn broken_links(&self) -> &[BrokenLink] {
        self.links_report
            .as_ref()
            .map(|r| r.broken.as_slice())
            .unwrap_or(&[])
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_step > 0 {
            self.selected_step -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if !self.check_outcomes.is_empty() && self.selected_step + 1 < self.check_outcomes.len() {
            self.selected_step += 1;
        }
    }

    pub fn selected_outcome(&self) -> Option<&Outcome> {
        self.check_outcomes.get(self.selected_step)
    }
}
