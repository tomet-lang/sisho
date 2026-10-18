//! `sisho <tool> ...`: a tool, run with the rest of the command line.

use std::ffi::OsString;
use std::process::ExitCode;

use anyhow::{Result, bail};
use sisho_run::{Status, Step};
use sisho_tools::Tool;

use crate::USAGE_ERROR;

/// `sisho <tool> ...`: run the tool with the rest of the command line.
///
/// `words` is what the command line held after `sisho`: the tool's name, then
/// its arguments, untouched.
pub fn tool(words: &[OsString]) -> Result<ExitCode> {
    let Some((name, args)) = words.split_first() else {
        bail!("no tool given");
    };
    let Some(tool) = name.to_str().and_then(Tool::from_name) else {
        eprintln!(
            "sisho: `{}` is neither a command nor a tool\n  tools: {}\n  try `sisho --help`",
            name.to_string_lossy(),
            tool_names()
        );
        return Ok(ExitCode::from(USAGE_ERROR));
    };

    let status = Step::new(tool.name(), tool.name())
        .args(args)
        .run_inherited();
    if let Status::NotFound = status {
        eprintln!("sisho: {} is not on PATH", tool.name());
    } else if let Status::CouldNotStart { reason } = &status {
        eprintln!("sisho: could not start {}: {reason}", tool.name());
    }
    Ok(ExitCode::from(passthrough_code(&status)))
}

/// The names of the tools, for messages.
pub fn tool_names() -> String {
    Tool::ALL
        .iter()
        .map(|tool| tool.name())
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
}
