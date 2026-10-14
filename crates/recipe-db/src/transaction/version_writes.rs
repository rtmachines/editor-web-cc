//! SQL of the version writes of `RecipeRepository` (task 9.6, `data-model.md` sections 3.10, 6,
//! design D5 and D7, ADR-0030).
//!
//! A status write runs `UPDATE ... WHERE id = $v AND row_version = $locked`. `$locked` is the
//! `row_version` that the lock statement of the same transaction read. The status condition stays
//! out of the `WHERE`, so trigger 4.7 rejects an illegal transition with `E-RCP-033`.

use recipe_app::error::StorageError;
use recipe_app::ports::{NewVersion, RowImage};
use recipe_domain::export::Sha256;
use recipe_domain::identity::UserId;
use recipe_domain::recipe::RecipeVersionId;
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{check_violation, map_error, without_lock};

/// The foreign key from `recipe_version` to `recipe`. A missing recipe row breaks it.
const VERSION_RECIPE_PLATFORM: &str = "recipe_version_recipe_platform";

/// One `INSERT` of the version row. The `SELECT` reads the platform of the recipe, so a missing
/// recipe inserts no row.
const INSERT_VERSION: &str = "\
INSERT INTO recipe_version (id, recipe_id, plc_platform, number, based_on_version_id, \
change_note, created_by) \
SELECT $1, r.id, r.plc_platform, $3, $4, $5, $6 FROM recipe r WHERE r.id = $2";

/// Copy the parameters of the base version. Each copy has a new `id` and the same key.
const COPY_PARAMETERS: &str = "\
INSERT INTO parameter (id, recipe_version_id, plc_platform, parameter_key, unit_id, tag, \
name_en, description_en, plc_type_id, s7_address, value, ordinal) \
SELECT gen_random_uuid(), $1, p.plc_platform, p.parameter_key, p.unit_id, p.tag, p.name_en, \
p.description_en, p.plc_type_id, p.s7_address, p.value, p.ordinal \
FROM parameter p WHERE p.recipe_version_id = $2";

/// Copy the limit rows. The join on `parameter_key` finds the new parameter of each old one.
const COPY_LIMITS: &str = "\
INSERT INTO parameter_limit (parameter_id, recipe_version_id, leaf_path, min_value, max_value, \
max_length) \
SELECT np.id, np.recipe_version_id, l.leaf_path, l.min_value, l.max_value, l.max_length \
FROM parameter_limit l \
JOIN parameter op ON op.id = l.parameter_id AND op.recipe_version_id = l.recipe_version_id \
JOIN parameter np ON np.recipe_version_id = $1 AND np.parameter_key = op.parameter_key \
WHERE l.recipe_version_id = $2";

/// Copy the translation rows. The join works as in `COPY_LIMITS`.
const COPY_TRANSLATIONS: &str = "\
INSERT INTO parameter_translation (parameter_id, recipe_version_id, lang, name, description) \
SELECT np.id, np.recipe_version_id, t.lang, t.name, t.description \
FROM parameter_translation t \
JOIN parameter op ON op.id = t.parameter_id AND op.recipe_version_id = t.recipe_version_id \
JOIN parameter np ON np.recipe_version_id = $1 AND np.parameter_key = op.parameter_key \
WHERE t.recipe_version_id = $2";

/// The image of the version row. A separate statement reads it after the writes (design D5).
const SELECT_IMAGE: &str = "SELECT to_jsonb(v) FROM recipe_version v WHERE id = $1";

const DISCARD: &str = "\
UPDATE recipe_version SET deleted_at = now() WHERE id = $1 AND row_version = $2 \
RETURNING to_jsonb(recipe_version.*) AS image, row_version";

const RETIRE: &str = "\
UPDATE recipe_version SET status = 'retired', retired_at = now() \
WHERE id = $1 AND row_version = $2 \
RETURNING to_jsonb(recipe_version.*) AS image, row_version";

const APPROVE: &str = "\
UPDATE recipe_version SET status = 'approved', approved_by = $3, approved_at = now(), \
content_sha256 = $4 WHERE id = $1 AND row_version = $2 \
RETURNING to_jsonb(recipe_version.*) AS image, row_version";

/// The row after a status write and its new `row_version`.
#[derive(sqlx::FromRow)]
pub(super) struct Written {
    image: Value,
    /// The `row_version` after the write. The next status write of the transaction needs it.
    pub(super) row_version: i64,
}

impl Written {
    /// Return the image of the row after the write.
    pub(super) fn into_image(self) -> RowImage {
        RowImage::new(self.image)
    }
}

/// Run one guarded status statement. No row means that the guard stopped the write.
async fn guarded(
    connection: &mut PgConnection,
    statement: &'static str,
    version: RecipeVersionId,
    locked: i64,
    extra: Option<(Uuid, String)>,
) -> Result<Written, StorageError> {
    let mut query = sqlx::query_as::<_, Written>(statement)
        .bind(*version.as_uuid())
        .bind(locked);
    if let Some((approver, hash)) = extra {
        query = query.bind(approver).bind(hash);
    }
    query
        .fetch_optional(connection)
        .await
        .map_err(map_error)?
        .ok_or_else(|| without_lock(version))
}

pub(super) async fn discard(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    locked: i64,
) -> Result<Written, StorageError> {
    guarded(connection, DISCARD, version, locked, None).await
}

pub(super) async fn retire(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    locked: i64,
) -> Result<Written, StorageError> {
    guarded(connection, RETIRE, version, locked, None).await
}

pub(super) async fn approve(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    locked: i64,
    (approver, hash): (UserId, Sha256),
) -> Result<Written, StorageError> {
    let extra = Some((*approver.as_uuid(), hash.to_string()));
    guarded(connection, APPROVE, version, locked, extra).await
}

/// Insert the version row, copy the parameters of the base version, and read the row image.
///
/// A missing recipe gives `MissingReference`. A second live draft gives `Duplicate` from the
/// error map.
pub(super) async fn insert(
    connection: &mut PgConnection,
    new: &NewVersion,
) -> Result<RowImage, StorageError> {
    let number = i32::try_from(new.number).map_err(|_| check_violation())?;
    let inserted = sqlx::query(INSERT_VERSION)
        .bind(*new.version.as_uuid())
        .bind(*new.recipe.as_uuid())
        .bind(number)
        .bind(new.based_on.map(|base| *base.as_uuid()))
        .bind(new.change_note.as_str())
        .bind(*new.creator.as_uuid())
        .execute(&mut *connection)
        .await
        .map_err(map_error)?;
    if inserted.rows_affected() == 0 {
        return Err(StorageError::MissingReference {
            constraint: VERSION_RECIPE_PLATFORM.to_owned(),
        });
    }
    if let Some(base) = new.based_on {
        for statement in [COPY_PARAMETERS, COPY_LIMITS, COPY_TRANSLATIONS] {
            sqlx::query(statement)
                .bind(*new.version.as_uuid())
                .bind(*base.as_uuid())
                .execute(&mut *connection)
                .await
                .map_err(map_error)?;
        }
    }
    let image: Value = sqlx::query_scalar(SELECT_IMAGE)
        .bind(*new.version.as_uuid())
        .fetch_one(connection)
        .await
        .map_err(map_error)?;
    Ok(RowImage::new(image))
}
