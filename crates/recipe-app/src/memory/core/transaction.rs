//! The in-memory transaction.

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::AuditEntry;
use recipe_domain::timestamp::Timestamp;

use recipe_domain::recipe::ParameterKey;

use super::journal::{DraftWrite, JournalEntry, Lock, LockMode, WriteKind};
use super::state::MemoryState;
use super::store::{MemoryStore, Shared};
use crate::error::StorageError;
use crate::memory::FaultPoint;
use crate::memory::sink::RecordedEvent;
use crate::ports::{
    AttemptRepository, AuditReads, AuditWritten, DiffReads, DraftRepository, Isolation, NonEmpty,
    PlantRepository, RecipeReads, RecipeRepository, SessionRepository, SettingRepository,
    Transaction, TypeCatalog, UnitLookup, UserRepository,
};

/// The length of the audit log and of the event list of the sink at the `begin` of a transaction.
///
/// The type solves one problem: a read must see one state. It sees the entries and events that
/// were committed when it began, together with the labels and users of its copy of the state.
#[derive(Clone, Copy, Debug)]
pub(super) struct VisibleLogs {
    /// The number of committed audit entries.
    pub(super) audit: usize,
    /// The number of recorded security events.
    pub(super) events: usize,
}

/// An open in-memory transaction.
///
/// It works on a copy of the state. `finish` replaces the shared state and appends
/// the audit entries in one step. A drop without `finish` discards the copy.
/// A read-only transaction cannot `finish`, so it never replaces the shared state.
/// Two open transactions do not exclude each other. The journal records the lock
/// order and no lock blocks.
#[derive(Debug)]
pub struct MemoryTransaction {
    shared: Arc<Mutex<Shared>>,
    state: MemoryState,
    pending_audit: Vec<AuditEntry>,
    now: Timestamp,
    isolation: Isolation,
    visible: VisibleLogs,
}

impl MemoryTransaction {
    pub(super) const fn new(
        shared: Arc<Mutex<Shared>>,
        state: MemoryState,
        now: Timestamp,
        isolation: Isolation,
        visible: VisibleLogs,
    ) -> Self {
        Self {
            shared,
            state,
            pending_audit: Vec::new(),
            now,
            isolation,
            visible,
        }
    }

    /// Return the fault that `fail_insert_unit` set, or `None`.
    pub(crate) fn insert_unit_fault(&self) -> Option<crate::memory::Fault> {
        self.shared.lock().faults.insert_unit
    }

    /// Return the fault that `fail_insert_struct` set, or `None`.
    pub(crate) fn insert_struct_fault(&self) -> Option<crate::memory::Fault> {
        self.shared.lock().faults.insert_struct
    }

    /// Return the fault that `fail_insert_version` set, or `None`.
    pub(crate) fn insert_version_fault(&self) -> Option<crate::memory::Fault> {
        self.shared.lock().faults.insert_version
    }

    /// Run the hook of `MemoryStore::on_read_draft`, when one is set.
    pub(crate) fn run_read_draft_hook(&self) {
        MemoryStore::from_shared(Arc::clone(&self.shared)).run_read_draft_hook();
    }

    /// Return a copy of the state that `read_draft` sees at the isolation level of the transaction.
    ///
    /// A `RepeatableRead` transaction gets its working copy, which is the state at `begin`. This
    /// models the one snapshot of PostgreSQL in `REPEATABLE READ` (ADR-0050 (f)). Every other
    /// level gets the latest committed state, not the working copy of the transaction.
    pub(crate) fn committed_state(&self) -> MemoryState {
        if self.isolation == Isolation::RepeatableRead {
            return self.state.clone();
        }
        self.shared.lock().state.clone()
    }

    /// Return a copy of the audit entries that were committed at `begin`, with their commit times.
    ///
    /// The entries are in commit order. The entries of this transaction and the entries that
    /// commit after `begin` are not in the result.
    ///
    /// A query uses this method. A command that reads the log after a lock needs the latest log,
    /// as `read_draft` reads the latest state.
    pub(crate) fn audit_at_begin(&self) -> Vec<(Timestamp, AuditEntry)> {
        let shared = self.shared.lock();
        shared
            .audit_times
            .iter()
            .copied()
            .zip(shared.audit.iter().cloned())
            .take(self.visible.audit)
            .collect()
    }

    /// Return a copy of the events that the sink held at `begin`, with their times, in record order.
    ///
    /// A query uses this method. A command that reads the events after a lock needs the latest
    /// events, as `read_draft` reads the latest state.
    pub(crate) fn events_at_begin(&self) -> Vec<RecordedEvent> {
        let mut events = self.shared.lock().sink.recorded();
        events.truncate(self.visible.events);
        events
    }

    /// Return the instant of `now()` in this transaction: the time of the clock at `begin`.
    #[must_use]
    pub const fn now(&self) -> Timestamp {
        self.now
    }

    /// Record a lock call in the journal of the store.
    pub fn lock(&self, table: &'static str, mode: LockMode, key: &str) {
        self.shared.lock().journal.push(JournalEntry::Lock(Lock {
            table,
            mode,
            key: key.to_owned(),
        }));
    }

    /// Record a draft write statement in the log of the store.
    pub fn record_draft_write(&self, kind: WriteKind, key: ParameterKey) {
        self.shared
            .lock()
            .draft_writes
            .push(DraftWrite { kind, key });
    }

    /// Return the working copy of the state.
    #[must_use]
    pub const fn state(&self) -> &MemoryState {
        &self.state
    }

    /// Return the working copy of the state for a write.
    pub const fn state_mut(&mut self) -> &mut MemoryState {
        &mut self.state
    }
}

#[async_trait]
impl Transaction for MemoryTransaction {
    fn plant(&mut self) -> &mut dyn PlantRepository {
        self
    }

    fn units(&mut self) -> &mut dyn UnitLookup {
        self
    }

    fn recipes(&mut self) -> &mut dyn RecipeRepository {
        self
    }

    fn recipe_reads(&mut self) -> &mut dyn RecipeReads {
        self
    }

    fn audit_reads(&mut self) -> &mut dyn AuditReads {
        self
    }

    fn diff_reads(&mut self) -> &mut dyn DiffReads {
        self
    }

    fn drafts(&mut self) -> &mut dyn DraftRepository {
        self
    }

    fn types(&mut self) -> &mut dyn TypeCatalog {
        self
    }

    fn users(&mut self) -> &mut dyn UserRepository {
        self
    }

    fn sessions(&mut self) -> &mut dyn SessionRepository {
        self
    }

    fn attempts(&mut self) -> &mut dyn AttemptRepository {
        self
    }

    fn settings(&mut self) -> &mut dyn SettingRepository {
        self
    }

    async fn append_audit(&mut self, entries: &NonEmpty<AuditEntry>) -> Result<(), StorageError> {
        let mut shared = self.shared.lock();
        if let Some(fault) = shared.take_queued(FaultPoint::AppendAudit) {
            return Err(fault.into());
        }
        if let Some(fault) = shared.faults.append_audit {
            return Err(fault.into());
        }
        drop(shared);
        self.pending_audit.extend(entries.iter().cloned());
        Ok(())
    }

    async fn finish(self: Box<Self>, _written: AuditWritten) -> Result<(), StorageError> {
        let this = *self;
        // A query transaction reads one locked state (ADR-0042 (e)), so both levels are read only.
        if matches!(
            this.isolation,
            Isolation::ReadOnly | Isolation::RepeatableRead
        ) {
            return Err(StorageError::invariant(
                "a read-only transaction cannot finish",
            ));
        }
        let mut shared = this.shared.lock();
        if let Some(fault) = shared.take_queued(FaultPoint::Finish) {
            return Err(fault.into());
        }
        if let Some(fault) = shared.faults.finish {
            return Err(fault.into());
        }
        shared.state = this.state;
        shared
            .audit_times
            .extend(std::iter::repeat_n(this.now, this.pending_audit.len()));
        shared.audit.extend(this.pending_audit);
        Ok(())
    }
}

mod settings;
#[cfg(test)]
mod tests;
