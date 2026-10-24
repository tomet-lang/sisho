//! Output that may go nowhere: a reader that closes the pipe early.
//!
//! `sisho --list | head` closes the pipe after a few lines. Rust ignores
//! the signal that would normally end the program quietly, so a write then
//! fails with `BrokenPipe`, and `println!` turns that into a panic. Writes
//! here return the error instead, and the program ends quietly with the
//! status a shell reports for a process killed by that signal.

use std::io;

use anyhow::Error;

/// 128 plus `SIGPIPE` (13), as a shell reports it.
pub const BROKEN_PIPE: u8 = 141;

/// Whether `error` is, or was caused by, a write to a pipe nobody reads.
pub fn is_broken_pipe(error: &Error) -> bool {
    error.chain().any(|cause| {
        let io_kind = cause.downcast_ref::<io::Error>().map(io::Error::kind);
        let json_kind = cause
            .downcast_ref::<serde_json::Error>()
            .and_then(serde_json::Error::io_error_kind);
        io_kind == Some(io::ErrorKind::BrokenPipe) || json_kind == Some(io::ErrorKind::BrokenPipe)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broken_pipe_is_recognised_through_context_and_json() {
        let pipe = io::Error::from(io::ErrorKind::BrokenPipe);
        assert!(is_broken_pipe(&Error::new(pipe)));

        let pipe = io::Error::from(io::ErrorKind::BrokenPipe);
        assert!(is_broken_pipe(
            &Error::new(pipe).context("while printing the report")
        ));

        let pipe = io::Error::from(io::ErrorKind::BrokenPipe);
        assert!(is_broken_pipe(&Error::new(serde_json::Error::io(pipe))));
    }

    #[test]
    fn other_errors_are_not() {
        assert!(!is_broken_pipe(&anyhow::anyhow!("something else")));
        assert!(!is_broken_pipe(&Error::new(io::Error::from(
            io::ErrorKind::NotFound
        ))));
    }
}
