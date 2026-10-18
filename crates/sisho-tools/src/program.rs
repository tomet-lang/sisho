use crate::tool::Tool;

/// What `sisho <word> ...` runs: a program of the ecosystem that `sisho`
/// passes the rest of the command line to, unchanged.
///
/// Every [`Tool`] is one. `tomet-shelf` is one that is not a `Tool`: it builds
/// and serves a site, which is neither checking nor fixing, so it takes no part
/// in `check` or `fix`, and `sisho` neither asks it for a version nor holds it
/// to a minimum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Program {
    word: &'static str,
    program: &'static str,
}

/// `sisho shelf ...` is `tomet-shelf ...`. The word is the role; the program
/// keeps the one name it has everywhere else.
pub const SHELF: Program = Program {
    word: "shelf",
    program: "tomet-shelf",
};

impl Program {
    /// The word after `sisho` that runs this program.
    pub fn word(self) -> &'static str {
        self.word
    }

    /// The program's name on `PATH`.
    pub fn program(self) -> &'static str {
        self.program
    }
}

impl Tool {
    /// The tool as something `sisho <word> ...` runs: its word is its name.
    pub fn as_program(self) -> Program {
        Program {
            word: self.name(),
            program: self.name(),
        }
    }
}

/// Every program that `sisho <word> ...` runs, the tools first.
pub fn programs() -> impl Iterator<Item = Program> {
    Tool::ALL.into_iter().map(Tool::as_program).chain([SHELF])
}

/// The program that `sisho <word> ...` runs, if there is one.
pub fn program(word: &str) -> Option<Program> {
    programs().find(|program| program.word() == word)
}
