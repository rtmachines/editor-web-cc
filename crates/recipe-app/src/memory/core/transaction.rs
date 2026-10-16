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
use crate::ports::{
    AttemptRepository, AuditWritten, DraftRepository, Isolation, NonEmpty, PlantRepository,
    RecipeReads, RecipeRepository, SessionRepository, Transaction, TypeCatalog, UnitLookup,
    UserRepository,
};

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
}

impl MemoryTransaction {
    pub(super) const fn new(
        shared: Arc<Mutex<Shared>>,
        state: MemoryState,
        now: Timestamp,
        isolation: Isolation,
    ) -> Self {
        Self {
            shared,
            state,
            pending_audit: Vec::new(),
            now,
            isolation,
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

    /// Return a copy of the latest committed state, not the working copy of this transaction.
    pub(crate) fn committed_state(&self) -> MemoryState {
        self.shared.lock().state.clone()
    }

    /// Return a copy of the committed audit entries with their commit times, in commit order.
    ///
    /// The entries of this transaction are not in the result.
    pub(crate) fn committed_audit(&self) -> Vec<(Timestamp, AuditEntry)> {
        let shared = self.shared.lock();
        shared
            .audit_times
            .iter()
            .copied()
            .zip(shared.audit.iter().cloned())
            .collect()
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
