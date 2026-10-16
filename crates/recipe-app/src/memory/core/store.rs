//! The in-memory unit of work.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::AuditEntry;
use recipe_domain::timestamp::Timestamp;

use super::journal::{DraftWrite, JournalEntry};
use super::state::MemoryState;
use super::transaction::MemoryTransaction;
use crate::error::StorageError;
use crate::memory::{Fault, FaultPoint, MemoryClock};
use crate::ports::Clock as _;
use crate::ports::{Isolation, Transaction, UnitOfWork};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Faults {
    pub(super) begin: Option<Fault>,
    pub(super) append_audit: Option<Fault>,
    pub(super) finish: Option<Fault>,
    pub(super) insert_unit: Option<Fault>,
    pub(super) insert_struct: Option<Fault>,
    pub(super) insert_version: Option<Fault>,
}

/// The change that a race test makes. It gets the store.
type Hook = Box<dyn FnOnce(&MemoryStore) + Send>;

/// A change that runs once, just before the next `read_draft`.
#[derive(Default)]
pub(super) struct ReadDraftHook(Option<Hook>);

impl fmt::Debug for ReadDraftHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0.is_some() {
            "ReadDraftHook(set)"
        } else {
            "ReadDraftHook(empty)"
        })
    }
}

/// The state that all clones of a store and all its transactions share.
#[derive(Debug, Default)]
pub(super) struct Shared {
    pub(super) state: MemoryState,
    pub(super) audit: Vec<AuditEntry>,
    /// The commit time of each entry of `audit`, in the same order (`occurred_at` in SQL).
    pub(super) audit_times: Vec<Timestamp>,
    pub(super) journal: Vec<JournalEntry>,
    pub(super) draft_writes: Vec<DraftWrite>,
    pub(super) faults: Faults,
    pub(super) queued_faults: Vec<(FaultPoint, Fault)>,
    pub(super) clock: MemoryClock,
    pub(super) read_draft_hook: ReadDraftHook,
}

impl Shared {
    /// Take the oldest queued fault of `point` out of the queue, or return `None`.
    pub(super) fn take_queued(&mut self, point: FaultPoint) -> Option<Fault> {
        let index = self
            .queued_faults
            .iter()
            .position(|(queued, _fault)| *queued == point)?;
        Some(self.queued_faults.remove(index).1)
    }
}

/// An in-memory `UnitOfWork`.
///
/// The type solves one problem: a use case test needs a transaction with all or
/// nothing semantics (R29), a record of the lock order, and failure injection, with
/// no database. A clone shares the state. A transaction works on a copy of the state
/// and replaces the shared state only at `finish`.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    shared: Arc<Mutex<Shared>>,
}

impl MemoryStore {
    /// Build an empty store whose transactions take their time from `clock`.
    ///
    /// The time of `now()` in SQL is the time of the clock at `begin`. A seed helper uses
    /// the time of the clock at the call.
    #[must_use]
    pub fn with_clock(clock: MemoryClock) -> Self {
        Self {
            shared: Arc::new(Mutex::new(Shared {
                clock,
                ..Shared::default()
            })),
        }
    }

    /// Build a handle of the store that owns `shared`.
    pub(super) const fn from_shared(shared: Arc<Mutex<Shared>>) -> Self {
        Self { shared }
    }

    /// Return a handle of the clock of the store. The handle shares the time.
    #[must_use]
    pub fn clock(&self) -> MemoryClock {
        self.shared.lock().clock.clone()
    }

    /// Change the committed state outside a transaction. The seed helpers use it.
    ///
    /// The closure gets the state and the current time of the clock.
    pub(crate) fn update_state<R>(
        &self,
        change: impl FnOnce(&mut MemoryState, Timestamp) -> R,
    ) -> R {
        let mut shared = self.shared.lock();
        let now = shared.clock.now();
        change(&mut shared.state, now)
    }

    /// Open a transaction and return its concrete type.
    ///
    /// The port method `begin` boxes the result. A test and an adapter of a later
    /// task use this method to reach `lock` and `state_mut`.
    ///
    /// # Errors
    ///
    /// Returns the fault that `fail_begin` set. The journal then holds no `Begin` line.
    pub fn begin_memory(&self, isolation: Isolation) -> Result<MemoryTransaction, StorageError> {
        let mut shared = self.shared.lock();
        if let Some(fault) = shared.take_queued(FaultPoint::Begin) {
            return Err(fault.into());
        }
        if let Some(fault) = shared.faults.begin {
            return Err(fault.into());
        }
        shared.journal.push(JournalEntry::Begin(isolation));
        Ok(MemoryTransaction::new(
            Arc::clone(&self.shared),
            shared.state.clone(),
            shared.clock.now(),
            isolation,
        ))
    }

    /// Run `work` as one pool-level transaction that commits at once (`LoginAttemptStore`,
    /// `SessionStore`).
    ///
    /// The method solves one problem: a pool port opens its own transaction and cannot use
    /// `UnitOfWork::commit`, because it writes no audit entry. It honours the fault of
    /// `fail_begin`, journals a `Begin` line, and gives `work` the committed state, the time of the
    /// clock, and the journal. The mutex stays locked while `work` runs, so the work is atomic.
    ///
    /// # Errors
    ///
    /// Returns the fault that `fail_begin` set. The journal then holds no `Begin` line.
    pub(crate) fn pool_transaction<R>(
        &self,
        isolation: Isolation,
        work: impl FnOnce(&mut MemoryState, Timestamp, &mut Vec<JournalEntry>) -> R,
    ) -> Result<R, StorageError> {
        let mut shared = self.shared.lock();
        if let Some(fault) = shared.faults.begin {
            return Err(fault.into());
        }
        shared.journal.push(JournalEntry::Begin(isolation));
        let now = shared.clock.now();
        let Shared { state, journal, .. } = &mut *shared;
        Ok(work(state, now, journal))
    }

    /// Return a copy of the committed audit entries, in commit order.
    #[must_use]
    pub fn audit_entries(&self) -> Vec<AuditEntry> {
        self.shared.lock().audit.clone()
    }

    /// Return a copy of the lock journal, in call order. Rolled back transactions stay in it.
    #[must_use]
    pub fn journal(&self) -> Vec<JournalEntry> {
        self.shared.lock().journal.clone()
    }

    /// Return a copy of the draft write log, in call order. Rolled back transactions stay in it.
    #[must_use]
    pub fn draft_writes(&self) -> Vec<DraftWrite> {
        self.shared.lock().draft_writes.clone()
    }

    /// Return a copy of the committed state.
    #[must_use]
    pub fn state(&self) -> MemoryState {
        self.shared.lock().state.clone()
    }

    /// Make every later `begin` fail with `fault`.
    pub fn fail_begin(&self, fault: Fault) {
        self.shared.lock().faults.begin = Some(fault);
    }

    /// Make the next `times` calls at `point` fail with `fault`. Later calls succeed.
    ///
    /// The method solves one problem: a retry test needs a failure that happens once or twice
    /// and then stops, while the `fail_*` methods fail every later call. A queued failure at
    /// `Begin` leaves no `Begin` line in the journal.
    pub fn fail_next(&self, point: FaultPoint, fault: Fault, times: u32) {
        let mut shared = self.shared.lock();
        for _ in 0..times {
            shared.queued_faults.push((point, fault));
        }
    }

    /// Make every later `append_audit` fail with `fault`.
    pub fn fail_append_audit(&self, fault: Fault) {
        self.shared.lock().faults.append_audit = Some(fault);
    }

    /// Make every later `finish` fail with `fault`.
    pub fn fail_finish(&self, fault: Fault) {
        self.shared.lock().faults.finish = Some(fault);
    }

    /// Make every later `insert_unit` fail with `fault`.
    ///
    /// The domain check of the command runs first and passes, so the failure models the
    /// second line of the database (`unit_path_unique`, `unit_parent_exists`).
    pub fn fail_insert_unit(&self, fault: Fault) {
        self.shared.lock().faults.insert_unit = Some(fault);
    }

    /// Make every later `insert_struct` fail with `fault`.
    ///
    /// The domain check of the command runs first and passes, so the failure models the
    /// second line of the database (`plc_type_named_unique`).
    pub fn fail_insert_struct(&self, fault: Fault) {
        self.shared.lock().faults.insert_struct = Some(fault);
    }

    /// Make every later `insert_version` fail with `fault`.
    ///
    /// The domain check of the command runs first and passes, so the failure models the
    /// second line of the database (`recipe_version_one_live_draft`).
    pub fn fail_insert_version(&self, fault: Fault) {
        self.shared.lock().faults.insert_version = Some(fault);
    }

    /// Run `hook` once, just before the next `read_draft` of any transaction.
    ///
    /// The method solves one problem: a request can read the root, and another request can
    /// commit before the first request reads the draft. The hook makes that commit. A second
    /// call replaces a hook that did not run yet.
    pub fn on_read_draft(&self, hook: impl FnOnce(&Self) + Send + 'static) {
        self.shared.lock().read_draft_hook = ReadDraftHook(Some(Box::new(hook)));
    }

    /// Take the hook of `on_read_draft` out of the store and run it without a lock.
    pub(super) fn run_read_draft_hook(&self) {
        let hook = self.shared.lock().read_draft_hook.0.take();
        if let Some(hook) = hook {
            hook(self);
        }
    }

    /// Stop all failures that the `fail_*` methods started.
    pub fn clear_failures(&self) {
        let mut shared = self.shared.lock();
        shared.faults = Faults::default();
        shared.queued_faults.clear();
    }
}

#[async_trait]
impl UnitOfWork for MemoryStore {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        Ok(Box::new(self.begin_memory(isolation)?))
    }
}
