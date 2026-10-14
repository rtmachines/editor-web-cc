//! In-memory identity ports (task 3.4): users, sessions, and login attempts.
//!
//! `UserRepository`, `SessionRepository`, and `AttemptRepository` work on the copy of a
//! transaction. `LoginAttemptStore` and `SessionStore` work on the committed state in one step.

mod attempts;
mod images;
mod pool;
mod seed;
mod sessions;
mod users;

use std::time::Duration;

use recipe_domain::identity::{CsrfToken, UserId};
use recipe_domain::timestamp::Timestamp;

/// The table name that the journal records for the class-1 advisory lock of `Login` T1.
pub const LOGIN_LOCK_TABLE: &str = "advisory_class_1";

/// A row of the table `session` (`data-model.md` section 3.6). The map key is the session id.
#[derive(Clone, Debug)]
pub(crate) struct StoredSession {
    pub(crate) user_id: UserId,
    pub(crate) epoch: i64,
    pub(crate) csrf_token: CsrfToken,
    pub(crate) created_at: Timestamp,
    pub(crate) last_seen_at: Timestamp,
    pub(crate) expires_at: Timestamp,
}

/// Convert a standard duration. A value that does not fit gives the largest duration.
fn to_time(span: Duration) -> time::Duration {
    time::Duration::try_from(span).unwrap_or(time::Duration::MAX)
}

/// Return `instant + span`. The result stops at the largest instant on overflow.
fn after(instant: Timestamp, span: Duration) -> Timestamp {
    Timestamp::from(instant.as_offset_date_time().saturating_add(to_time(span)))
}

/// Return `instant - span`. The result stops at the smallest instant on overflow.
fn before(instant: Timestamp, span: Duration) -> Timestamp {
    Timestamp::from(instant.as_offset_date_time().saturating_sub(to_time(span)))
}

#[cfg(test)]
mod tests;
