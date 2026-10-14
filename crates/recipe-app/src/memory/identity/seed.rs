//! Seed helpers for the identity tables. They need no transaction and no constraint check.

use std::net::IpAddr;

use recipe_domain::identity::TypedLogin;
use recipe_domain::timestamp::Timestamp;

use super::{StoredSession, after};
use crate::memory::MemoryStore;
use crate::ports::{AttemptId, AttemptRow, NewSession, NewUser, UserRow};

impl MemoryStore {
    /// Insert an active user with epoch 0 into the committed state and return the row.
    ///
    /// The time of the clock is `created_at` and `updated_at`. A fixture uses this method to
    /// prepare the state that a command starts from.
    pub fn seed_user(&self, user: &NewUser) -> UserRow {
        self.update_state(|state, now| {
            let row = UserRow {
                id: user.id,
                login: user.login.clone(),
                password_hash: user.password_hash.clone(),
                role: user.role,
                is_active: true,
                session_epoch: 0,
                must_change_password: user.must_change_password,
                deactivated_at: None,
                created_at: now,
                updated_at: now,
            };
            state.users.insert(row.id, row.clone());
            row
        })
    }

    /// Insert a session into the committed state. The time of the clock sets the three times.
    pub fn seed_session(&self, session: &NewSession) {
        self.update_state(|state, now| {
            let stored = StoredSession {
                user_id: session.user_id,
                epoch: session.epoch,
                csrf_token: session.csrf_token.clone(),
                created_at: now,
                last_seen_at: now,
                expires_at: after(now, session.idle_timeout),
            };
            state.sessions.insert(session.id.clone(), stored);
        });
    }

    /// Insert a login attempt with a fixed time into the committed state and return its key.
    pub fn seed_attempt(
        &self,
        login: &TypedLogin,
        client_ip: IpAddr,
        attempted_at: Timestamp,
        succeeded: bool,
    ) -> AttemptId {
        self.update_state(|state, _now| {
            state.last_attempt_key = state.last_attempt_key.saturating_add(1);
            let id = AttemptId::from(state.last_attempt_key);
            let row = AttemptRow {
                id,
                login: login.clone(),
                client_ip,
                attempted_at,
                succeeded,
            };
            state.attempts.insert(id, row);
            id
        })
    }
}
