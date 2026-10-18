//! Layer 1: `sisho serve`.
//!
//! Kept in its own crate so that the HTTP stack (axum, tokio) stays out of
//! the build of every other command.
