use std::ffi::{OsStr, OsString};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Instant;

use crate::outcome::{Outcome, Status};

/// One command to run: a display name, a program and its arguments.
///
/// Programs and arguments are `OsString`s, so a path that is not valid UTF-8
/// reaches the tool unchanged.
#[derive(Clone, Debug)]
pub struct Step {
    name: String,
    program: OsString,
    args: Vec<OsString>,
    current_dir: Option<PathBuf>,
}

impl Step {
    /// A step named `name` that runs `program`.
    pub fn new(name: impl Into<String>, program: impl AsRef<OsStr>) -> Self {
        Self {
            name: name.into(),
            program: program.as_ref().to_owned(),
            args: Vec::new(),
            current_dir: None,
        }
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|arg| arg.as_ref().to_owned()));
        self
    }

    /// Run in `dir` instead of the current directory.
    pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn program(&self) -> &OsStr {
        &self.program
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.args
    }

    /// The command line for display: program and arguments joined by spaces,
    /// lossily decoded and not quoted.
    pub fn command_line(&self) -> String {
        let mut line = self.program.to_string_lossy().into_owned();
        for arg in &self.args {
            line.push(' ');
            line.push_str(&arg.to_string_lossy());
        }
        line
    }

    /// Run to completion with standard input, output and error handed to the
    /// child as they are, for a pass-through where the tool talks to the
    /// terminal itself. Nothing is captured.
    ///
    /// Like [`Step::run`], every way of not passing is a [`Status`].
    pub fn run_inherited(&self) -> Status {
        match self.command().status() {
            Ok(status) => status_of(status),
            Err(error) => start_failure(&error),
        }
    }

    /// Run to completion, capturing standard output and standard error.
    ///
    /// Standard input is closed, so a tool that would wait for input fails
    /// instead of hanging the run. Never panics and never returns an error:
    /// every way of not passing is a [`Status`].
    pub fn run(&self) -> Outcome {
        let mut command = self.command();
        command.stdin(Stdio::null());

        let started = Instant::now();
        let result = command.output();
        let duration = started.elapsed();

        let (status, stdout, stderr) = match result {
            Ok(output) => (
                status_of(output.status),
                String::from_utf8_lossy(&output.stdout).into_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ),
            Err(error) => (start_failure(&error), String::new(), String::new()),
        };

        Outcome {
            name: self.name.clone(),
            command_line: self.command_line(),
            status,
            stdout,
            stderr,
            duration,
        }
    }
}

impl Step {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        if let Some(dir) = &self.current_dir {
            command.current_dir(dir);
        }
        if let Some(gate) = crate::gate::gate() {
            gate.configure(&mut command);
        }
        command
    }
}

fn start_failure(error: &std::io::Error) -> Status {
    if error.kind() == ErrorKind::NotFound {
        Status::NotFound
    } else {
        Status::CouldNotStart {
            reason: error.to_string(),
        }
    }
}

fn status_of(status: ExitStatus) -> Status {
    if status.success() {
        return Status::Passed;
    }
    if let Some(code) = status.code() {
        return Status::Failed { code };
    }
    signal_of(status)
}

#[cfg(unix)]
fn signal_of(status: ExitStatus) -> Status {
    use std::os::unix::process::ExitStatusExt;
    match status.signal() {
        Some(signal) => Status::Signalled { signal },
        None => Status::Failed { code: -1 },
    }
}

#[cfg(not(unix))]
fn signal_of(_status: ExitStatus) -> Status {
    Status::Failed { code: -1 }
}
