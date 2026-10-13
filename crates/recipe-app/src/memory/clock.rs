//! In-memory clock with a settable time.

use std::sync::Arc;

use parking_lot::Mutex;
use recipe_domain::timestamp::Timestamp;
use time::{Duration, OffsetDateTime};

use crate::ports::Clock;

/// A clock that a test sets and advances.
///
/// The type solves one problem: a test needs a fixed and repeatable time. A clone
/// shares the time, so the clone in a use case follows the clock in the test.
#[derive(Clone, Debug)]
pub struct MemoryClock {
    now: Arc<Mutex<Timestamp>>,
}

impl MemoryClock {
    /// Build a clock that starts at `start`.
    #[must_use]
    pub fn new(start: OffsetDateTime) -> Self {
        Self {
            now: Arc::new(Mutex::new(Timestamp::from(start))),
        }
    }

    /// Replace the time of the clock.
    pub fn set(&self, instant: Timestamp) {
        *self.now.lock() = instant;
    }

    /// Move the time forward by `by`. The time stops at the largest instant on overflow.
    pub fn advance(&self, by: Duration) {
        let mut now = self.now.lock();
        *now = Timestamp::from(now.as_offset_date_time().saturating_add(by));
    }
}

impl Default for MemoryClock {
    /// Start at the Unix epoch.
    fn default() -> Self {
        Self::new(OffsetDateTime::UNIX_EPOCH)
    }
}

impl Clock for MemoryClock {
    fn now(&self) -> Timestamp {
        *self.now.lock()
    }
}

#[cfg(test)]
mod tests;
