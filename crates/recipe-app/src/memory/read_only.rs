//! In-memory `ReadOnlySwitch` (task 3.10, design D6).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

use crate::ports::ReadOnlySwitch;

/// In-memory adapter of [`ReadOnlySwitch`].
///
/// The type solves one problem: the tests of the job `disk_check` must read the flag and must
/// see every call of `set`, also a call that does not change the flag. The flag is an atomic, like
/// the flag of the web crate. A clone shares the flag and the history.
#[derive(Clone, Debug, Default)]
pub struct MemoryReadOnly {
    /// The flag. True means that the read-only mode is on.
    flag: Arc<AtomicBool>,
    /// The argument of each call of `set`, in call order.
    history: Arc<Mutex<Vec<bool>>>,
}

impl MemoryReadOnly {
    /// Build a switch whose flag starts as `on`. The history starts empty.
    #[must_use]
    pub fn new(on: bool) -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(on)),
            history: Arc::default(),
        }
    }

    /// Return the argument of each call of `set`, in call order.
    #[must_use]
    pub fn history(&self) -> Vec<bool> {
        self.history.lock().clone()
    }
}

impl ReadOnlySwitch for MemoryReadOnly {
    fn set(&self, on: bool) {
        self.history.lock().push(on);
        self.flag.store(on, Ordering::SeqCst);
    }

    fn is_on(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests;
