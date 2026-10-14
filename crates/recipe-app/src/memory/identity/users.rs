//! In-memory `UserRepository`.

use async_trait::async_trait;
use recipe_domain::identity::{Login, PasswordHash, Role, UserId};
use recipe_domain::timestamp::Timestamp;

use super::images::user_image;
use crate::error::{APP_USER_LOGIN_UNIQUE, StorageError};
use crate::memory::rows::{duplicate, missing_row};
use crate::memory::{LockMode, MemoryTransaction};
use crate::ports::{Locked, NewUser, RowImage, UserRepository, UserRow, UserSummary};

/// The table name of the journal for a user lock.
const USER_TABLE: &str = "app_user";
/// The name of the primary key of `app_user`.
const USER_PKEY: &str = "app_user_pkey";

/// Raise the session epoch by 1, or fail when the number overflows.
fn bump_epoch(row: &mut UserRow) -> Result<(), StorageError> {
    row.session_epoch = row
        .session_epoch
        .checked_add(1)
        .ok_or_else(|| StorageError::invariant("the session epoch overflows"))?;
    Ok(())
}

fn summary_of(row: &UserRow) -> UserSummary {
    UserSummary {
        id: row.id,
        login: row.login.clone(),
        role: row.role,
        is_active: row.is_active,
        must_change_password: row.must_change_password,
        deactivated_at: row.deactivated_at,
        created_at: row.created_at,
    }
}

impl MemoryTransaction {
    fn user_by_login(&self, login: &Login) -> Option<&UserRow> {
        self.state().users.values().find(|row| row.login == *login)
    }

    /// Change one user row, set `updated_at`, and return the image after the change.
    fn change_user(
        &mut self,
        id: UserId,
        edit: impl FnOnce(&mut UserRow, Timestamp) -> Result<(), StorageError>,
    ) -> Result<RowImage, StorageError> {
        let now = self.now();
        let row = self
            .state_mut()
            .users
            .get_mut(&id)
            .ok_or_else(|| missing_row(USER_TABLE))?;
        edit(row, now)?;
        row.updated_at = now;
        Ok(user_image(row))
    }
}

#[async_trait]
impl UserRepository for MemoryTransaction {
    async fn find_by_login(&mut self, login: &Login) -> Result<Option<UserRow>, StorageError> {
        Ok(self.user_by_login(login).cloned())
    }

    async fn lock_by_login(
        &mut self,
        login: &Login,
    ) -> Result<Option<Locked<UserRow>>, StorageError> {
        let Some(row) = self.user_by_login(login).cloned() else {
            return Ok(None);
        };
        self.lock(USER_TABLE, LockMode::ForNoKeyUpdate, &row.id.to_string());
        let before = user_image(&row);
        Ok(Some(Locked { state: row, before }))
    }

    async fn count_active_administrators(&mut self) -> Result<u32, StorageError> {
        let count = self
            .state()
            .users
            .values()
            .filter(|row| row.is_active && row.role == Role::Administrator)
            .count();
        u32::try_from(count).map_err(|_| StorageError::invariant("too many administrators"))
    }

    async fn insert(&mut self, user: &NewUser) -> Result<RowImage, StorageError> {
        let state = self.state();
        if state.users.contains_key(&user.id) {
            return Err(duplicate(USER_PKEY));
        }
        if self.user_by_login(&user.login).is_some() {
            return Err(duplicate(APP_USER_LOGIN_UNIQUE));
        }
        let now = self.now();
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
        let image = user_image(&row);
        self.state_mut().users.insert(row.id, row);
        Ok(image)
    }

    async fn set_role(&mut self, id: UserId, role: Role) -> Result<RowImage, StorageError> {
        self.change_user(id, |row, _now| {
            row.role = role;
            bump_epoch(row)
        })
    }

    async fn deactivate(&mut self, id: UserId) -> Result<RowImage, StorageError> {
        self.change_user(id, |row, now| {
            row.is_active = false;
            row.deactivated_at = Some(now);
            bump_epoch(row)
        })
    }

    async fn activate(&mut self, id: UserId) -> Result<RowImage, StorageError> {
        self.change_user(id, |row, _now| {
            row.is_active = true;
            row.deactivated_at = None;
            Ok(())
        })
    }

    async fn reset_password(
        &mut self,
        id: UserId,
        hash: &PasswordHash,
    ) -> Result<RowImage, StorageError> {
        self.change_user(id, |row, _now| {
            row.password_hash = hash.clone();
            row.must_change_password = true;
            bump_epoch(row)
        })
    }

    async fn change_own_password(
        &mut self,
        id: UserId,
        hash: &PasswordHash,
    ) -> Result<RowImage, StorageError> {
        self.change_user(id, |row, _now| {
            row.password_hash = hash.clone();
            row.must_change_password = false;
            bump_epoch(row)
        })
    }

    async fn list(
        &mut self,
        filter: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<UserSummary>, StorageError> {
        let mut rows: Vec<&UserRow> = self
            .state()
            .users
            .values()
            .filter(|row| filter.is_none_or(|text| row.login.as_str().contains(text)))
            .collect();
        rows.sort_by(|left, right| left.login.cmp(&right.login));
        let size = usize::try_from(page_size).unwrap_or(usize::MAX);
        let skipped = usize::try_from(page)
            .ok()
            .and_then(|number| number.checked_mul(size))
            .unwrap_or(usize::MAX);
        Ok(rows
            .into_iter()
            .skip(skipped)
            .take(size)
            .map(summary_of)
            .collect())
    }

    async fn summary_by_login(
        &mut self,
        login: &Login,
    ) -> Result<Option<UserSummary>, StorageError> {
        Ok(self.user_by_login(login).map(summary_of))
    }
}
