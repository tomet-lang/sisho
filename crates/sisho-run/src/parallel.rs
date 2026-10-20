use std::thread;
use std::time::Duration;

use crate::outcome::{Outcome, Status};
use crate::step::Step;

/// One unit of work that produces an [`Outcome`]: a [`Step`], or work `sisho`
/// does in-process.
///
/// Jobs of both kinds sit in the same list, which is why this is a type of its
/// own rather than a bare closure.
pub struct Job<'a>(Box<dyn FnOnce() -> Outcome + Send + 'a>);

impl<'a> Job<'a> {
    /// A job that runs `work`.
    pub fn new(work: impl FnOnce() -> Outcome + Send + 'a) -> Self {
        Self(Box::new(work))
    }

    /// A job that runs `step` as a subprocess.
    pub fn step(step: &'a Step) -> Self {
        Self::new(|| step.run())
    }
}

/// Run `jobs` concurrently on threads, calling `on_complete` with each job's
/// index and outcome as soon as it finishes, and return all outcomes in the
/// original order.
pub fn run_parallel_streaming<F>(jobs: Vec<Job<'_>>, on_complete: F) -> Vec<Outcome>
where
    F: Fn(usize, Outcome) + Send + Sync,
{
    let outcomes = thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .into_iter()
            .enumerate()
            .map(|(index, Job(work))| {
                let on_complete = &on_complete;
                scope.spawn(move || {
                    let _token = match crate::gate::gate() {
                        Some(g) => match g.acquire() {
                            Ok(token) => Some(token),
                            Err(err) => {
                                let outcome = Outcome {
                                    name: "jobserver".to_string(),
                                    command_line: String::new(),
                                    status: Status::CouldNotStart {
                                        reason: format!("failed to acquire jobserver token: {err}"),
                                    },
                                    stdout: String::new(),
                                    stderr: String::new(),
                                    duration: Duration::ZERO,
                                };
                                on_complete(index, outcome.clone());
                                return outcome;
                            }
                        },
                        None => None,
                    };
                    let outcome = work();
                    on_complete(index, outcome.clone());
                    outcome
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| match handle.join() {
                Ok(outcome) => outcome,
                Err(panic) => std::panic::resume_unwind(panic),
            })
            .collect()
    });

    if let Some(gate) = crate::gate::gate() {
        gate.reclaim_implicit_token();
    }

    outcomes
}

/// Run `jobs` at the same time, one thread each, and return their outcomes in
/// the order the jobs were given -- not the order they finished.
///
/// The count is not bounded; a run has a handful of steps, not thousands. If a
/// job panics, the panic is passed on.
pub fn run_parallel(jobs: Vec<Job<'_>>) -> Vec<Outcome> {
    run_parallel_streaming(jobs, |_, _| {})
}

