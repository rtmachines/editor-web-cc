//! SQL of `version_page`, `version_detail`, and `last_change` of `RecipeReads` (design D5,
//! `data-model.md` sections 3.5, 3.10, 3.15).
//!
//! Each function runs one statement on the connection of the open transaction and takes no lock.

use recipe_app::error::StorageError;
use recipe_app::ports::{VersionDetail, VersionSummary};
use recipe_domain::identity::Login;
use recipe_domain::recipe::{RecipeId, RecipeVersionId, VersionState, VersionStatus};
use recipe_domain::text::LongText;
use recipe_domain::timestamp::Timestamp;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::window::bind_window;
use crate::error::{corrupt, map_error};

/// One page of the versions of a recipe, the newest number first.
const SELECT_PAGE: &str = "\
SELECT number, status, deleted_at, change_note, created_at, approved_at \
FROM recipe_version WHERE recipe_id = $1 ORDER BY number DESC OFFSET $2 LIMIT $3";

/// The header of one version with the logins of its actors. A version without an approver has
/// no row of the second join.
const SELECT_DETAIL: &str = "\
SELECT v.id, v.recipe_id, v.number, v.status, v.deleted_at, v.change_note, \
    c.login AS created_by, a.login AS approved_by, \
    v.created_at, v.approved_at, v.updated_at, v.row_version \
FROM recipe_version v \
JOIN app_user c ON c.id = v.created_by \
LEFT JOIN app_user a ON a.id = v.approved_by \
WHERE v.id = $1";

/// The login of the actor of the newest audit entry of the recipe `$1` at the instant `$2`.
/// The index `audit_log_aggregate_idx` serves the filter and the order. An entry without an
/// actor gives a row with a null login.
const SELECT_LAST_ACTOR: &str = "\
SELECT u.login FROM audit_log l \
LEFT JOIN app_user u ON u.id = l.actor_user_id \
WHERE l.aggregate_id = $1 AND l.occurred_at = $2 \
ORDER BY l.id DESC LIMIT 1";

#[derive(sqlx::FromRow)]
struct SummaryRecord {
    number: i32,
    status: String,
    deleted_at: Option<OffsetDateTime>,
    change_note: String,
    created_at: OffsetDateTime,
    approved_at: Option<OffsetDateTime>,
}

#[derive(sqlx::FromRow)]
struct DetailRecord {
    id: Uuid,
    recipe_id: Uuid,
    number: i32,
    status: String,
    deleted_at: Option<OffsetDateTime>,
    change_note: String,
    created_by: String,
    approved_by: Option<String>,
    created_at: OffsetDateTime,
    approved_at: Option<OffsetDateTime>,
    updated_at: OffsetDateTime,
    row_version: i64,
}

/// Build the state of a version row from its status and its deletion mark.
fn state(status: &str, deleted_at: Option<OffsetDateTime>) -> Result<VersionState, StorageError> {
    let status = VersionStatus::parse(status).map_err(|error| corrupt("recipe_version", error))?;
    VersionState::from_stored(status, deleted_at.is_some())
        .map_err(|error| corrupt("recipe_version", error))
}

/// Convert the number of a version row.
fn number(value: i32) -> Result<u32, StorageError> {
    u32::try_from(value).map_err(|error| corrupt("recipe_version", error))
}

/// Parse a login of `app_user`. A row that fails the grammar is corrupt.
fn login(text: &str) -> Result<Login, StorageError> {
    Login::parse(text).map_err(|error| corrupt("app_user", error))
}

/// Build the summary of a stored row.
fn summary(record: SummaryRecord) -> Result<VersionSummary, StorageError> {
    Ok(VersionSummary {
        number: number(record.number)?,
        state: state(&record.status, record.deleted_at)?,
        change_note: LongText::parse(&record.change_note)
            .map_err(|error| corrupt("recipe_version", error))?,
        created_at: Timestamp::from(record.created_at),
        approved_at: record.approved_at.map(Timestamp::from),
    })
}

/// Build the detail of a stored row.
fn detail(record: DetailRecord) -> Result<VersionDetail, StorageError> {
    Ok(VersionDetail {
        id: RecipeVersionId::from(record.id),
        recipe: RecipeId::from(record.recipe_id),
        number: number(record.number)?,
        state: state(&record.status, record.deleted_at)?,
        change_note: LongText::parse(&record.change_note)
            .map_err(|error| corrupt("recipe_version", error))?,
        created_by: login(&record.created_by)?,
        approved_by: record.approved_by.as_deref().map(login).transpose()?,
        created_at: Timestamp::from(record.created_at),
        approved_at: record.approved_at.map(Timestamp::from),
        updated_at: Timestamp::from(record.updated_at),
        row_version: record.row_version,
    })
}

/// Read one page of the versions of `recipe`, the newest number first.
pub(super) async fn page(
    connection: &mut PgConnection,
    recipe: RecipeId,
    offset: u64,
    limit: u32,
) -> Result<Vec<VersionSummary>, StorageError> {
    let (offset, limit) = bind_window(offset, limit)?;
    let records = sqlx::query_as::<_, SummaryRecord>(SELECT_PAGE)
        .bind(*recipe.as_uuid())
        .bind(offset)
        .bind(limit)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    records.into_iter().map(summary).collect()
}

/// Read the header of `version` with the logins of its actors. A missing row gives `None`.
pub(super) async fn by_id(
    connection: &mut PgConnection,
    version: RecipeVersionId,
) -> Result<Option<VersionDetail>, StorageError> {
    let record = sqlx::query_as::<_, DetailRecord>(SELECT_DETAIL)
        .bind(*version.as_uuid())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    record.map(detail).transpose()
}

/// Read the login of the actor of the newest audit entry of `recipe` at `at`.
pub(super) async fn last_actor(
    connection: &mut PgConnection,
    recipe: RecipeId,
    at: Timestamp,
) -> Result<Option<Login>, StorageError> {
    let found: Option<Option<String>> = sqlx::query_scalar(SELECT_LAST_ACTOR)
        .bind(*recipe.as_uuid())
        .bind(*at.as_offset_date_time())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    found.flatten().as_deref().map(login).transpose()
}
