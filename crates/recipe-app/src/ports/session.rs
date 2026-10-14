//! Ports of the `session` table (`data-model.md` section 3.6, design D5 and D10).

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::identity::{CsrfToken, Login, Role, SessionId, UserId};
use recipe_domain::timestamp::Timestamp;

use crate::error::StorageError;

/// Text that `Debug` prints in place of a secret.
const REDACTED: &str = "[redacted]";

/// The two timeouts that `SessionStore::authenticate` applies.
///
/// The type solves one problem: the statement needs both limits, and they come from
/// `session.idle_timeout` and `session.absolute_timeout` (`reliability.md` section 7.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionLimits {
    /// The idle time after which a session ends. A valid request moves `expires_at` by it.
    pub idle_timeout: Duration,
    /// The age after which a session ends, counted from `created_at`.
    pub absolute_timeout: Duration,
}

/// The values of a new session row.
///
/// The store sets `created_at`, `last_seen_at`, and `expires_at = now + idle_timeout`.
/// `Debug` redacts the session id and the CSRF token.
#[derive(Clone)]
pub struct NewSession {
    /// The SHA-256 of the cookie token.
    pub id: SessionId,
    /// The owner of the session.
    pub user_id: UserId,
    /// The `session_epoch` of the user at the insert.
    pub epoch: i64,
    /// The CSRF token of the session.
    pub csrf_token: CsrfToken,
    /// The idle timeout that sets the first `expires_at`.
    pub idle_timeout: Duration,
}

impl fmt::Debug for NewSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NewSession")
            .field("id", &REDACTED)
            .field("user_id", &self.user_id)
            .field("epoch", &self.epoch)
            .field("csrf_token", &REDACTED)
            .field("idle_timeout", &self.idle_timeout)
            .finish()
    }
}

/// A valid session with the data of its user (design D10).
///
/// The type solves one problem: `authenticate` returns the session and the user columns in one
/// statement, so the role is never older than the request. `Debug` redacts the session id and the
/// CSRF token.
#[derive(Clone)]
pub struct SessionRow {
    /// The SHA-256 of the cookie token.
    pub id: SessionId,
    /// The owner of the session.
    pub user_id: UserId,
    /// The login of the owner.
    pub login: Login,
    /// The role of the owner now.
    pub role: Role,
    /// Whether the owner must change the password.
    pub must_change_password: bool,
    /// The epoch of the session. It equals the `session_epoch` of the owner.
    pub epoch: i64,
    /// The CSRF token of the session.
    pub csrf_token: CsrfToken,
    /// The time of the login.
    pub created_at: Timestamp,
    /// The time of this request.
    pub last_seen_at: Timestamp,
    /// The idle deadline after this request.
    pub expires_at: Timestamp,
}

impl fmt::Debug for SessionRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionRow")
            .field("id", &REDACTED)
            .field("user_id", &self.user_id)
            .field("login", &self.login)
            .field("role", &self.role)
            .field("must_change_password", &self.must_change_password)
            .field("epoch", &self.epoch)
            .field("csrf_token", &REDACTED)
            .field("created_at", &self.created_at)
            .field("last_seen_at", &self.last_seen_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Port of the `session` rows in one transaction.
///
/// The port solves one problem: `Login`, `Logout`, and the administrative commands insert and
/// delete sessions in the transaction of the command, so a rollback keeps the sessions.
#[async_trait]
pub trait SessionRepository: Send {
    /// Insert a session. The store sets the three times from the time of the transaction.
    ///
    /// An unknown user gives `MissingReference`. A known id gives `Duplicate`.
    async fn insert(&mut self, session: &NewSession) -> Result<(), StorageError>;

    /// Delete one session. Return `true` when a row existed.
    async fn delete(&mut self, id: &SessionId) -> Result<bool, StorageError>;

    /// Delete every session of a user. Return the number of deleted rows.
    async fn delete_all_of_user(&mut self, user: UserId) -> Result<u64, StorageError>;
}

/// Pool-level port that authenticates one request (design D10).
///
/// The port solves one problem: every request touches its session, and the touch must not wait
/// for a command. One statement checks the session and moves its times. It runs in its own
/// transaction, also in read-only mode (ADR-0035 k).
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Check a session and move `last_seen_at` and `expires_at` when it is valid.
    ///
    /// A missing session, an idle session, a session older than `absolute_timeout`, an inactive
    /// user, and another epoch all give `None`. A concurrent delete therefore gives `None`.
    async fn authenticate(
        &self,
        id: &SessionId,
        limits: &SessionLimits,
    ) -> Result<Option<SessionRow>, StorageError>;
}
