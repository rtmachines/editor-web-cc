//! SQL of the recipe reads and recipe locks of `RecipeRepository` (task 9.5, `data-model.md`
//! sections 3.9, 3.10, 6).
//!
//! Each function runs on the connection of the open transaction. A lock statement returns the
//! image of the row before any change. A recipe is built from its row and the headers of its
//! versions with the constructors of the domain.

use std::collections::BTreeMap;

use recipe_app::error::StorageError;
use recipe_app::ports::{Locked, RowImage};
use recipe_domain::plant::MachineId;
use recipe_domain::recipe::{
    Recipe, RecipeId, ReferenceNumber, StoredRecipe, StoredVersionHeader, VersionState,
    VersionStatus,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{corrupt, map_error};

const SELECT_BY_REFERENCE: &str = "\
SELECT id, machine_id, reference_number, name, deleted_at \
FROM recipe WHERE reference_number = $1";

const SELECT_BY_ID: &str = "\
SELECT id, machine_id, reference_number, name, deleted_at FROM recipe WHERE id = $1";

/// `recipe FOR UPDATE`. The image comes from the locked row.
const LOCK_RECIPE: &str = "\
SELECT id, machine_id, reference_number, name, deleted_at, to_jsonb(r) AS image \
FROM recipe r WHERE id = $1 FOR UPDATE";

/// Every recipe of a machine `FOR UPDATE`, ordered by `id` (`ArchiveMachine`).
const LOCK_RECIPES_OF_MACHINE: &str = "\
SELECT id, machine_id, reference_number, name, deleted_at \
FROM recipe WHERE machine_id = $1 ORDER BY id FOR UPDATE";

const SELECT_HEADERS: &str = "\
SELECT id, recipe_id, number, status, deleted_at, row_version \
FROM recipe_version WHERE recipe_id = $1";

const SELECT_HEADERS_OF_RECIPES: &str = "\
SELECT id, recipe_id, number, status, deleted_at, row_version \
FROM recipe_version WHERE recipe_id = ANY($1)";

#[derive(sqlx::FromRow)]
struct RecipeRecord {
    id: Uuid,
    machine_id: Uuid,
    reference_number: String,
    name: String,
    deleted_at: Option<OffsetDateTime>,
}

#[derive(sqlx::FromRow)]
struct LockedRecipeRecord {
    #[sqlx(flatten)]
    recipe: RecipeRecord,
    image: Value,
}

#[derive(sqlx::FromRow)]
struct HeaderRecord {
    id: Uuid,
    recipe_id: Uuid,
    number: i32,
    status: String,
    deleted_at: Option<OffsetDateTime>,
    row_version: i64,
}

/// Build the stored header of a version row. A corrupt row gives a storage error.
fn stored_header(record: HeaderRecord) -> Result<StoredVersionHeader, StorageError> {
    let status =
        VersionStatus::parse(&record.status).map_err(|error| corrupt("recipe_version", error))?;
    let state = VersionState::from_stored(status, record.deleted_at.is_some())
        .map_err(|error| corrupt("recipe_version", error))?;
    Ok(StoredVersionHeader {
        id: record.id.into(),
        number: u32::try_from(record.number).map_err(|error| corrupt("recipe_version", error))?,
        state,
        row_version: record.row_version,
    })
}

/// Build the recipe of a recipe row and the version rows of the same recipe.
fn build_recipe(record: RecipeRecord, headers: Vec<HeaderRecord>) -> Result<Recipe, StorageError> {
    let versions = headers
        .into_iter()
        .map(stored_header)
        .collect::<Result<Vec<_>, StorageError>>()?;
    Recipe::try_from(StoredRecipe {
        id: RecipeId::from(record.id),
        machine: MachineId::from(record.machine_id),
        reference: ReferenceNumber::parse(&record.reference_number)
            .map_err(|error| corrupt("recipe", error))?,
        name: Text::parse(&record.name).map_err(|error| corrupt("recipe", error))?,
        deleted_at: record.deleted_at.map(Timestamp::from),
        versions,
    })
    .map_err(|error| corrupt("recipe", error))
}

/// Read the version rows of one recipe.
async fn headers_of(
    connection: &mut PgConnection,
    recipe: Uuid,
) -> Result<Vec<HeaderRecord>, StorageError> {
    sqlx::query_as::<_, HeaderRecord>(SELECT_HEADERS)
        .bind(recipe)
        .fetch_all(connection)
        .await
        .map_err(map_error)
}

/// Read a recipe row with the headers of its versions.
async fn with_headers(
    connection: &mut PgConnection,
    record: RecipeRecord,
) -> Result<Recipe, StorageError> {
    let headers = headers_of(connection, record.id).await?;
    build_recipe(record, headers)
}

pub(super) async fn by_reference(
    connection: &mut PgConnection,
    reference: &ReferenceNumber,
) -> Result<Option<Recipe>, StorageError> {
    let record = sqlx::query_as::<_, RecipeRecord>(SELECT_BY_REFERENCE)
        .bind(reference.as_str())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    match record {
        Some(record) => with_headers(connection, record).await.map(Some),
        None => Ok(None),
    }
}

pub(super) async fn by_id(
    connection: &mut PgConnection,
    id: RecipeId,
) -> Result<Option<Recipe>, StorageError> {
    let record = sqlx::query_as::<_, RecipeRecord>(SELECT_BY_ID)
        .bind(*id.as_uuid())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    match record {
        Some(record) => with_headers(connection, record).await.map(Some),
        None => Ok(None),
    }
}

/// Take `recipe FOR UPDATE`. A missing row gives `None`, and PostgreSQL takes no lock.
pub(super) async fn lock(
    connection: &mut PgConnection,
    id: RecipeId,
) -> Result<Option<Locked<Recipe>>, StorageError> {
    let record = sqlx::query_as::<_, LockedRecipeRecord>(LOCK_RECIPE)
        .bind(*id.as_uuid())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    let Some(record) = record else {
        return Ok(None);
    };
    let state = with_headers(connection, record.recipe).await?;
    Ok(Some(Locked {
        state,
        before: RowImage::new(record.image),
    }))
}

/// Take `FOR UPDATE` on every recipe of a machine, ordered by `id`.
///
/// One further statement reads the version rows of all locked recipes, so the number of
/// statements does not depend on the number of recipes.
pub(super) async fn lock_of_machine(
    connection: &mut PgConnection,
    machine: MachineId,
) -> Result<Vec<Recipe>, StorageError> {
    let records = sqlx::query_as::<_, RecipeRecord>(LOCK_RECIPES_OF_MACHINE)
        .bind(*machine.as_uuid())
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let ids: Vec<Uuid> = records.iter().map(|record| record.id).collect();
    let headers = sqlx::query_as::<_, HeaderRecord>(SELECT_HEADERS_OF_RECIPES)
        .bind(ids)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    let mut by_recipe: BTreeMap<Uuid, Vec<HeaderRecord>> = BTreeMap::new();
    for header in headers {
        by_recipe.entry(header.recipe_id).or_default().push(header);
    }
    records
        .into_iter()
        .map(|record| {
            let headers = by_recipe.remove(&record.id).unwrap_or_default();
            build_recipe(record, headers)
        })
        .collect()
}
