//! In-memory `BackupDirectory` (task 3.10, design D6).

use std::io;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::timestamp::Timestamp;

use super::call_faults::CallFaults;
use crate::ports::{BackupDirectory, BackupFault};

/// The values that an adapter holds. The port has one method, so the fault key is the unit value.
#[derive(Debug, Default)]
struct Inner {
    /// The time of the newest complete set, or `None` when no set exists.
    newest: Option<Timestamp>,
    /// True when the directory cannot be read.
    unreadable: bool,
    /// The planned faults and the call counter.
    faults: CallFaults<(), ()>,
}

/// In-memory adapter of [`BackupDirectory`].
///
/// The type solves one problem: the tests of the alert "dump too old" must tell "no set" from
/// "directory unreadable". A new adapter holds no set. A test sets the newest set, makes the
/// directory unreadable, or fails one call. A clone shares the values, the counter, and the faults.
#[derive(Clone, Debug, Default)]
pub struct MemoryBackupDirectory {
    /// The values of the adapter.
    inner: Arc<Mutex<Inner>>,
}

impl MemoryBackupDirectory {
    /// Set the time of the newest complete set. The call also makes the directory readable.
    pub fn set_newest(&self, newest: Option<Timestamp>) {
        let mut inner = self.inner.lock();
        inner.newest = newest;
        inner.unreadable = false;
    }

    /// Make every later call fail until `set_newest` runs.
    pub fn set_unreadable(&self) {
        self.inner.lock().unreadable = true;
    }

    /// Return the number of calls.
    #[must_use]
    pub fn calls(&self) -> u64 {
        self.inner.lock().faults.calls(())
    }

    /// Make call number `call` fail. Later calls succeed.
    pub fn fail_on_call(&self, call: u64) {
        self.inner.lock().faults.fail_on_call((), call, ());
    }

    /// Make call number `call` and every later call fail.
    pub fn fail_from_call(&self, call: u64) {
        self.inner.lock().faults.fail_from_call((), call, ());
    }

    /// Stop every planned fault. The call counter keeps its value.
    pub fn clear_faults(&self) {
        self.inner.lock().faults.clear();
    }
}

#[async_trait]
impl BackupDirectory for MemoryBackupDirectory {
    async fn newest_complete_set(&self) -> Result<Option<Timestamp>, BackupFault> {
        let mut inner = self.inner.lock();
        let planned_fault = inner.faults.next(()).is_some();
        if planned_fault || inner.unreadable {
            return Err(BackupFault::Unreadable(io::Error::other(
                "injected backup directory fault",
            )));
        }
        Ok(inner.newest)
    }
}

#[cfg(test)]
mod tests;
