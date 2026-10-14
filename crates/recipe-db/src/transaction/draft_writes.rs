//! SQL of the parameter writes of `DraftRepository` (tasks 9.7 and 9.8, `data-model.md`
//! sections 3.12 to 3.14, 6, design D5).
//!
//! Every statement names the parameter by `parameter_key` inside the version. Trigger 4.1
//! rejects a write in a version that is not a live draft with `E-RCP-031`.

use recipe_app::error::StorageError;
use recipe_app::ports::RowImage;
use recipe_domain::recipe::{
    LangCode, Parameter, ParameterId, ParameterKey, RecipeVersionId, StoredLimit, StoredValue,
    Translation,
};
use recipe_domain::text::LongText;
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{check_violation, map_error, missing_row};

/// The foreign key from `parameter` to `recipe_version`. A missing version row breaks it.
const PARAMETER_VERSION_PLATFORM: &str = "parameter_version_platform";

/// The table name in the error of a write that finds no parameter.
const PARAMETER_TABLE: &str = "parameter";

/// One `INSERT` of the parameter row. The `SELECT` reads the platform of the version, so a
/// missing version inserts no row.
const INSERT_PARAMETER: &str = "\
INSERT INTO parameter (id, recipe_version_id, plc_platform, parameter_key, unit_id, tag, \
name_en, description_en, plc_type_id, s7_address, value, ordinal) \
SELECT $2, v.id, v.plc_platform, $3, $4, $5, $6, $7, $8, $9, $10, $11 \
FROM recipe_version v WHERE v.id = $1";

/// One `INSERT` for all limit rows of a parameter.
const INSERT_LIMITS: &str = "\
INSERT INTO parameter_limit (parameter_id, recipe_version_id, leaf_path, min_value, max_value, \
max_length) \
SELECT $1, $2, l.leaf_path, l.min_value, l.max_value, l.max_length \
FROM unnest($3::text[], $4::numeric[], $5::numeric[], $6::int2[]) \
AS l(leaf_path, min_value, max_value, max_length)";

/// The SQL expression of a parameter image with the arrays `limits` and `translations`
/// (design D5). `$alias` names the parameter row in the statement.
///
/// The macro solves one problem: the insert and the delete build the same image, and the text of
/// the expression must not drift apart.
macro_rules! parameter_image {
    ($alias:literal) => {
        concat!(
            "to_jsonb(",
            $alias,
            ") || jsonb_build_object(\
             'limits', COALESCE((SELECT jsonb_agg(to_jsonb(l) ORDER BY l.leaf_path COLLATE \"C\") \
             FROM parameter_limit l WHERE l.parameter_id = ",
            $alias,
            ".id), '[]'::jsonb), \
             'translations', COALESCE((SELECT jsonb_agg(to_jsonb(t) ORDER BY t.lang COLLATE \"C\") \
             FROM parameter_translation t WHERE t.parameter_id = ",
            $alias,
            ".id), '[]'::jsonb))"
        )
    };
}

/// A separate statement reads the image of an inserted parameter after the writes.
const SELECT_IMAGE: &str = concat!(
    "SELECT ",
    parameter_image!("p"),
    " FROM parameter p WHERE p.id = $1"
);

/// `DELETE parameter` with the image of the row and its child rows from the same statement.
/// The sub-selects read the snapshot before the delete, also the rows of the cascade.
const DELETE_PARAMETER: &str = concat!(
    "WITH gone AS (DELETE FROM parameter WHERE recipe_version_id = $1 AND parameter_key = $2 \
     RETURNING *) SELECT ",
    parameter_image!("g"),
    " FROM gone g"
);

const SET_VALUE: &str =
    "UPDATE parameter SET value = $3 WHERE recipe_version_id = $1 AND parameter_key = $2";

const SELECT_PARAMETER: &str =
    "SELECT id FROM parameter WHERE recipe_version_id = $1 AND parameter_key = $2";

const DELETE_LIMITS: &str = "DELETE FROM parameter_limit WHERE parameter_id = $1";

const UPSERT_TRANSLATION: &str = "\
INSERT INTO parameter_translation (parameter_id, recipe_version_id, lang, name, description) \
SELECT p.id, p.recipe_version_id, $3, $4, $5 FROM parameter p \
WHERE p.recipe_version_id = $1 AND p.parameter_key = $2 \
ON CONFLICT (parameter_id, lang) DO UPDATE SET name = EXCLUDED.name, \
description = EXCLUDED.description";

/// `DELETE parameter_translation` with the image of the deleted row from a CTE.
const DELETE_TRANSLATION: &str = "\
WITH gone AS (DELETE FROM parameter_translation t USING parameter p \
WHERE t.parameter_id = p.id AND p.recipe_version_id = $1 AND p.parameter_key = $2 \
AND t.lang = $3 RETURNING t.*) SELECT to_jsonb(gone) FROM gone";

/// The table name in the error of a write that finds no translation.
const TRANSLATION_TABLE: &str = "parameter_translation";

const UPDATE_DEFINITION: &str = "\
UPDATE parameter SET unit_id = $3, tag = $4, name_en = $5, description_en = $6, \
plc_type_id = $7, s7_address = $8, ordinal = $9 \
WHERE recipe_version_id = $1 AND parameter_key = $2";

/// The limit columns of a parameter in the form of the `unnest` arguments.
struct LimitColumns {
    paths: Vec<String>,
    minimums: Vec<Option<Decimal>>,
    maximums: Vec<Option<Decimal>>,
    lengths: Vec<Option<i16>>,
}

impl LimitColumns {
    fn from_limits(limits: &[StoredLimit]) -> Result<Self, StorageError> {
        let lengths = limits
            .iter()
            .map(|limit| limit.max_length.map(i16::try_from).transpose())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| check_violation())?;
        Ok(Self {
            paths: limits.iter().map(|l| l.leaf_path.to_string()).collect(),
            minimums: limits.iter().map(|l| l.min).collect(),
            maximums: limits.iter().map(|l| l.max).collect(),
            lengths,
        })
    }
}

/// Insert all limit rows of a parameter with one statement. No limit inserts nothing.
async fn insert_limits(
    connection: &mut PgConnection,
    (parameter, version): (ParameterId, RecipeVersionId),
    limits: &[StoredLimit],
) -> Result<(), StorageError> {
    if limits.is_empty() {
        return Ok(());
    }
    let columns = LimitColumns::from_limits(limits)?;
    sqlx::query(INSERT_LIMITS)
        .bind(*parameter.as_uuid())
        .bind(*version.as_uuid())
        .bind(columns.paths)
        .bind(columns.minimums)
        .bind(columns.maximums)
        .bind(columns.lengths)
        .execute(connection)
        .await
        .map_err(map_error)?;
    Ok(())
}

/// Insert the parameter row and its limit rows, then read the image in a separate statement.
///
/// A known tag gives `Duplicate` with `parameter_tag_unique` from the error map. A missing
/// version gives `MissingReference`.
pub(super) async fn insert(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    parameter: &Parameter,
) -> Result<RowImage, StorageError> {
    let ordinal = i32::try_from(parameter.ordinal().value()).map_err(|_| check_violation())?;
    let inserted = sqlx::query(INSERT_PARAMETER)
        .bind(*version.as_uuid())
        .bind(*parameter.id().as_uuid())
        .bind(*parameter.key().as_uuid())
        .bind(*parameter.unit().as_uuid())
        .bind(parameter.tag().as_str())
        .bind(parameter.name_en().as_str())
        .bind(parameter.description_en().as_str())
        .bind(*parameter.plc_type().as_uuid())
        .bind(parameter.s7_address().map(|address| address.to_string()))
        .bind(parameter.value().as_json())
        .bind(ordinal)
        .execute(&mut *connection)
        .await
        .map_err(map_error)?;
    if inserted.rows_affected() == 0 {
        return Err(StorageError::MissingReference {
            constraint: PARAMETER_VERSION_PLATFORM.to_owned(),
        });
    }
    insert_limits(connection, (parameter.id(), version), parameter.limits()).await?;
    let image: Value = sqlx::query_scalar(SELECT_IMAGE)
        .bind(*parameter.id().as_uuid())
        .fetch_one(connection)
        .await
        .map_err(map_error)?;
    Ok(RowImage::new(image))
}

/// Write the definition columns of a parameter. The value, the limits, and the translations
/// stay unchanged.
pub(super) async fn update_definition(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    parameter: &Parameter,
) -> Result<(), StorageError> {
    let ordinal = i32::try_from(parameter.ordinal().value()).map_err(|_| check_violation())?;
    let updated = sqlx::query(UPDATE_DEFINITION)
        .bind(*version.as_uuid())
        .bind(*parameter.key().as_uuid())
        .bind(*parameter.unit().as_uuid())
        .bind(parameter.tag().as_str())
        .bind(parameter.name_en().as_str())
        .bind(parameter.description_en().as_str())
        .bind(*parameter.plc_type().as_uuid())
        .bind(parameter.s7_address().map(|address| address.to_string()))
        .bind(ordinal)
        .execute(connection)
        .await
        .map_err(map_error)?;
    if updated.rows_affected() == 0 {
        return Err(missing_row(PARAMETER_TABLE));
    }
    Ok(())
}

/// Write the value of a parameter. The limit rows and the translation rows stay unchanged.
pub(super) async fn set_value(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    key: ParameterKey,
    value: &StoredValue,
) -> Result<(), StorageError> {
    let updated = sqlx::query(SET_VALUE)
        .bind(*version.as_uuid())
        .bind(*key.as_uuid())
        .bind(value.as_json())
        .execute(connection)
        .await
        .map_err(map_error)?;
    if updated.rows_affected() == 0 {
        return Err(missing_row(PARAMETER_TABLE));
    }
    Ok(())
}

/// Delete the limit rows of a parameter and insert the new ones. The value stays unchanged.
pub(super) async fn replace_limits(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    key: ParameterKey,
    limits: &[StoredLimit],
) -> Result<(), StorageError> {
    let parameter = sqlx::query_scalar::<_, Uuid>(SELECT_PARAMETER)
        .bind(*version.as_uuid())
        .bind(*key.as_uuid())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?
        .ok_or_else(|| missing_row(PARAMETER_TABLE))?;
    sqlx::query(DELETE_LIMITS)
        .bind(parameter)
        .execute(&mut *connection)
        .await
        .map_err(map_error)?;
    insert_limits(connection, (ParameterId::from(parameter), version), limits).await
}

/// Insert a translation, or replace the name and the description of the same language.
pub(super) async fn upsert_translation(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    key: ParameterKey,
    translation: &Translation,
) -> Result<(), StorageError> {
    let written = sqlx::query(UPSERT_TRANSLATION)
        .bind(*version.as_uuid())
        .bind(*key.as_uuid())
        .bind(translation.lang.as_str())
        .bind(translation.name.as_str())
        .bind(
            translation
                .description
                .as_ref()
                .map_or("", LongText::as_str),
        )
        .execute(connection)
        .await
        .map_err(map_error)?;
    if written.rows_affected() == 0 {
        return Err(missing_row(PARAMETER_TABLE));
    }
    Ok(())
}

/// Delete one translation and return the image of the deleted row.
pub(super) async fn delete_translation(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    key: ParameterKey,
    lang: &LangCode,
) -> Result<RowImage, StorageError> {
    let image: Option<Value> = sqlx::query_scalar(DELETE_TRANSLATION)
        .bind(*version.as_uuid())
        .bind(*key.as_uuid())
        .bind(lang.as_str())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    image
        .map(RowImage::new)
        .ok_or_else(|| missing_row(TRANSLATION_TABLE))
}

/// Delete a parameter with its child rows and return the image from before the delete.
pub(super) async fn delete(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    key: ParameterKey,
) -> Result<RowImage, StorageError> {
    let image: Option<Value> = sqlx::query_scalar(DELETE_PARAMETER)
        .bind(*version.as_uuid())
        .bind(*key.as_uuid())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    image
        .map(RowImage::new)
        .ok_or_else(|| missing_row(PARAMETER_TABLE))
}
