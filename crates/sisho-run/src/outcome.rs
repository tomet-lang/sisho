use std::time::Duration;

/// How a step ended.
///
/// A tool that is not installed is [`Status::NotFound`], not a pass and not a
/// skip: a step that did not run proves nothing, and counting it as a pass
/// would print `ok` for something never read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The command ran and exited with status 0.
    Passed,
    /// The command ran and exited with a non-zero status.
    Failed { code: i32 },
    /// The command was ended by a signal (Unix only).
    Signalled { signal: i32 },
    /// The program was not found on `PATH`.
    NotFound,
    /// The program exists but could not be started (permissions, resource
    /// limits, ...).
    CouldNotStart { reason: String },
}

impl Status {
    /// Whether the step passed. Everything but [`Status::Passed`] counts as a
    /// failure.
    pub fn is_pass(&self) -> bool {
        matches!(self, Status::Passed)
    }
}

/// What one step left behind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Display name of the step, e.g. `tomet check`.
    pub name: String,
    /// The command line that was run, for display. Arguments are joined with
    /// spaces and not quoted; it is for reading, not for re-running.
    pub command_line: String,
    pub status: Status,
    /// Captured standard output (lossily decoded as UTF-8).
    pub stdout: String,
    /// Captured standard error (lossily decoded as UTF-8).
    pub stderr: String,
    pub duration: Duration,
}

impl Outcome {
    /// Build an outcome for work done in-process, with no output captured.
    pub fn in_process(
        name: impl Into<String>,
        status: Status,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
        duration: Duration,
    ) -> Self {
        let name = name.into();
        Self {
            command_line: name.clone(),
            name,
            status,
            stdout: stdout.into(),
            stderr: stderr.into(),
            duration,
        }
    }

    pub fn is_pass(&self) -> bool {
        self.status.is_pass()
    }
}

/// The outcomes of one run, in plan order.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
}

impl Report {
    pub fn new(outcomes: Vec<Outcome>) -> Self {
        Self { outcomes }
    }

    /// Whether every step passed. An empty report passes.
    pub fn passed(&self) -> bool {
        self.outcomes.iter().all(Outcome::is_pass)
    }

    /// The steps that did not pass, in plan order.
    pub fn failures(&self) -> impl Iterator<Item = &Outcome> {
        self.outcomes.iter().filter(|o| !o.is_pass())
    }

    /// 0 if every step passed, 1 if any did not.
    ///
    /// `sisho`'s own errors use 2, which is the program's to return; this
    /// crate never sees them.
    pub fn exit_code(&self) -> u8 {
        if self.passed() { 0 } else { 1 }
    }
}
