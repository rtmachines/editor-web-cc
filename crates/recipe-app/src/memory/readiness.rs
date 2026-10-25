//! In-memory `ReadinessProbe` (task 3.10, design D6).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;

use super::call_faults::CallFaults;
use crate::ports::{NotReadyCause, Readiness, ReadinessProbe};

/// The values that a probe holds. The probe has one method, so the fault key is the unit value.
#[derive(Debug)]
struct Inner {
    /// The result of a call without a fault.
    value: Readiness,
    /// The timeout of each call, in call order.
    timeouts: Vec<Duration>,
    /// The planned faults and the call counter.
    faults: CallFaults<(), NotReadyCause>,
}

impl Default for Inner {
    /// Build a probe that is ready, with no call and no fault.
    fn default() -> Self {
        Self {
            value: Readiness::Ready,
            timeouts: Vec::new(),
            faults: CallFaults::default(),
        }
    }
}

/// In-memory adapter of [`ReadinessProbe`].
///
/// The type solves one problem: the tests of the job `alerts` and of `/readyz` need a probe
/// that they can switch between ready and not ready, without a database. The port has no error
/// type, so a fault is a [`Readiness::NotReady`] with the cause that the test gives. A fault wins
/// over the value that `set` gave. A clone shares the value, the counter, and the faults.
#[derive(Clone, Debug, Default)]
pub struct MemoryReadiness {
    /// The values of the probe.
    inner: Arc<Mutex<Inner>>,
}

impl MemoryReadiness {
    /// Set the result of every call that has no fault.
    pub fn set(&self, value: Readiness) {
        self.inner.lock().value = value;
    }

    /// Return the timeout of each call, in call order.
    #[must_use]
    pub fn timeouts(&self) -> Vec<Duration> {
        self.inner.lock().timeouts.clone()
    }

    /// Return the number of calls.
    #[must_use]
    pub fn calls(&self) -> u64 {
        self.inner.lock().faults.calls(())
    }

    /// Make call number `call` return `NotReady(cause)`. Later calls use the set value.
    pub fn fail_on_call(&self, call: u64, cause: NotReadyCause) {
        self.inner.lock().faults.fail_on_call((), call, cause);
    }

    /// Make call number `call` and every later call return `NotReady(cause)`.
    pub fn fail_from_call(&self, call: u64, cause: NotReadyCause) {
        self.inner.lock().faults.fail_from_call((), call, cause);
    }

    /// Stop every planned fault. The call counter keeps its value.
    pub fn clear_faults(&self) {
        self.inner.lock().faults.clear();
    }
}

#[async_trait]
impl ReadinessProbe for MemoryReadiness {
    async fn check(&self, timeout: Duration) -> Readiness {
        let mut inner = self.inner.lock();
        inner.timeouts.push(timeout);
        match inner.faults.next(()) {
            Some(cause) => Readiness::NotReady(cause),
            None => inner.value,
        }
    }
}

#[cfg(test)]
mod tests;
