//! Ports of the `login_attempt` table (`data-model.md` section 3.7, design D5 and D6).

use std::net::IpAddr;
use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::identity::{AttemptCount, ThrottlePolicy, TypedLogin};
use recipe_domain::timestamp::Timestamp;

use crate::error::StorageError;

/// The primary key of a `login_attempt` row.
///
/// The type solves one problem: T1 returns the key and T2 marks that row, and a bare number
/// could stand for any other counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptId(i64);

impl From<i64> for AttemptId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl AttemptId {
    /// Return the number of the key.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

/// The pair that T1 counts: the typed login and the client address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptKey {
    /// The typed login. It may not match an account.
    pub login: TypedLogin,
    /// The canonical address of the client.
    pub client_ip: IpAddr,
}

/// The result of T1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptOutcome {
    /// The counts that T1 read before it inserted its row.
    pub count: AttemptCount,
    /// The key of the new row, or `None` when the budget of the address was already reached.
    pub attempt: Option<AttemptId>,
}

/// A row of `login_attempt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptRow {
    /// The primary key.
    pub id: AttemptId,
    /// The typed login.
    pub login: TypedLogin,
    /// The address of the client.
    pub client_ip: IpAddr,
    /// The database time of the attempt.
    pub attempted_at: Timestamp,
    /// Whether the password was right.
    pub succeeded: bool,
}

/// The attempts that `UnlockLogin` deleted.
///
/// The type solves one problem: the row image of `LoginUnlocked` holds at most
/// `audit.unlock_rows` attempts, but it also needs the number of all deleted attempts (ADR-0035 q).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletedAttempts {
    /// The newest deleted attempts, newest first, at most `keep`.
    pub newest: Vec<AttemptRow>,
    /// The number of all deleted attempts.
    pub count: u64,
}

/// The throttle state of one login for the query `GetUser`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoginThrottleState {
    /// The failures of the login in the window, over every address.
    pub failures: u32,
    /// The time of the last failure in the window.
    pub last_failure: Option<Timestamp>,
}

/// Port of the `login_attempt` rows in one transaction.
///
/// The port solves one problem: T2 of `Login`, `ChangeOwnPassword`, `UnlockLogin`, and `GetUser`
/// work on the attempt rows inside the transaction of the use case.
#[async_trait]
pub trait AttemptRepository: Send {
    /// Set `succeeded` on one attempt (`UPDATE login_attempt SET succeeded = true`).
    ///
    /// An update that changes no row is a success. `UnlockLogin` may delete the row between T1
    /// and the final transaction of `Login` or `ChangeOwnPassword` (ADR-0035).
    async fn mark_succeeded(&mut self, id: AttemptId) -> Result<(), StorageError>;

    /// Delete every attempt of `login` and return the newest `keep` of them (`UnlockLogin`).
    async fn delete_for_login(
        &mut self,
        login: &TypedLogin,
        keep: u32,
    ) -> Result<DeletedAttempts, StorageError>;

    /// Count the failures of `login` in the last `window`, over every address (`GetUser`).
    async fn throttle_state(
        &mut self,
        login: &TypedLogin,
        window: Duration,
    ) -> Result<LoginThrottleState, StorageError>;
}

/// Pool-level port of T1 of `Login` (design D6).
///
/// The port solves one problem: T1 writes no audit entry, so it cannot use `UnitOfWork::commit`.
/// It runs in its own transaction and commits before the password check.
#[async_trait]
pub trait LoginAttemptStore: Send + Sync {
    /// Take the class-1 advisory lock of the login, count the failures, and insert the attempt.
    ///
    /// The lock is its own statement, so the count sees every attempt that a request with the same
    /// login committed before. The row is a failure. T1 inserts no row when
    /// `ThrottlePolicy::budget_reached` is true for the failures of the address.
    async fn begin_attempt(
        &self,
        key: &AttemptKey,
        policy: &ThrottlePolicy,
    ) -> Result<AttemptOutcome, StorageError>;
}
