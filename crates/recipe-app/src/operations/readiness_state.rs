//! In-process readiness state of the server (`reliability.md` sections 3.7 and 8).

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use recipe_domain::timestamp::Timestamp;

/// Value of the atomic while the server is ready. No instant has this value.
const READY: i64 = i64::MIN;

/// Return the whole seconds since the Unix epoch of an instant.
fn epoch_seconds(instant: Timestamp) -> i64 {
    instant.as_offset_date_time().unix_timestamp()
}

/// The readiness of the server and the start of the current outage.
///
/// The type solves one problem: the health router writes the result of each readiness check.
/// The job `alerts` must read how long `/readyz` has answered `not ready`
/// (`alerts.not_ready_for`). Both run in other tasks, so the state is one atomic and needs no lock.
/// The atomic holds the epoch seconds of the start of the outage, or `READY`. One atomic cannot
/// show a ready flag and a start time that disagree. A clone shares the atomic.
///
/// Every access uses `Ordering::Relaxed`. The atomic publishes no other memory, and a reader only
/// needs its value. The job reads again at its next run, so a stale read is safe.
#[derive(Clone, Debug)]
pub struct ReadinessState(Arc<AtomicI64>);

impl ReadinessState {
    /// Build the state of a server that started at `started`. The server is not ready yet.
    ///
    /// Readiness turns on after the first successful check (ADR-0061).
    #[must_use]
    pub fn new(started: Timestamp) -> Self {
        Self(Arc::new(AtomicI64::new(epoch_seconds(started))))
    }

    /// Mark the server as ready. The health router calls it after a successful check.
    pub fn mark_ready(&self) {
        self.0.store(READY, Ordering::Relaxed);
    }

    /// Mark the server as not ready since `at`.
    ///
    /// A state that is already not ready keeps its earlier start, so a second failed check
    /// does not restart the count.
    pub fn mark_not_ready(&self, at: Timestamp) {
        let _ignored = self.0.compare_exchange(
            READY,
            epoch_seconds(at),
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
    }

    /// Return true when the server is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.0.load(Ordering::Relaxed) == READY
    }

    /// Return how long the server has been not ready at `now`, or `None` when it is ready.
    ///
    /// A clock that moved back gives zero.
    #[must_use]
    pub fn not_ready_for(&self, now: Timestamp) -> Option<Duration> {
        let since = self.0.load(Ordering::Relaxed);
        if since == READY {
            return None;
        }
        let seconds = epoch_seconds(now)
            .checked_sub(since)
            .and_then(|elapsed| u64::try_from(elapsed).ok())
            .unwrap_or(0);
        Some(Duration::from_secs(seconds))
    }
}
