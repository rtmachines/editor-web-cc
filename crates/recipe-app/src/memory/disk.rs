//! In-memory `DiskSpace` (task 3.10, design D6).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use super::call_faults::CallFaults;
use crate::ports::{DiskFault, DiskSpace, FreeSpace};

/// Why a read of the free space fails. The adapter turns it into a [`DiskFault`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskFailure {
    /// The system has no call for the free space.
    Unsupported,
    /// The call failed for the path.
    Unreadable,
}

impl DiskFailure {
    /// Return the fault of the port. An unreadable path carries a fixed `io::Error`.
    fn into_fault(self) -> DiskFault {
        match self {
            Self::Unsupported => DiskFault::Unsupported,
            Self::Unreadable => DiskFault::Unreadable(io::Error::other("injected disk fault")),
        }
    }
}

/// What a read of one path gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskReading {
    /// The read succeeds with this free space.
    Space(FreeSpace),
    /// The read fails for this reason.
    Failure(DiskFailure),
}

/// The values that an adapter holds. The port has one method, so the fault key is the unit value.
#[derive(Debug, Default)]
struct Inner {
    /// The reading of each path. A path without a reading is unreadable.
    readings: BTreeMap<PathBuf, DiskReading>,
    /// The path of each call, in call order.
    queries: Vec<PathBuf>,
    /// The planned faults and the call counter. A fault hits one call, whatever the path.
    faults: CallFaults<(), DiskFailure>,
}

/// In-memory adapter of [`DiskSpace`].
///
/// The type solves one problem: the tests of the job `disk_check` need a free space for each of
/// the three volumes `pgdata`, `backups`, and `walarchive`, and they need a failure of one call.
/// A fault wins over the reading of the path. A clone shares the readings, the counter, and the
/// faults.
#[derive(Clone, Debug, Default)]
pub struct MemoryDiskSpace {
    /// The values of the adapter.
    inner: Arc<Mutex<Inner>>,
}

impl MemoryDiskSpace {
    /// Set the reading of `path`. The call replaces an earlier reading of the same path.
    pub fn set(&self, path: &Path, reading: DiskReading) {
        self.inner
            .lock()
            .readings
            .insert(path.to_path_buf(), reading);
    }

    /// Return the path of each call, in call order.
    #[must_use]
    pub fn queries(&self) -> Vec<PathBuf> {
        self.inner.lock().queries.clone()
    }

    /// Return the number of calls.
    #[must_use]
    pub fn calls(&self) -> u64 {
        self.inner.lock().faults.calls(())
    }

    /// Make call number `call` fail with `failure`, whatever the path. Later calls succeed.
    pub fn fail_on_call(&self, call: u64, failure: DiskFailure) {
        self.inner.lock().faults.fail_on_call((), call, failure);
    }

    /// Make call number `call` and every later call fail with `failure`.
    pub fn fail_from_call(&self, call: u64, failure: DiskFailure) {
        self.inner.lock().faults.fail_from_call((), call, failure);
    }

    /// Stop every planned fault. The call counter keeps its value.
    pub fn clear_faults(&self) {
        self.inner.lock().faults.clear();
    }
}

#[async_trait]
impl DiskSpace for MemoryDiskSpace {
    async fn free_space(&self, path: &Path) -> Result<FreeSpace, DiskFault> {
        let mut inner = self.inner.lock();
        inner.queries.push(path.to_path_buf());
        let failure = match inner.faults.next(()) {
            Some(failure) => failure,
            None => match inner.readings.get(path) {
                Some(DiskReading::Space(space)) => return Ok(*space),
                Some(DiskReading::Failure(failure)) => *failure,
                None => DiskFailure::Unreadable,
            },
        };
        Err(failure.into_fault())
    }
}

#[cfg(test)]
mod tests;
