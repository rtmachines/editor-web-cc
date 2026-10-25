//! In-memory `MaintenanceStore` (task 3.10, design D6).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::setting::{SettingKey, SettingValue};
use recipe_domain::timestamp::Timestamp;

use super::call_faults::CallFaults;
use super::instants::earlier;
use super::{Fault, MemoryStore};
use crate::error::StorageError;
use crate::ports::{AttemptId, Isolation, MaintenanceStore, StateValue};

/// The number of `security_event` rows that one call of `security_event_purge()` deletes.
///
/// The value equals `security_event.purge_batch` in `reliability.md` section 7.11.
pub const SECURITY_PURGE_BATCH: u64 = 5_000;

/// A method of [`MaintenanceStore`]. A fault and a call counter belong to one method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaintenanceMethod {
    /// The method `delete_expired_sessions`.
    DeleteExpiredSessions,
    /// The method `purge_login_attempts`.
    PurgeLoginAttempts,
    /// The method `purge_security_events`.
    PurgeSecurityEvents,
    /// The method `record_state`.
    RecordState,
}

impl MaintenanceMethod {
    /// Every method of the port.
    pub const ALL: [Self; 4] = [
        Self::DeleteExpiredSessions,
        Self::PurgeLoginAttempts,
        Self::PurgeSecurityEvents,
        Self::RecordState,
    ];
}

/// In-memory adapter of [`MaintenanceStore`] on a [`MemoryStore`].
///
/// The type solves one problem: the tests of the jobs `cleanup`, `audit_chain_check`, and
/// `heartbeat` need the writes of the port without a database. Each method runs in its own short
/// transaction, so a fault on the n-th call keeps the batches of the calls before it. A fault
/// fires before the transaction, so the failed call changes nothing. A clone shares the faults,
/// the counters, and the store.
#[derive(Clone, Debug)]
pub struct MemoryMaintenance {
    /// The store that holds the rows.
    store: MemoryStore,
    /// The planned faults and the call counters of the methods.
    faults: Arc<Mutex<CallFaults<MaintenanceMethod, Fault>>>,
}

impl MemoryMaintenance {
    /// Build an adapter that reads and writes the rows of `store`.
    #[must_use]
    pub fn new(store: MemoryStore) -> Self {
        Self {
            store,
            faults: Arc::default(),
        }
    }

    /// Add `count` security events that are older than the retention.
    ///
    /// The sink of the store keeps no old rows, so the adapter keeps a backlog counter.
    /// `purge_security_events` takes at most [`SECURITY_PURGE_BATCH`] rows from the counter.
    pub fn seed_expired_security_events(&self, count: u64) {
        self.store.update_state(|state, _now| {
            state.expired_security_events = state.expired_security_events.saturating_add(count);
        });
    }

    /// Return the number of security events that the next purges can still delete.
    #[must_use]
    pub fn expired_security_events(&self) -> u64 {
        self.store.state().expired_security_events
    }

    /// Make call number `call` of `method` fail with `fault`. Later calls succeed.
    pub fn fail_on_call(&self, method: MaintenanceMethod, call: u64, fault: Fault) {
        self.faults.lock().fail_on_call(method, call, fault);
    }

    /// Make call number `call` of `method` and every later call fail with `fault`.
    pub fn fail_from_call(&self, method: MaintenanceMethod, call: u64, fault: Fault) {
        self.faults.lock().fail_from_call(method, call, fault);
    }

    /// Stop every planned fault. The call counters keep their values.
    pub fn clear_faults(&self) {
        self.faults.lock().clear();
    }

    /// Return the number of calls that `method` received.
    #[must_use]
    pub fn calls(&self, method: MaintenanceMethod) -> u64 {
        self.faults.lock().calls(method)
    }

    /// Count one call of `method` and return its planned fault as an error.
    fn begin_call(&self, method: MaintenanceMethod) -> Result<(), StorageError> {
        match self.faults.lock().next(method) {
            Some(fault) => Err(fault.into()),
            None => Ok(()),
        }
    }
}

/// Return a row count as `u64`.
fn count(rows: usize) -> Result<u64, StorageError> {
    u64::try_from(rows).map_err(|_| StorageError::invariant("a purge counts too many rows"))
}

/// Return the key and the text of the row that `record_state` writes.
///
/// The text comes from `StateValue::to_text` (ADR-0073 (i)). A value that has no text gives a
/// `StorageError`, as in `recipe-db`.
fn state_row(value: &StateValue) -> Result<(SettingKey, SettingValue), StorageError> {
    let text = value.to_text().ok_or_else(|| {
        StorageError::invariant(match value {
            StateValue::AuditLastCheckSeq(_) => "a chain sequence number is negative",
            StateValue::AuditLastCheckAt(_) | StateValue::HeartbeatLastAt(_) => {
                "an instant has no four digit year in UTC"
            }
        })
    })?;
    let key = SettingKey::parse(value.key())
        .map_err(|_| StorageError::invariant("a state key is not a setting key"))?;
    let text = SettingValue::parse(&text)
        .map_err(|_| StorageError::invariant("a state text is not a setting value"))?;
    Ok((key, text))
}

#[async_trait]
impl MaintenanceStore for MemoryMaintenance {
    async fn delete_expired_sessions(
        &self,
        absolute_timeout: Duration,
    ) -> Result<u64, StorageError> {
        self.begin_call(MaintenanceMethod::DeleteExpiredSessions)?;
        self.store
            .pool_transaction(Isolation::ReadCommitted, |state, now, _journal| {
                let oldest_allowed = earlier(now, absolute_timeout);
                let before = state.sessions.len();
                state.sessions.retain(|_id, session| {
                    session.expires_at >= now && session.created_at >= oldest_allowed
                });
                count(before.saturating_sub(state.sessions.len()))
            })?
    }

    async fn purge_login_attempts(
        &self,
        retention: Duration,
        batch: u32,
    ) -> Result<u64, StorageError> {
        self.begin_call(MaintenanceMethod::PurgeLoginAttempts)?;
        self.store
            .pool_transaction(Isolation::ReadCommitted, |state, now, _journal| {
                let cutoff = earlier(now, retention);
                let mut old: Vec<(Timestamp, AttemptId)> = state
                    .attempts
                    .values()
                    .filter(|row| row.attempted_at < cutoff)
                    .map(|row| (row.attempted_at, row.id))
                    .collect();
                old.sort_unstable();
                old.truncate(usize::try_from(batch).unwrap_or(usize::MAX));
                for (_attempted_at, id) in &old {
                    state.attempts.remove(id);
                }
                count(old.len())
            })?
    }

    async fn purge_security_events(&self) -> Result<u64, StorageError> {
        self.begin_call(MaintenanceMethod::PurgeSecurityEvents)?;
        self.store
            .pool_transaction(Isolation::ReadCommitted, |state, _now, _journal| {
                let deleted = state.expired_security_events.min(SECURITY_PURGE_BATCH);
                state.expired_security_events =
                    state.expired_security_events.saturating_sub(deleted);
                deleted
            })
    }

    async fn record_state(&self, value: &StateValue) -> Result<(), StorageError> {
        self.begin_call(MaintenanceMethod::RecordState)?;
        let (key, text) = state_row(value)?;
        self.store
            .pool_transaction(Isolation::ReadCommitted, |state, now, _journal| {
                state.upsert_setting(key, text, now)
            })?
    }
}

#[cfg(test)]
mod tests;
