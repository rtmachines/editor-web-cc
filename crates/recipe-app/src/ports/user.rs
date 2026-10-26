//! Port of the `app_user` table (`data-model.md` sections 3.5 and 6, design D5).

use std::fmt;

use async_trait::async_trait;
use recipe_domain::identity::{Login, PasswordHash, Role, UserId};
use recipe_domain::timestamp::Timestamp;

use super::{Locked, RowImage};
use crate::error::StorageError;

/// Text that `Debug` prints in place of a secret.
const REDACTED: &str = "[redacted]";

/// A row of `app_user`, with its password hash.
///
/// The type solves one problem: the login check and the final transaction of `Login` need the
/// hash and the epoch, and no other code may see the hash. `Debug` redacts it. A row image and a
/// query result never hold the hash (ADR-0029).
#[derive(Clone, PartialEq, Eq)]
pub struct UserRow {
    /// The primary key.
    pub id: UserId,
    /// The login of the user.
    pub login: Login,
    /// The Argon2id hash of the password.
    pub password_hash: PasswordHash,
    /// The role of the user.
    pub role: Role,
    /// Whether the user may log in.
    pub is_active: bool,
    /// The epoch that a session must carry (`session_epoch`).
    pub session_epoch: i64,
    /// Whether the user must change the password before any other request.
    pub must_change_password: bool,
    /// The time of the deactivation, or `None` for an active user.
    pub deactivated_at: Option<Timestamp>,
    /// The time of the insert.
    pub created_at: Timestamp,
    /// The time of the last change.
    pub updated_at: Timestamp,
}

impl fmt::Debug for UserRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserRow")
            .field("id", &self.id)
            .field("login", &self.login)
            .field("password_hash", &REDACTED)
            .field("role", &self.role)
            .field("is_active", &self.is_active)
            .field("session_epoch", &self.session_epoch)
            .field("must_change_password", &self.must_change_password)
            .field("deactivated_at", &self.deactivated_at)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// A user as the queries `ListUsers` and `GetUser` show it. It has no hash and no epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserSummary {
    /// The primary key.
    pub id: UserId,
    /// The login of the user.
    pub login: Login,
    /// The role of the user.
    pub role: Role,
    /// Whether the user may log in.
    pub is_active: bool,
    /// Whether the user must change the password.
    pub must_change_password: bool,
    /// The time of the deactivation, or `None` for an active user.
    pub deactivated_at: Option<Timestamp>,
    /// The time of the insert.
    pub created_at: Timestamp,
}

/// The values of a new `app_user` row.
///
/// The type solves one problem: the insert needs the hash, and the use case must not pass it in
/// a loose string. `Debug` redacts the hash.
#[derive(Clone, PartialEq, Eq)]
pub struct NewUser {
    /// The identifier that the use case chose.
    pub id: UserId,
    /// The login of the user.
    pub login: Login,
    /// The Argon2id hash of the first password.
    pub password_hash: PasswordHash,
    /// The role of the user.
    pub role: Role,
    /// Whether the first request must change the password.
    pub must_change_password: bool,
}

impl fmt::Debug for NewUser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NewUser")
            .field("id", &self.id)
            .field("login", &self.login)
            .field("password_hash", &REDACTED)
            .field("role", &self.role)
            .field("must_change_password", &self.must_change_password)
            .finish()
    }
}

/// Port of the `app_user` rows in one transaction.
///
/// The port solves one problem: the nine Identity commands and the user queries need one set of
/// statements, in memory and in PostgreSQL. Each write method runs exactly the column changes of
/// its row in `data-model.md` section 6 and returns the row image after the change, without
/// `password_hash`. A write on a missing row is a defect, because the use case locks the row first.
/// Deleting the sessions of the user is a separate call of `SessionRepository`.
#[async_trait]
pub trait UserRepository: Send {
    /// Read the user with `login` without a lock. An unknown login gives `None`.
    async fn find_by_login(&mut self, login: &Login) -> Result<Option<UserRow>, StorageError>;

    /// Read the user with `login` and take `FOR NO KEY UPDATE` (ADR-0015).
    ///
    /// The result holds the row image before any change. An unknown login gives `None` and no lock.
    async fn lock_by_login(
        &mut self,
        login: &Login,
    ) -> Result<Option<Locked<UserRow>>, StorageError>;

    /// Take the guard lock of the last Administrator (ADR-0096 (b)).
    ///
    /// The lock is the advisory lock of class 4 and key 0. It lasts until the end of the
    /// transaction. A second call in the same transaction does not wait. The wait obeys
    /// `db.lock_timeout`, and a timeout gives `StorageError::Deadlock` with `LockTimeout`.
    /// `SetUserRole` and `DeactivateUser` call it first, so the count of
    /// `count_active_administrators` sees each commit of an earlier holder.
    async fn lock_administrator_guard(&mut self) -> Result<(), StorageError>;

    /// Count the active users with the role `administrator` (last-administrator rule).
    async fn count_active_administrators(&mut self) -> Result<u32, StorageError>;

    /// Insert a user. The row is active, has epoch 0, and gets the times of the transaction.
    ///
    /// A duplicate login gives `Duplicate` with `app_user_login_unique`.
    async fn insert(&mut self, user: &NewUser) -> Result<RowImage, StorageError>;

    /// Set the role and raise the epoch by 1 (`SetUserRole`).
    async fn set_role(&mut self, id: UserId, role: Role) -> Result<RowImage, StorageError>;

    /// Clear `is_active`, set `deactivated_at`, and raise the epoch by 1 (`DeactivateUser`).
    async fn deactivate(&mut self, id: UserId) -> Result<RowImage, StorageError>;

    /// Set `is_active` and clear `deactivated_at`. The epoch stays (`ActivateUser`).
    async fn activate(&mut self, id: UserId) -> Result<RowImage, StorageError>;

    /// Replace the hash, set `must_change_password`, and raise the epoch by 1 (`ResetPassword`).
    async fn reset_password(
        &mut self,
        id: UserId,
        hash: &PasswordHash,
    ) -> Result<RowImage, StorageError>;

    /// Replace the hash, clear `must_change_password`, and raise the epoch by 1
    /// (`ChangeOwnPassword`).
    async fn change_own_password(
        &mut self,
        id: UserId,
        hash: &PasswordHash,
    ) -> Result<RowImage, StorageError>;

    /// List users ordered by login in byte order (`ListUsers`, design D9).
    ///
    /// A `filter` keeps the logins that contain it as plain text, so `_` is no wildcard.
    /// `page` counts from 0 and each page holds `page_size` rows.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Other` (`E-DB-008`) when `page * page_size` is above `i64::MAX`.
    /// The SQL adapter and the memory adapter both return it.
    async fn list(
        &mut self,
        filter: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<UserSummary>, StorageError>;

    /// Read one user for the query `GetUser`. An unknown login gives `None`.
    async fn summary_by_login(
        &mut self,
        login: &Login,
    ) -> Result<Option<UserSummary>, StorageError>;
}
