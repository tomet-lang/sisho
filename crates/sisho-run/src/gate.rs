use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use jobserver::{Acquired, Client};

/// The jobserver gate governing parallel task execution.
///
/// A process owns one implicit token, and shares any tokens in the jobserver
/// pipe with other processes. The gate coordinates token acquisition:
/// - The implicit token is preferred and handed out to one caller at a time
///   via an atomic flag.
/// - Every other caller blocks acquiring a token from the jobserver client.
/// - When the implicit token is dropped, it is released to the jobserver pipe
///   so that any blocked threads can make progress without polling.
pub struct JobServerGate {
    client: Client,
    implicit_token_held: AtomicBool,
}

impl JobServerGate {
    /// Create a new gate wrapping `client`.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            implicit_token_held: AtomicBool::new(false),
        }
    }

    /// Access the underlying [`Client`].
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Configure a [`Command`] to inherit this jobserver.
    pub fn configure(&self, cmd: &mut Command) {
        self.client.configure(cmd);
    }

    /// Acquire a token. Blocks until one is available.
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
    ///
    /// Note: with an inherited jobserver whose parent is saturated, a sibling
    /// process may acquire the donated token before this call, causing this to
    /// block until some process returns a token to the pipe.
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
/// Dropping this token releases it back to the gate or the jobserver.
pub enum JobToken<'a> {
    Implicit(&'a JobServerGate),
    Acquired(Acquired),
}

impl Drop for JobToken<'_> {
    fn drop(&mut self) {
        if let JobToken::Implicit(gate) = self {
            // Forward the implicit token to the jobserver pipe so that
            // any threads blocked in Client::acquire() wake up and make progress.
            let _ = gate.client.release_raw();
        }
    }
}

static GATE: OnceLock<JobServerGate> = OnceLock::new();

/// Set the process-wide jobserver gate.
pub fn init_gate(client: Client) {
    let _ = GATE.set(JobServerGate::new(client));
}

/// Returns the process-wide jobserver gate, if initialized.
pub fn gate() -> Option<&'static JobServerGate> {
    GATE.get()
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
        for pipe_tokens in [0, 1, 3] {
            let total_capacity = pipe_tokens + 1; // pipe tokens + implicit token
            let client = Client::new(pipe_tokens).unwrap();
            let gate = Arc::new(JobServerGate::new(client));

            let current_active = Arc::new(AtomicUsize::new(0));
            let max_observed = Arc::new(AtomicUsize::new(0));

            thread::scope(|s| {
                for _ in 0..10 {
                    let gate = Arc::clone(&gate);
                    let current = Arc::clone(&current_active);
                    let max_active = Arc::clone(&max_observed);

                    s.spawn(move || {
                        let _token = gate.acquire().unwrap();
                        let active = current.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(active, Ordering::SeqCst);

                        // Hold long enough for other threads to overlap
                        thread::sleep(Duration::from_millis(30));

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

            // Verify tokens in pipe are preserved exactly
            assert_eq!(gate.client().available().unwrap_or(0), pipe_tokens);
        }
    }
}
