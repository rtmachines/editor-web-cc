//! Supervised background jobs (design D6, `reliability.md` section 2.4).
//!
//! Each job has a function `run_once` in this crate. The loop, the timer, and the supervisor
//! live in `recipe-server`, so a test calls `run_once` without a clock or a runtime.
//! One module holds one job.

/// The kind of the trigger that started a run of a job.
///
/// The type solves one problem: the run of `audit_chain_check` must know whether a catch-up
/// trigger or a nightly trigger fired it (ADR-0088 (b)). The catch-up run skips after a newer
/// completed check. The nightly run never skips. A job with one trigger ignores the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKind {
    /// An interval or a time of day fired the run.
    Scheduled,
    /// A one-shot trigger fired the run. This includes the first run of `Schedule::now_then_every`.
    CatchUp,
}

pub mod alerts;
pub mod audit_chain_check;
pub mod cleanup;
pub mod clock_check;
pub mod disk_check;
pub mod heartbeat;
