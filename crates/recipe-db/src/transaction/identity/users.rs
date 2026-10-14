//! `UserRepository` on the transaction (task 5.4, `data-model.md` sections 3.5 and 6).
//!
//! Every row image is `to_jsonb(app_user) - 'password_hash'`. The hash never leaves the database
//! except in `find_by_login` and `lock_by_login`, which the login check needs (ADR-0029).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{Locked, NewUser, RowImage, UserRepository, UserRow, UserSummary};
use recipe_domain::identity::{Login, PasswordHash, Role, UserId};
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use sqlx::postgres::PgArguments;
use sqlx::{PgConnection, Postgres};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{corrupt, map_error, missing_row};
use crate::transaction::PgTransaction;

/// The table name of the defects.
const TABLE: &str = "app_user";

const SELECT_BY_LOGIN: &str = "\
SELECT id, login, password_hash, role, is_active, session_epoch, must_change_password, \
    deactivated_at, created_at, updated_at \
FROM app_user WHERE login = $1";

/// `app_user FOR NO KEY UPDATE` (ADR-0015). The image is the row before any change.
const LOCK_BY_LOGIN: &str = "\
SELECT id, login, password_hash, role, is_active, session_epoch, must_change_password, \
    deactivated_at, created_at, updated_at, to_jsonb(u) - 'password_hash' AS image \
FROM app_user u WHERE login = $1 FOR NO KEY UPDATE";

const COUNT_ACTIVE_ADMINISTRATORS: &str = "\
SELECT count(*) FROM app_user WHERE role = 'administrator' AND is_active";

const INSERT: &str = "\
INSERT INTO app_user (id, login, password_hash, role, must_change_password) \
VALUES ($1, $2, $3, $4, $5) RETURNING to_jsonb(app_user) - 'password_hash'";

const SET_ROLE: &str = "\
UPDATE app_user SET role = $2, session_epoch = session_epoch + 1 WHERE id = $1 \
RETURNING to_jsonb(app_user) - 'password_hash'";

const DEACTIVATE: &str = "\
UPDATE app_user SET is_active = false, deactivated_at = now(), session_epoch = session_epoch + 1 \
WHERE id = $1 RETURNING to_jsonb(app_user) - 'password_hash'";

const ACTIVATE: &str = "\
UPDATE app_user SET is_active = true, deactivated_at = NULL WHERE id = $1 \
RETURNING to_jsonb(app_user) - 'password_hash'";

const RESET_PASSWORD: &str = "\
UPDATE app_user SET password_hash = $2, must_change_password = true, \
    session_epoch = session_epoch + 1 \
WHERE id = $1 RETURNING to_jsonb(app_user) - 'password_hash'";

const CHANGE_OWN_PASSWORD: &str = "\
UPDATE app_user SET password_hash = $2, must_change_password = false, \
    session_epoch = session_epoch + 1 \
WHERE id = $1 RETURNING to_jsonb(app_user) - 'password_hash'";

/// The order and the filter of `ListUsers` (design D9). `strpos` treats `_` as a plain character.
const LIST: &str = "\
SELECT id, login, role, is_active, must_change_password, deactivated_at, created_at \
FROM app_user \
WHERE ($1::text IS NULL OR strpos(login, $1) > 0) \
ORDER BY login COLLATE \"C\" LIMIT $2 OFFSET $3";

const SUMMARY_BY_LOGIN: &str = "\
SELECT id, login, role, is_active, must_change_password, deactivated_at, created_at \
FROM app_user WHERE login = $1";

#[derive(sqlx::FromRow)]
struct UserRecord {
    id: Uuid,
    login: String,
    password_hash: String,
    role: String,
    is_active: bool,
    session_epoch: i64,
    must_change_password: bool,
    deactivated_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LockedRecord {
    #[sqlx(flatten)]
    user: UserRecord,
    image: Value,
}

#[derive(sqlx::FromRow)]
struct SummaryRecord {
    id: Uuid,
    login: String,
    role: String,
    is_active: bool,
    must_change_password: bool,
    deactivated_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl TryFrom<UserRecord> for UserRow {
    type Error = StorageError;

    fn try_from(record: UserRecord) -> Result<Self, StorageError> {
        Ok(Self {
            id: UserId::from(record.id),
            login: Login::parse(&record.login).map_err(|error| corrupt(TABLE, error))?,
            password_hash: PasswordHash::parse(&record.password_hash)
                .map_err(|error| corrupt(TABLE, error))?,
            role: Role::parse(&record.role).map_err(|error| corrupt(TABLE, error))?,
            is_active: record.is_active,
            session_epoch: record.session_epoch,
            must_change_password: record.must_change_password,
            deactivated_at: record.deactivated_at.map(Timestamp::from),
            created_at: Timestamp::from(record.created_at),
            updated_at: Timestamp::from(record.updated_at),
        })
    }
}

impl TryFrom<SummaryRecord> for UserSummary {
    type Error = StorageError;

    fn try_from(record: SummaryRecord) -> Result<Self, StorageError> {
        Ok(Self {
            id: UserId::from(record.id),
            login: Login::parse(&record.login).map_err(|error| corrupt(TABLE, error))?,
            role: Role::parse(&record.role).map_err(|error| corrupt(TABLE, error))?,
            is_active: record.is_active,
            must_change_password: record.must_change_password,
            deactivated_at: record.deactivated_at.map(Timestamp::from),
            created_at: Timestamp::from(record.created_at),
        })
    }
}

/// A statement that returns the image of one row.
type ImageQuery<'q> = sqlx::query::QueryScalar<'q, Postgres, Value, PgArguments>;

fn image_query<'q>(statement: &'q str) -> ImageQuery<'q> {
    sqlx::query_scalar(statement)
}

/// Run an `INSERT` or `UPDATE ... RETURNING` image statement. A missing row is a defect.
async fn change(
    connection: &mut PgConnection,
    query: ImageQuery<'_>,
) -> Result<RowImage, StorageError> {
    let image = query.fetch_optional(connection).await.map_err(map_error)?;
    image.map(RowImage::new).ok_or_else(|| missing_row(TABLE))
}

/// Return the `LIMIT` and `OFFSET` of a page, or an error when the numbers do not fit.
fn page_bounds(page: u32, page_size: u32) -> Result<(i64, i64), StorageError> {
    let offset = u64::from(page)
        .checked_mul(u64::from(page_size))
        .and_then(|rows| i64::try_from(rows).ok())
        .ok_or_else(|| corrupt(TABLE, "the page lies outside the range of a row count"))?;
    Ok((i64::from(page_size), offset))
}

#[async_trait]
impl UserRepository for PgTransaction {
    async fn find_by_login(&mut self, login: &Login) -> Result<Option<UserRow>, StorageError> {
        let record = sqlx::query_as::<_, UserRecord>(SELECT_BY_LOGIN)
            .bind(login.as_str())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(map_error)?;
        record.map(UserRow::try_from).transpose()
    }

    async fn lock_by_login(
        &mut self,
        login: &Login,
    ) -> Result<Option<Locked<UserRow>>, StorageError> {
        let record = sqlx::query_as::<_, LockedRecord>(LOCK_BY_LOGIN)
            .bind(login.as_str())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(map_error)?;
        record
            .map(|locked| {
                Ok(Locked {
                    state: UserRow::try_from(locked.user)?,
                    before: RowImage::new(locked.image),
                })
            })
            .transpose()
    }

    async fn count_active_administrators(&mut self) -> Result<u32, StorageError> {
        let count: i64 = sqlx::query_scalar(COUNT_ACTIVE_ADMINISTRATORS)
            .fetch_one(&mut *self.tx)
            .await
            .map_err(map_error)?;
        u32::try_from(count).map_err(|error| corrupt(TABLE, error))
    }

    async fn insert(&mut self, user: &NewUser) -> Result<RowImage, StorageError> {
        let query = image_query(INSERT)
            .bind(*user.id.as_uuid())
            .bind(user.login.as_str())
            .bind(user.password_hash.as_str())
            .bind(user.role.as_sql())
            .bind(user.must_change_password);
        change(&mut self.tx, query).await
    }

    async fn set_role(&mut self, id: UserId, role: Role) -> Result<RowImage, StorageError> {
        let query = image_query(SET_ROLE)
            .bind(*id.as_uuid())
            .bind(role.as_sql());
        change(&mut self.tx, query).await
    }

    async fn deactivate(&mut self, id: UserId) -> Result<RowImage, StorageError> {
        change(&mut self.tx, image_query(DEACTIVATE).bind(*id.as_uuid())).await
    }

    async fn activate(&mut self, id: UserId) -> Result<RowImage, StorageError> {
        change(&mut self.tx, image_query(ACTIVATE).bind(*id.as_uuid())).await
    }

    async fn reset_password(
        &mut self,
        id: UserId,
        hash: &PasswordHash,
    ) -> Result<RowImage, StorageError> {
        let query = image_query(RESET_PASSWORD)
            .bind(*id.as_uuid())
            .bind(hash.as_str());
        change(&mut self.tx, query).await
    }

    async fn change_own_password(
        &mut self,
        id: UserId,
        hash: &PasswordHash,
    ) -> Result<RowImage, StorageError> {
        let query = image_query(CHANGE_OWN_PASSWORD)
            .bind(*id.as_uuid())
            .bind(hash.as_str());
        change(&mut self.tx, query).await
    }

    async fn list(
        &mut self,
        filter: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<UserSummary>, StorageError> {
        let (limit, offset) = page_bounds(page, page_size)?;
        let records = sqlx::query_as::<_, SummaryRecord>(LIST)
            .bind(filter)
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *self.tx)
            .await
            .map_err(map_error)?;
        records.into_iter().map(UserSummary::try_from).collect()
    }

    async fn summary_by_login(
        &mut self,
        login: &Login,
    ) -> Result<Option<UserSummary>, StorageError> {
        let record = sqlx::query_as::<_, SummaryRecord>(SUMMARY_BY_LOGIN)
            .bind(login.as_str())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(map_error)?;
        record.map(UserSummary::try_from).transpose()
    }
}
