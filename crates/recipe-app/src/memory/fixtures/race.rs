//! A unit of work that changes the store between the read and the locks of a command.

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::error::StorageError;
use crate::memory::MemoryStore;
use crate::ports::{Isolation, Transaction, UnitOfWork};

/// The change that a race test makes. It gets the store.
type Hook = Box<dyn FnOnce(&MemoryStore) + Send>;

/// A store that runs a hook once, just before the first `READ COMMITTED` transaction opens.
///
/// The type solves one problem: design D10 says that a command runs its domain method on
/// the locked state. A test must change the state after the read of step 2 and before the
/// locks of step 4, and the use case offers no seam between them.
pub(crate) struct RaceStore {
    inner: MemoryStore,
    hook: Mutex<Option<Hook>>,
}

impl RaceStore {
    /// Wrap `inner`. The hook runs at the first `begin(ReadCommitted)`.
    pub(crate) fn new(
        inner: MemoryStore,
        hook: impl FnOnce(&MemoryStore) + Send + 'static,
    ) -> Self {
        Self {
            inner,
            hook: Mutex::new(Some(Box::new(hook))),
        }
    }
}

#[async_trait]
impl UnitOfWork for RaceStore {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        if isolation == Isolation::ReadCommitted {
            let hook = self.hook.lock().take();
            if let Some(hook) = hook {
                hook(&self.inner);
            }
        }
        self.inner.begin(isolation).await
    }
}
