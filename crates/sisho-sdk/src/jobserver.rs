//! GNU Jobserver integration for cooperative concurrency.
//!
//! When `sisho` invokes an extension, it shares GNU jobserver descriptors
//! through environment variables (such as `CARGO_MAKEFLAGS`).
//!
//! An extension performing parallel work (multiple worker threads or child
//! processes) can use [`JobServerGate`] to acquire tokens, ensuring that total
//! parallelism across `sisho` and all tools stays strictly within the CPU budget.

use std::sync::atomic::{AtomicBool, Ordering};

pub use jobserver::{Acquired, Client};

/// The jobserver gate governing parallel task execution.
///
/// A process owns one implicit token, and shares any tokens in the jobserver
/// pipe with other processes. The gate coordinates token acquisition:
/// - The implicit token is preferred and handed out to one caller at a time.
/// - Subsequent callers block acquiring a token from the jobserver client.
/// - When the implicit token is dropped, it is released to the jobserver pipe
///   so that any blocked threads can make progress without polling.
pub struct JobServerGate {
    client: Client,
    implicit_token_held: AtomicBool,
}

impl JobServerGate {
    /// Create a gate wrapping an existing [`Client`].
    pub fn new(client: Client) -> Self {
        Self {
            client,
            implicit_token_held: AtomicBool::new(false),
        }
    }

    /// Access the underlying [`Client`].
    ///
    /// This can be used to configure child commands (e.g. `client.configure(&mut cmd)`)
    /// so subprocesses share the same jobserver.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Initialize a gate from environment variables if present, or create an
    /// in-process jobserver client with `fallback_limit` tokens.
    ///
    /// If `fallback_limit` is `None`, default to available parallelism (or 4).
    ///
    /// # Safety
    /// This function reads and takes ownership of file descriptors specified in
    /// environment variables. It should be called once, early in `main` before
    /// any other file descriptors are opened or background threads are spawned.
    pub unsafe fn from_env_or_limit(fallback_limit: Option<usize>) -> std::io::Result<Self> {
        let from_env = unsafe { Client::from_env_ext(true) };
        let client = match from_env.client {
            Ok(client) => client,
            Err(_) => {
                let limit = fallback_limit.unwrap_or_else(|| {
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4)
                });
                let pipe_tokens = limit.saturating_sub(1);
                Client::new(pipe_tokens)?
            }
        };
        Ok(Self::new(client))
    }

    /// Acquire a concurrency token. Blocks until one is available.
    pub fn acquire(&self) -> std::io::Result<JobToken<'_>> {
        if self
            .implicit_token_held
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Ok(JobToken::Implicit(self))
        } else {
            let acquired = self.client.acquire()?;
            Ok(JobToken::Acquired(acquired))
        }
    }

    /// Reclaim the donated implicit token after parallel jobs have completed.
    ///
    /// When the implicit token is released, it is donated into the jobserver
    /// pipe so that blocked threads wake up. Once all jobs have joined, this
    /// reclaims that single token without releasing it back, restoring the
    /// pipe to its original token count and resetting `implicit_token_held`.
    pub fn reclaim_implicit_token(&self) {
        if self.implicit_token_held.load(Ordering::Acquire) {
            if let Ok(acquired) = self.client.acquire() {
                acquired.drop_without_releasing();
            }
            self.implicit_token_held.store(false, Ordering::Release);
        }
    }
}

/// An RAII guard representing a held token.
///
/// Dropping this token releases it back to the gate or the jobserver pipe.
pub enum JobToken<'a> {
    Implicit(&'a JobServerGate),
    Acquired(Acquired),
}

impl Drop for JobToken<'_> {
    fn drop(&mut self) {
        if let JobToken::Implicit(gate) = self {
            let _ = gate.client.release_raw();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    use super::*;

    #[test]
    fn gate_concurrency_ceiling() {
        for pipe_tokens in [0, 1, 2] {
            let total_capacity = pipe_tokens + 1;
            let client = Client::new(pipe_tokens).unwrap();
            let gate = Arc::new(JobServerGate::new(client));

            let current_active = Arc::new(AtomicUsize::new(0));
            let max_observed = Arc::new(AtomicUsize::new(0));

            thread::scope(|s| {
                for _ in 0..6 {
                    let gate = Arc::clone(&gate);
                    let current = Arc::clone(&current_active);
                    let max_active = Arc::clone(&max_observed);

                    s.spawn(move || {
                        let _token = gate.acquire().unwrap();
                        let active = current.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(active, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(20));
                        current.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            });

            gate.reclaim_implicit_token();
            let peak = max_observed.load(Ordering::SeqCst);
            assert_eq!(
                peak, total_capacity,
                "for capacity {total_capacity}, peak observed was {peak}"
            );
        }
    }
}
