//! In-memory `CheckResults` (ADR-0151 (r), design D5).

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;

use super::call_faults::CallFaults;
use crate::ports::{CheckResult, CheckResultFault, CheckResults, ScheduledCheck};

/// What a read of one check gives.
#[derive(Clone, Copy, Debug)]
enum Reading {
    /// The check has this newest result, or `None` when it never ran.
    Result(Option<CheckResult>),
    /// The result file cannot be read.
    Unreadable,
}

/// The values that an adapter holds.
#[derive(Debug, Default)]
struct Inner {
    /// The reading of each check. A check without a reading never ran.
    readings: HashMap<ScheduledCheck, Reading>,
    /// The time that each call waits before it returns.
    delay: Duration,
    /// The planned faults and the call counters, by check.
    faults: CallFaults<ScheduledCheck, ()>,
}

/// In-memory adapter of [`CheckResults`].
///
/// The type solves one problem: the tests of the alerts "restore test" and "base backup" must
/// tell "never ran", "failed", and "unreadable" apart, and they must move the clock during a
/// read. A new adapter holds no result. A test sets the result of a check, makes a check
/// unreadable, fails one call, or plans a delay. A clone shares the values, the counters, and the
/// faults.
#[derive(Clone, Debug, Default)]
pub struct MemoryCheckResults {
    /// The values of the adapter.
    inner: Arc<Mutex<Inner>>,
}

impl MemoryCheckResults {
    /// Set the newest result of `check`. The call also makes the check readable.
    pub fn set_result(&self, check: ScheduledCheck, result: Option<CheckResult>) {
        self.inner
            .lock()
            .readings
            .insert(check, Reading::Result(result));
    }

    /// Make every later read of `check` fail until `set_result` runs.
    pub fn set_unreadable(&self, check: ScheduledCheck) {
        self.inner
            .lock()
            .readings
            .insert(check, Reading::Unreadable);
    }

    /// Make every call wait for `delay` before it returns. The wait uses the clock of `tokio`.
    pub fn set_delay(&self, delay: Duration) {
        self.inner.lock().delay = delay;
    }

    /// Return the number of calls for `check`.
    #[must_use]
    pub fn calls(&self, check: ScheduledCheck) -> u64 {
        self.inner.lock().faults.calls(check)
    }

    /// Make call number `call` for `check` fail. Later calls succeed.
    pub fn fail_on_call(&self, check: ScheduledCheck, call: u64) {
        self.inner.lock().faults.fail_on_call(check, call, ());
    }

    /// Make call number `call` for `check` and every later call fail.
    pub fn fail_from_call(&self, check: ScheduledCheck, call: u64) {
        self.inner.lock().faults.fail_from_call(check, call, ());
    }

    /// Stop every planned fault. The call counters keep their values.
    pub fn clear_faults(&self) {
        self.inner.lock().faults.clear();
    }
}

#[async_trait]
impl CheckResults for MemoryCheckResults {
    async fn newest(&self, check: ScheduledCheck) -> Result<Option<CheckResult>, CheckResultFault> {
        let (answer, delay) = {
            let mut inner = self.inner.lock();
            let planned_fault = inner.faults.next(check).is_some();
            let reading = inner.readings.get(&check).copied();
            let answer = if planned_fault || matches!(reading, Some(Reading::Unreadable)) {
                Err(CheckResultFault::Unreadable {
                    check,
                    source: io::Error::other("injected check result fault"),
                })
            } else {
                Ok(match reading {
                    Some(Reading::Result(result)) => result,
                    Some(Reading::Unreadable) | None => None,
                })
            };
            (answer, inner.delay)
        };
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        answer
    }
}

#[cfg(test)]
mod tests;
