use sisho_run::Step;

/// A tool in the ecosystem that `sisho` orchestrates.
///
/// `tandr` joins this list when it has a command line to call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Tomet,
    Twrit,
}

impl Tool {
    /// Every tool, in the order plans list them.
    pub const ALL: [Tool; 2] = [Tool::Tomet, Tool::Twrit];

    /// The tool's name, which is also its program name on `PATH`.
    pub fn name(self) -> &'static str {
        match self {
            Tool::Tomet => "tomet",
            Tool::Twrit => "twrit",
        }
    }

    /// The tool called `name`, for a pass-through `sisho <name> ...`.
    pub fn from_name(name: &str) -> Option<Tool> {
        Tool::ALL.into_iter().find(|tool| tool.name() == name)
    }

    /// `<tool> --version`, for the header of a report.
    pub fn version_step(self) -> Step {
        Step::new(format!("{} --version", self.name()), self.name()).arg("--version")
    }
}
