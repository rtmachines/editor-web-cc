//! SQL of the recipe writes of `RecipeRepository` (task 9.6, `data-model.md` sections 3.9, 6).
//!
//! A write of one row returns the image after the change with `RETURNING` (design D5).

use recipe_app::error::StorageError;
use recipe_app::ports::{InsertedRecipe, NewVersion, RowImage};
use recipe_domain::recipe::{Recipe, RecipeId};
use recipe_domain::text::Text;
use serde_json::Value;
use sqlx::PgConnection;

use super::version_writes;
use crate::error::map_error;

/// The foreign key from `recipe` to `machine`. A missing machine row breaks it.
const RECIPE_MACHINE_PLATFORM: &str = "recipe_machine_platform";

/// One `INSERT` of the recipe row. The `SELECT` reads the platform of the machine, so a missing
/// machine inserts no row.
const INSERT_RECIPE: &str = "\
INSERT INTO recipe (id, machine_id, plc_platform, reference_number, name, deleted_at, created_by) \
SELECT $1, m.id, m.plc_platform, $3, $4, $5, $6 FROM machine m WHERE m.id = $2 \
RETURNING to_jsonb(recipe)";

const RENAME_RECIPE: &str = "UPDATE recipe SET name = $2 WHERE id = $1 RETURNING to_jsonb(recipe)";

const DELETE_RECIPE: &str =
    "UPDATE recipe SET deleted_at = now() WHERE id = $1 RETURNING to_jsonb(recipe)";

const RESTORE_RECIPE: &str =
    "UPDATE recipe SET deleted_at = NULL WHERE id = $1 RETURNING to_jsonb(recipe)";

/// Insert the recipe row and the row of version 1.
///
/// A known reference number gives `Duplicate` with `recipe_reference_unique` from the error map.
pub(super) async fn insert(
    connection: &mut PgConnection,
    recipe: &Recipe,
    first: &NewVersion,
) -> Result<InsertedRecipe, StorageError> {
    let image: Option<Value> = sqlx::query_scalar(INSERT_RECIPE)
        .bind(*recipe.id().as_uuid())
        .bind(*recipe.machine().as_uuid())
        .bind(recipe.reference().as_str())
        .bind(recipe.name().as_str())
        .bind(recipe.deleted_at().map(|at| *at.as_offset_date_time()))
        .bind(*first.creator.as_uuid())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    let Some(image) = image else {
        return Err(StorageError::MissingReference {
            constraint: RECIPE_MACHINE_PLATFORM.to_owned(),
        });
    };
    let version = version_writes::insert(connection, first).await?;
    Ok(InsertedRecipe {
        recipe: RowImage::new(image),
        version,
    })
}

async fn update_one(
    connection: &mut PgConnection,
    statement: &'static str,
    id: RecipeId,
    name: Option<&Text>,
) -> Result<RowImage, StorageError> {
    let mut query = sqlx::query_scalar::<_, Value>(statement).bind(*id.as_uuid());
    if let Some(name) = name {
        query = query.bind(name.as_str());
    }
    let image = query.fetch_one(connection).await.map_err(map_error)?;
    Ok(RowImage::new(image))
}

pub(super) async fn rename(
    connection: &mut PgConnection,
    id: RecipeId,
    name: &Text,
) -> Result<RowImage, StorageError> {
    update_one(connection, RENAME_RECIPE, id, Some(name)).await
}

pub(super) async fn mark_deleted(
    connection: &mut PgConnection,
    id: RecipeId,
) -> Result<RowImage, StorageError> {
    update_one(connection, DELETE_RECIPE, id, None).await
}

pub(super) async fn restore(
    connection: &mut PgConnection,
    id: RecipeId,
) -> Result<RowImage, StorageError> {
    update_one(connection, RESTORE_RECIPE, id, None).await
}
