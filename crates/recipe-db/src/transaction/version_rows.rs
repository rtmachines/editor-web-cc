//! SQL of the version locks of `RecipeRepository` (task 9.5, `data-model.md` sections 3.10, 6).
//!
//! A lock statement returns the image of the row before any change and the `row_version` that
//! it read. The transaction keeps that value for the guard of a status write (ADR-0030).

use recipe_app::error::StorageError;
use recipe_app::ports::{RowImage, SharedVersion};
use recipe_domain::export::Sha256;
use recipe_domain::recipe::{RecipeId, RecipeVersionId, VersionState, VersionStatus};
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{corrupt, map_error};

/// Every version of a recipe `FOR UPDATE`, ordered by `id` (`DeleteRecipe`).
const LOCK_VERSIONS: &str = "\
SELECT id, row_version, to_jsonb(v) AS image \
FROM recipe_version v WHERE recipe_id = $1 ORDER BY id FOR UPDATE";

const LOCK_VERSION: &str = "\
SELECT id, row_version, to_jsonb(v) AS image \
FROM recipe_version v WHERE id = $1 FOR UPDATE";

/// `recipe_version FOR SHARE` (`ExportVersion`).
const LOCK_VERSION_SHARED: &str = "\
SELECT recipe_id, number, status, deleted_at, content_sha256, approved_by, approved_at \
FROM recipe_version WHERE id = $1 FOR SHARE";

/// A version row that a statement locked.
#[derive(sqlx::FromRow)]
pub(super) struct LockedVersion {
    id: Uuid,
    /// The `row_version` that the lock statement read.
    pub(super) row_version: i64,
    /// The image of the row before any change.
    image: Value,
}

impl LockedVersion {
    /// Return the image of the row before any change.
    pub(super) fn into_image(self) -> RowImage {
        RowImage::new(self.image)
    }

    pub(super) fn version(&self) -> RecipeVersionId {
        RecipeVersionId::from(self.id)
    }
}

#[derive(sqlx::FromRow)]
struct SharedRecord {
    recipe_id: Uuid,
    number: i32,
    status: String,
    deleted_at: Option<OffsetDateTime>,
    content_sha256: Option<String>,
    approved_by: Option<Uuid>,
    approved_at: Option<OffsetDateTime>,
}

/// Take `FOR UPDATE` on every version of a recipe, ordered by `id`.
pub(super) async fn lock_all(
    connection: &mut PgConnection,
    recipe: RecipeId,
) -> Result<Vec<LockedVersion>, StorageError> {
    sqlx::query_as::<_, LockedVersion>(LOCK_VERSIONS)
        .bind(*recipe.as_uuid())
        .fetch_all(connection)
        .await
        .map_err(map_error)
}

/// Take `FOR UPDATE` on one version. A missing row gives `None`, and PostgreSQL takes no lock.
pub(super) async fn lock_one(
    connection: &mut PgConnection,
    version: RecipeVersionId,
) -> Result<Option<LockedVersion>, StorageError> {
    sqlx::query_as::<_, LockedVersion>(LOCK_VERSION)
        .bind(*version.as_uuid())
        .fetch_optional(connection)
        .await
        .map_err(map_error)
}

/// Take `FOR SHARE` on one version and read what the export needs.
pub(super) async fn lock_shared(
    connection: &mut PgConnection,
    version: RecipeVersionId,
) -> Result<Option<SharedVersion>, StorageError> {
    let record = sqlx::query_as::<_, SharedRecord>(LOCK_VERSION_SHARED)
        .bind(*version.as_uuid())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    record.map(shared_version).transpose()
}

fn shared_version(record: SharedRecord) -> Result<SharedVersion, StorageError> {
    let status =
        VersionStatus::parse(&record.status).map_err(|error| corrupt("recipe_version", error))?;
    Ok(SharedVersion {
        recipe: RecipeId::from(record.recipe_id),
        number: u32::try_from(record.number).map_err(|error| corrupt("recipe_version", error))?,
        state: VersionState::from_stored(status, record.deleted_at.is_some())
            .map_err(|error| corrupt("recipe_version", error))?,
        content_sha256: record
            .content_sha256
            .as_deref()
            .map(Sha256::parse)
            .transpose()
            .map_err(|error| corrupt("recipe_version", error))?,
        approved_by: record.approved_by.map(Into::into),
        approved_at: record.approved_at.map(Timestamp::from),
    })
}
