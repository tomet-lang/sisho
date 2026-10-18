use std::fmt;

/// A version as `major.minor.patch`.
///
/// What follows the patch number (`-dev`, `+build`) is ignored when versions
/// are compared: `sisho` asks for a minimum, and a pre-release of the minimum
/// is taken to be the minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// The first `major.minor.patch` in `text`, looking at each word in turn:
    /// `tomet 0.2.0` and `0.2.0` and `twrit 1.4.7-dev` all have one.
    pub fn find(text: &str) -> Option<Self> {
        text.split_whitespace().find_map(Self::parse)
    }

    fn parse(word: &str) -> Option<Self> {
        let mut parts = word.splitn(3, '.');
        let major = number(parts.next()?)?;
        let minor = number(parts.next()?)?;
        let patch = number(parts.next()?)?;
        Some(Self::new(major, minor, patch))
    }
}

/// The digits a part starts with, as a number; `None` if it starts with none.
fn number(part: &str) -> Option<u64> {
    let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What the version a tool reported comes to, against what `sisho` needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The tool is new enough.
    Fine(Version),
    /// The tool reported a version older than the one `sisho` needs.
    TooOld { found: Version, needs: Version },
    /// The tool is there but did not report a version `sisho` could read.
    /// The tools gave `--version` its meaning at the version `sisho` needs,
    /// so one that cannot say is older than that.
    Unreported { needs: Version },
}

impl Verdict {
    pub fn is_fine(&self) -> bool {
        matches!(self, Verdict::Fine(_))
    }
}

/// Judge what `reported` (the first line a tool printed for `--version`, if
/// it printed one) comes to against `needs`.
pub fn judge(reported: Option<&str>, needs: Version) -> Verdict {
    match reported.and_then(Version::find) {
        Some(found) if found >= needs => Verdict::Fine(found),
        Some(found) => Verdict::TooOld { found, needs },
        None => Verdict::Unreported { needs },
    }
}
