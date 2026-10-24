use sisho_run::Step;

use crate::version::{self, Verdict, Version};

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

    /// `<tool> --version`, for the header of a report.
    pub fn version_step(self) -> Step {
        Step::new(format!("{} --version", self.name()), self.name()).arg("--version")
    }

    /// The oldest version of the tool that `sisho` works with.
    ///
    /// Both gave `--version` its meaning at 0.2.0, so a tool that cannot
    /// report a version is older than this. When `sisho` comes to rely on
    /// something a later version added, this is where that is said: `tomet`
    /// 0.4.0 is the first whose conversion commands take a directory, `-` and
    /// `--dry-run` alike, which `sisho convert` leaves to them.
    pub const fn min_version(self) -> Version {
        match self {
            Tool::Tomet => Version::new(0, 4, 0),
            Tool::Twrit => Version::new(0, 2, 0),
        }
    }

    /// What the version this tool reported (the first line it printed for
    /// `--version`, if it printed one) comes to against [`Tool::min_version`].
    pub fn judge(self, reported: Option<&str>) -> Verdict {
        version::judge(reported, self.min_version())
    }
}
