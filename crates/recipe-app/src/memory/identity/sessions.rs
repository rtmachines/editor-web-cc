//! In-memory `SessionRepository`.

use async_trait::async_trait;
use recipe_domain::identity::{SessionId, UserId};

use super::{StoredSession, after};
use crate::error::StorageError;
use crate::memory::MemoryTransaction;
use crate::memory::rows::{duplicate, missing_reference};
use crate::ports::{NewSession, SessionRepository};

/// The name of the primary key of `session`.
const SESSION_PKEY: &str = "session_pkey";
/// The name of the foreign key from `session` to `app_user`.
const SESSION_USER_FKEY: &str = "session_user_id_fkey";

#[async_trait]
impl SessionRepository for MemoryTransaction {
    async fn insert(&mut self, session: &NewSession) -> Result<(), StorageError> {
        let state = self.state();
        if !state.users.contains_key(&session.user_id) {
            return Err(missing_reference(SESSION_USER_FKEY));
        }
        if state.sessions.contains_key(&session.id) {
            return Err(duplicate(SESSION_PKEY));
        }
        let now = self.now();
        let stored = StoredSession {
            user_id: session.user_id,
            epoch: session.epoch,
            csrf_token: session.csrf_token.clone(),
            created_at: now,
            last_seen_at: now,
            expires_at: after(now, session.idle_timeout),
        };
        self.state_mut().sessions.insert(session.id.clone(), stored);
        Ok(())
    }

    async fn delete(&mut self, id: &SessionId) -> Result<bool, StorageError> {
        Ok(self.state_mut().sessions.remove(id).is_some())
    }

    async fn delete_all_of_user(&mut self, user: UserId) -> Result<u64, StorageError> {
        let sessions = &mut self.state_mut().sessions;
        let before = sessions.len();
        sessions.retain(|_id, stored| stored.user_id != user);
        let removed = before.saturating_sub(sessions.len());
        u64::try_from(removed).map_err(|_| StorageError::invariant("too many sessions"))
    }
}
