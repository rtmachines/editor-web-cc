//! In-memory `LoginAttemptStore` and `SessionStore`: pool-level ports with their own transaction.

use async_trait::async_trait;
use recipe_domain::identity::{AttemptCount, SessionId, ThrottlePolicy};
use recipe_domain::timestamp::Timestamp;

use super::{LOGIN_LOCK_TABLE, StoredSession, after, before};
use crate::error::StorageError;
use crate::memory::{JournalEntry, Lock, LockMode, MemoryState, MemoryStore};
use crate::ports::{
    AttemptId, AttemptKey, AttemptOutcome, AttemptRow, Isolation, LoginAttemptStore, SessionLimits,
    SessionRow, SessionStore,
};

/// Run the statements of T1 on the committed state.
fn begin(
    state: &mut MemoryState,
    now: Timestamp,
    journal: &mut Vec<JournalEntry>,
    key: &AttemptKey,
    policy: &ThrottlePolicy,
) -> Result<AttemptOutcome, StorageError> {
    // The adapter stores and compares the canonical form, like the PostgreSQL adapter.
    let client_ip = key.client_ip.to_canonical();
    journal.push(JournalEntry::Lock(Lock {
        table: LOGIN_LOCK_TABLE,
        mode: LockMode::Advisory,
        key: key.login.as_str().to_owned(),
    }));
    let start = before(now, policy.window);
    let failures: Vec<&AttemptRow> = state
        .attempts
        .values()
        .filter(|row| !row.succeeded && row.attempted_at > start && row.client_ip == client_ip)
        .collect();
    let pair: Vec<&&AttemptRow> = failures
        .iter()
        .filter(|row| row.login == key.login)
        .collect();
    let count = AttemptCount {
        pair_failures: u32::try_from(pair.len()).unwrap_or(u32::MAX),
        last_pair_failure: pair.iter().map(|row| row.attempted_at).max(),
        address_failures: u32::try_from(failures.len()).unwrap_or(u32::MAX),
        oldest_address_failure: failures.iter().map(|row| row.attempted_at).min(),
        now,
    };
    if policy.budget_reached(count.address_failures) {
        return Ok(AttemptOutcome {
            count,
            attempt: None,
        });
    }
    let next = state
        .last_attempt_key
        .checked_add(1)
        .ok_or_else(|| StorageError::invariant("the attempt key overflows"))?;
    state.last_attempt_key = next;
    let id = AttemptId::from(next);
    state.attempts.insert(
        id,
        AttemptRow {
            id,
            login: key.login.clone(),
            client_ip,
            attempted_at: now,
            succeeded: false,
        },
    );
    Ok(AttemptOutcome {
        count,
        attempt: Some(id),
    })
}

/// Run the statement of `SessionStore::authenticate` on the committed state (design D10).
fn touch(
    state: &mut MemoryState,
    now: Timestamp,
    id: &SessionId,
    limits: &SessionLimits,
) -> Option<SessionRow> {
    let session = state.sessions.get(id)?;
    let user = state.users.get(&session.user_id)?;
    let oldest_allowed = before(now, limits.absolute_timeout);
    let is_valid = session.expires_at > now
        && session.created_at > oldest_allowed
        && user.is_active
        && session.epoch == user.session_epoch;
    if !is_valid {
        return None;
    }
    let (login, role, must_change_password) =
        (user.login.clone(), user.role, user.must_change_password);
    let StoredSession {
        user_id,
        epoch,
        csrf_token,
        created_at,
        ..
    } = session.clone();
    let touched = state.sessions.get_mut(id)?;
    touched.last_seen_at = now;
    touched.expires_at = after(now, limits.idle_timeout);
    Some(SessionRow {
        id: id.clone(),
        user_id,
        login,
        role,
        must_change_password,
        epoch,
        csrf_token,
        created_at,
        last_seen_at: touched.last_seen_at,
        expires_at: touched.expires_at,
    })
}

#[async_trait]
impl LoginAttemptStore for MemoryStore {
    async fn begin_attempt(
        &self,
        key: &AttemptKey,
        policy: &ThrottlePolicy,
    ) -> Result<AttemptOutcome, StorageError> {
        self.pool_transaction(Isolation::ReadCommitted, |state, now, journal| {
            begin(state, now, journal, key, policy)
        })?
    }
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn authenticate(
        &self,
        id: &SessionId,
        limits: &SessionLimits,
    ) -> Result<Option<SessionRow>, StorageError> {
        self.pool_transaction(Isolation::ReadCommitted, |state, now, _journal| {
            touch(state, now, id, limits)
        })
    }
}
