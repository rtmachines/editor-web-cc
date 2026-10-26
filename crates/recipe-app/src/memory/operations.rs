//! In-memory `OperationsReads` (task 3.10, design D6).

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::{SecurityEvent, SecurityEventKind};
use recipe_domain::identity::Permission;
use recipe_domain::setting::{SettingKey, SettingValue};
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;

use super::call_faults::CallFaults;
use super::instants::earlier;
use super::sink::RecordedEvent;
use super::{Fault, MemoryState, MemoryStore};
use crate::auth::{Operation, Requirement};
use crate::error::StorageError;
use crate::ports::APPLICATION_ROLE;
use crate::ports::{
    Isolation, OperationsReads, OperationsSnapshot, ReplicationState, RoleCheck, SnapshotWindows,
    TimeStateKey,
};

/// A method of [`OperationsReads`]. A fault and a call counter belong to one method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationsMethod {
    /// The method `snapshot`.
    Snapshot,
    /// The method `database_now`.
    DatabaseNow,
    /// The method `role_check`.
    RoleCheck,
    /// The method `state`.
    State,
}

impl OperationsMethod {
    /// Every method of the port.
    pub const ALL: [Self; 4] = [
        Self::Snapshot,
        Self::DatabaseNow,
        Self::RoleCheck,
        Self::State,
    ];
}

/// The facts that a test sets and the adapter cannot read from the store.
#[derive(Clone, Debug)]
struct Facts {
    /// The state of the WAL receiver.
    replication: ReplicationState,
    /// The facts of the role check.
    role_check: RoleCheck,
}

impl Default for Facts {
    /// Build the facts of a healthy system: a connected receiver with no lag and the right role.
    fn default() -> Self {
        Self {
            replication: ReplicationState {
                connected: true,
                lag_bytes: Some(0),
            },
            role_check: RoleCheck {
                current_user: APPLICATION_ROLE.to_owned(),
                is_superuser: false,
                owns_relations: false,
            },
        }
    }
}

/// In-memory adapter of [`OperationsReads`] on a [`MemoryStore`].
///
/// The type solves one problem: the tests of the jobs `alerts` and `clock_check` and of the
/// start need the database values without a database. The adapter takes the time from the clock
/// of the store, the events from its sink, and the state keys from its `app_setting` rows. A test
/// sets the replication state and the role check. Each call reads in one `READ ONLY` transaction.
/// A fault fires before the read. A clone shares the facts, the faults, and the store.
#[derive(Clone, Debug)]
pub struct MemoryOperations {
    /// The store that holds the rows, the clock, and the sink.
    store: MemoryStore,
    /// The replication state and the role check.
    facts: Arc<Mutex<Facts>>,
    /// The planned faults and the call counters of the methods.
    faults: Arc<Mutex<CallFaults<OperationsMethod, Fault>>>,
}

impl MemoryOperations {
    /// Build an adapter that reads `store`. The replication and the role are healthy.
    #[must_use]
    pub fn new(store: MemoryStore) -> Self {
        Self {
            store,
            facts: Arc::default(),
            faults: Arc::default(),
        }
    }

    /// Set the state of the WAL receiver that `snapshot` returns.
    pub fn set_replication(&self, replication: ReplicationState) {
        self.facts.lock().replication = replication;
    }

    /// Set the facts that `role_check` returns.
    pub fn set_role_check(&self, role_check: RoleCheck) {
        self.facts.lock().role_check = role_check;
    }

    /// Write `text` as the stored text of a state key, with no check of its form.
    ///
    /// The method solves one problem: a test must prove that a bad text gives a `StorageError`
    /// and not "no value". The write creates the row or changes its value.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` when `text` is not a valid setting value.
    pub fn seed_state_text(&self, key: TimeStateKey, text: &str) -> Result<(), StorageError> {
        let key = state_setting_key(key)?;
        let text = SettingValue::parse(text)
            .map_err(|_| StorageError::invariant("a seeded text is not a setting value"))?;
        self.store
            .update_state(|state, now| state.upsert_setting(key, text, now))
    }

    /// Make call number `call` of `method` fail with `fault`. Later calls succeed.
    pub fn fail_on_call(&self, method: OperationsMethod, call: u64, fault: Fault) {
        self.faults.lock().fail_on_call(method, call, fault);
    }

    /// Make call number `call` of `method` and every later call fail with `fault`.
    pub fn fail_from_call(&self, method: OperationsMethod, call: u64, fault: Fault) {
        self.faults.lock().fail_from_call(method, call, fault);
    }

    /// Stop every planned fault. The call counters keep their values.
    pub fn clear_faults(&self) {
        self.faults.lock().clear();
    }

    /// Return the number of calls that `method` received.
    #[must_use]
    pub fn calls(&self, method: OperationsMethod) -> u64 {
        self.faults.lock().calls(method)
    }

    /// Count one call of `method` and return its planned fault as an error.
    fn begin_call(&self, method: OperationsMethod) -> Result<(), StorageError> {
        match self.faults.lock().next(method) {
            Some(fault) => Err(fault.into()),
            None => Ok(()),
        }
    }
}

/// Return the `app_setting` key of a time state key.
fn state_setting_key(key: TimeStateKey) -> Result<SettingKey, StorageError> {
    SettingKey::parse(key.key())
        .map_err(|_| StorageError::invariant("a state key is not a setting key"))
}

/// Read one state key as a time. A bad text form gives a `StorageError`, never `None`.
fn read_time_state(
    state: &MemoryState,
    key: TimeStateKey,
) -> Result<Option<Timestamp>, StorageError> {
    let key = state_setting_key(key)?;
    state
        .settings
        .get(&key)
        .map(|row| {
            TimeStateKey::parse_text(row.value.as_str()).ok_or_else(|| {
                StorageError::invariant(
                    "a stored state value is not in the text form of ADR-0073 (i)",
                )
            })
        })
        .transpose()
}

/// Return true when `event` is a `forbidden` event of an operation that needs `UserManage`.
fn is_user_manage_denial(event: &SecurityEvent) -> bool {
    event
        .detail()
        .and_then(|detail| detail.get("operation"))
        .and_then(Value::as_str)
        .and_then(|name| Operation::ALL.into_iter().find(|op| op.name() == name))
        .is_some_and(|op| op.requirement() == Requirement::Permission(Permission::UserManage))
}

/// Return true when `event` counts for the condition "security events".
fn is_security_alert(event: &SecurityEvent) -> bool {
    match event.kind() {
        SecurityEventKind::CsrfRejected => true,
        SecurityEventKind::Forbidden => is_user_manage_denial(event),
        _ => false,
    }
}

/// Return the sum of `count` of the `login_failed` events after `since`.
fn failed_logins_after(events: &[RecordedEvent], since: Timestamp) -> u64 {
    events
        .iter()
        .filter(|recorded| {
            recorded.at > since && recorded.event.kind() == SecurityEventKind::LoginFailed
        })
        .fold(0_u64, |sum, recorded| {
            sum.saturating_add(u64::from(recorded.event.count()))
        })
}

/// Return the number of events after `since` that count for "security events".
fn security_alerts_after(events: &[RecordedEvent], since: Timestamp) -> u64 {
    let count = events
        .iter()
        .filter(|recorded| recorded.at > since && is_security_alert(&recorded.event))
        .count();
    u64::try_from(count).unwrap_or(u64::MAX)
}

/// Return the time of the newest `audit_chain_broken` event.
fn newest_chain_break(events: &[RecordedEvent]) -> Option<Timestamp> {
    events
        .iter()
        .filter(|recorded| recorded.event.kind() == SecurityEventKind::AuditChainBroken)
        .map(|recorded| recorded.at)
        .max()
}

/// Build the snapshot of one run of the job `alerts` from the state and the events.
fn build_snapshot(
    state: &MemoryState,
    now: Timestamp,
    events: &[RecordedEvent],
    windows: &SnapshotWindows,
    replication: ReplicationState,
) -> Result<OperationsSnapshot, StorageError> {
    Ok(OperationsSnapshot {
        database_now: now,
        replication,
        failed_logins: failed_logins_after(events, earlier(now, windows.failed_logins)),
        security_alerts: security_alerts_after(events, earlier(now, windows.security)),
        audit_chain_broken_at: newest_chain_break(events),
        audit_last_check_at: read_time_state(state, TimeStateKey::AuditLastCheckAt)?,
        first_setting_at: state.settings.values().map(|row| row.created_at).min(),
    })
}

#[async_trait]
impl OperationsReads for MemoryOperations {
    async fn snapshot(
        &self,
        windows: &SnapshotWindows,
    ) -> Result<OperationsSnapshot, StorageError> {
        self.begin_call(OperationsMethod::Snapshot)?;
        // The store holds its lock inside the transaction, so the events are read before it.
        let events = self.store.sink().recorded();
        let replication = self.facts.lock().replication;
        self.store
            .pool_transaction(Isolation::ReadOnly, |state, now, _journal| {
                build_snapshot(state, now, &events, windows, replication)
            })?
    }

    async fn database_now(&self) -> Result<Timestamp, StorageError> {
        self.begin_call(OperationsMethod::DatabaseNow)?;
        self.store
            .pool_transaction(Isolation::ReadOnly, |_state, now, _journal| now)
    }

    async fn role_check(&self) -> Result<RoleCheck, StorageError> {
        self.begin_call(OperationsMethod::RoleCheck)?;
        let role_check = self.facts.lock().role_check.clone();
        self.store
            .pool_transaction(Isolation::ReadOnly, |_state, _now, _journal| role_check)
    }

    async fn state(&self, key: TimeStateKey) -> Result<Option<Timestamp>, StorageError> {
        self.begin_call(OperationsMethod::State)?;
        self.store
            .pool_transaction(Isolation::ReadOnly, |state, _now, _journal| {
                read_time_state(state, key)
            })?
    }
}

#[cfg(test)]
mod tests;
