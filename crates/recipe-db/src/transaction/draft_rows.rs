//! SQL of the draft reads and the touch of `DraftRepository` (task 9.7, `data-model.md`
//! sections 3.10, 3.11, 6, design D6, ADR-0030).
//!
//! The read runs one statement for the version header, one for the editors, and three set
//! queries for the parameters (`parameter_rows`). The lock variant adds `FOR UPDATE OF v`, so
//! the statement locks the version row and no row of `recipe`.

use recipe_app::error::StorageError;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{MachineId, PlcPlatform};
use recipe_domain::recipe::{
    RecipeVersionId, ReferenceNumber, StoredDraft, VersionState, VersionStatus,
};
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::parameter_rows;
use crate::error::{corrupt, map_error, missing_row};

/// The version row with the recipe columns that the draft carries.
const SELECT_HEADER: &str = "\
SELECT v.id, v.number, v.status, v.deleted_at, v.row_version, v.created_by, v.plc_platform, \
r.machine_id, r.reference_number \
FROM recipe_version v JOIN recipe r ON r.id = v.recipe_id WHERE v.id = $1";

/// The same statement with the lock. `OF v` leaves the recipe row unlocked.
const LOCK_HEADER: &str = "\
SELECT v.id, v.number, v.status, v.deleted_at, v.row_version, v.created_by, v.plc_platform, \
r.machine_id, r.reference_number \
FROM recipe_version v JOIN recipe r ON r.id = v.recipe_id WHERE v.id = $1 FOR UPDATE OF v";

const SELECT_EDITORS: &str = "\
SELECT user_id FROM draft_editor WHERE recipe_version_id = $1 ORDER BY user_id";

/// The touch statement. Trigger 4.4 adds 1 to `row_version`.
const TOUCH: &str =
    "UPDATE recipe_version SET updated_at = now() WHERE id = $1 RETURNING row_version";

const INSERT_EDITOR: &str = "\
INSERT INTO draft_editor (recipe_version_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING";

/// The table name in the error of a write that finds no row.
const VERSION_TABLE: &str = "recipe_version";

#[derive(sqlx::FromRow)]
struct HeaderRecord {
    id: Uuid,
    number: i32,
    status: String,
    deleted_at: Option<OffsetDateTime>,
    row_version: i64,
    created_by: Uuid,
    plc_platform: String,
    machine_id: Uuid,
    reference_number: String,
}

/// Build the stored draft of a header row, its editors, and its parameters.
async fn build(
    connection: &mut PgConnection,
    record: HeaderRecord,
) -> Result<StoredDraft, StorageError> {
    let version = RecipeVersionId::from(record.id);
    let editors = sqlx::query_scalar::<_, Uuid>(SELECT_EDITORS)
        .bind(record.id)
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let parameters = parameter_rows::of_version(connection, version).await?;
    let status =
        VersionStatus::parse(&record.status).map_err(|error| corrupt("recipe_version", error))?;
    Ok(StoredDraft {
        reference: ReferenceNumber::parse(&record.reference_number)
            .map_err(|error| corrupt("recipe", error))?,
        machine: MachineId::from(record.machine_id),
        platform: PlcPlatform::parse(&record.plc_platform)
            .map_err(|error| corrupt("recipe_version", error))?,
        version,
        number: u32::try_from(record.number).map_err(|error| corrupt("recipe_version", error))?,
        state: VersionState::from_stored(status, record.deleted_at.is_some())
            .map_err(|error| corrupt("recipe_version", error))?,
        row_version: record.row_version,
        created_by: UserId::from(record.created_by),
        editors: editors.into_iter().map(UserId::from).collect(),
        parameters,
    })
}

async fn load(
    connection: &mut PgConnection,
    statement: &'static str,
    version: RecipeVersionId,
) -> Result<Option<StoredDraft>, StorageError> {
    let record = sqlx::query_as::<_, HeaderRecord>(statement)
        .bind(*version.as_uuid())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    match record {
        Some(record) => build(connection, record).await.map(Some),
        None => Ok(None),
    }
}

/// Take `recipe_version FOR UPDATE` and read the draft. A missing row gives `None` and no lock.
pub(super) async fn lock(
    connection: &mut PgConnection,
    version: RecipeVersionId,
) -> Result<Option<StoredDraft>, StorageError> {
    load(connection, LOCK_HEADER, version).await
}

/// Read the draft without a lock.
pub(super) async fn read(
    connection: &mut PgConnection,
    version: RecipeVersionId,
) -> Result<Option<StoredDraft>, StorageError> {
    load(connection, SELECT_HEADER, version).await
}

/// Run the touch statement, then insert the editor. Return the new `row_version`.
pub(super) async fn touch(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    editor: UserId,
) -> Result<i64, StorageError> {
    let row_version = sqlx::query_scalar::<_, i64>(TOUCH)
        .bind(*version.as_uuid())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?
        .ok_or_else(|| missing_row(VERSION_TABLE))?;
    sqlx::query(INSERT_EDITOR)
        .bind(*version.as_uuid())
        .bind(*editor.as_uuid())
        .execute(connection)
        .await
        .map_err(map_error)?;
    Ok(row_version)
}
