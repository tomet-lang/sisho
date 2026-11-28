//! `sisho <tool> ...`: a tool, run with the rest of the command line.

use std::ffi::OsString;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Result, bail};
use sisho_run::{Status, Step};
use sisho_tools::{program, programs};

use crate::USAGE_ERROR;
use crate::extension::{extension_dir_envs, find_active, is_valid_word};
use crate::trust::{TrustConfig, TrustState};

/// Exit code when an extension is found on `PATH` but cannot be executed
/// (for example because it is `untrusted` or `denied`).
const CANNOT_EXEC: u8 = 126;

/// `sisho <tool> ...`: run the tool or extension with the rest of the command line.
///
/// `words` is what the command line held after `sisho`: the tool or extension's
/// name, then its arguments, untouched.
pub fn tool(words: &[OsString]) -> Result<ExitCode> {
    let Some((name, args)) = words.split_first() else {
        bail!("no tool given");
    };

    let word = name.to_str();
    if let Some(program) = word.and_then(program) {
        let step = Step::new(program.word(), program.program()).args(args);
        return Ok(run_inherited(&step, program.program()));
    }

    if let Some(word) = word.filter(|w| is_valid_word(w))
        && let Some(entry) = find_active(word)
    {
        let allowed = match entry.trust {
            TrustState::Allowed => true,
            TrustState::Untrusted => {
                if io::stdin().is_terminal()
                    && io::stderr().is_terminal()
                    && prompt_allow_extension(word, &entry.path, &entry.canonical_path)
                {
                    let mut config = TrustConfig::load();
                    config.set_state(&entry.canonical_path, TrustState::Allowed);
                    config.save()?;
                    true
                } else {
                    false
                }
            }
            TrustState::Denied => false,
        };

        if !allowed {
            eprintln!(
                "sisho: extension '{word}' ({}) is not trusted ({}); run 'sisho --allow {word}' to allow it",
                entry.canonical_path.display(),
                entry.trust
            );
            return Ok(ExitCode::from(CANNOT_EXEC));
        }

        let extension = format!("sisho-{word}");
        let mut step = Step::new(word, &extension).args(args);
        for (key, val) in extension_dir_envs(word) {
            step = step.env(key, val);
        }
        let status = step.run_inherited();
        match &status {
            Status::NotFound => {
                // Fall through: an unknown word and a missing extension
                // both report the same usage error (exit 2).
            }
            Status::CouldNotStart { reason } => {
                eprintln!("sisho: could not start {extension}: {reason}");
                return Ok(ExitCode::from(passthrough_code(&status)));
            }
            _ => return Ok(ExitCode::from(passthrough_code(&status))),
        }
    }

    eprintln!(
        "sisho: `{}` is neither a command nor a tool\n  tools: {}\n  an extension is a program named `sisho-<word>` on PATH\n  try `sisho --help`",
        name.to_string_lossy(),
        tool_names()
    );
    Ok(ExitCode::from(USAGE_ERROR))
}

fn prompt_allow_extension(word: &str, path: &Path, canonical_path: &Path) -> bool {
    let stdin = io::stdin();
    let stderr = io::stderr();
    prompt_allow_with_io(word, path, canonical_path, &mut stdin.lock(), &mut stderr.lock())
}

fn prompt_allow_with_io<R: BufRead, W: Write>(
    word: &str,
    path: &Path,
    canonical_path: &Path,
    input: &mut R,
    err: &mut W,
) -> bool {
    if path == canonical_path {
        let _ = writeln!(
            err,
            "sisho: extension '{word}' ({}) is not trusted (untrusted).",
            canonical_path.display()
        );
    } else {
        let _ = writeln!(
            err,
            "sisho: extension '{word}' ({} -> {}) is not trusted (untrusted).",
            path.display(),
            canonical_path.display()
        );
    }
    let _ = write!(err, "Allow and run it? [y/N]: ");
    let _ = err.flush();

    let mut line = String::new();
    match input.read_line(&mut line) {
        Ok(0) | Err(_) => false,
        Ok(_) => {
            let trimmed = line.trim();
            trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes")
        }
    }
}

/// Run `step` with standard input, output and error handed to the program as
/// they are, and take on its exit code (see [`passthrough_code`]). Says so on
/// standard error when `program` is not on `PATH` or cannot be started.
pub fn run_inherited(step: &Step, program: &str) -> ExitCode {
    let status = step.run_inherited();
    if let Status::NotFound = status {
        eprintln!("sisho: {program} is not on PATH");
    } else if let Status::CouldNotStart { reason } = &status {
        eprintln!("sisho: could not start {program}: {reason}");
    }
    ExitCode::from(passthrough_code(&status))
}

/// The words that run a program, for messages.
pub fn tool_names() -> String {
    programs()
        .map(|program| program.word())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The exit code `sisho` takes on when a tool it passed through ends as `status`.
///
/// The tool's own code is kept as it is. The rest follows `env`: 127 for a
/// program that is not found, 126 for one that cannot be started, 128 plus the
/// signal for one that was killed.
fn passthrough_code(status: &Status) -> u8 {
    match status {
        Status::Passed => 0,
        // A code outside 0..=255 cannot be an exit status; keep the low byte
        // as the operating system does.
        Status::Failed { code } => (code & 0xff) as u8,
        Status::Signalled { signal } => (128 + (signal & 0x7f)) as u8,
        Status::NotFound => 127,
        Status::CouldNotStart { .. } => 126,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tools_own_exit_code_is_kept() {
        assert_eq!(passthrough_code(&Status::Passed), 0);
        assert_eq!(passthrough_code(&Status::Failed { code: 3 }), 3);
        assert_eq!(passthrough_code(&Status::Failed { code: 255 }), 255);
    }

    #[test]
    fn what_is_not_an_exit_code_follows_env() {
        assert_eq!(passthrough_code(&Status::NotFound), 127);
        assert_eq!(
            passthrough_code(&Status::CouldNotStart {
                reason: String::new()
            }),
            126
        );
        assert_eq!(passthrough_code(&Status::Signalled { signal: 9 }), 137);
    }

    #[test]
    fn prompt_allow_accepts_y_and_yes_only() {
        let path = Path::new("/run/current-system/sw/bin/sisho-board");
        let canonical = Path::new("/nix/store/abc-sisho-board/bin/sisho-board");
        for (input, expected) in [
            ("y\n", true),
            ("Y\n", true),
            (" yes \n", true),
            ("n\n", false),
            ("\n", false),
            ("", false),
            ("maybe\n", false),
        ] {
            let mut reader = io::Cursor::new(input.as_bytes());
            let mut err = Vec::new();
            assert_eq!(
                prompt_allow_with_io("board", path, canonical, &mut reader, &mut err),
                expected,
                "unexpected result for input {input:?}"
            );
            let err_text = String::from_utf8(err).unwrap();
            assert!(err_text.contains(
                "sisho: extension 'board' (/run/current-system/sw/bin/sisho-board -> /nix/store/abc-sisho-board/bin/sisho-board) is not trusted (untrusted)."
            ));
            assert!(err_text.contains("Allow and run it? [y/N]: "));
        }
    }
}
