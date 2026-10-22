//! SQL of `DiffReads` (ADR-0050 (e), (f), design D4, `data-model.md` sections 3.12 to 3.14).
//!
//! `change_flags` runs one statement. It compares both versions inside PostgreSQL and sends no
//! value to the application. The parameters match by key with a full outer join. The child rows
//! match by key and leaf path or language with a full outer join. Each compare uses
//! `IS DISTINCT FROM`, so `NULL` equals `NULL`.
//!
//! The flags compare the stored text and the stored JSONB, but the domain diff compares parsed
//! values. Both agree only for the canonical stored text of `reliability.md` section 4.4. The
//! application writes only that text.
//!
//! `pairs` runs three set statements for each side, as `parameter_rows::of_version` does, and
//! reuses its row mapping. No method takes a lock. The use case opens the transaction with
//! `Isolation::RepeatableRead`, so both methods read one snapshot.

use std::collections::BTreeMap;

use recipe_app::error::StorageError;
use recipe_app::ports::{ChangeFlags, DiffSides};
use recipe_domain::plant::UnitPath;
use recipe_domain::recipe::{
    Ordinal, ParameterKey, ParameterTag, RecipeVersionId, StoredParameter,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::parameter_rows::{
    LimitRecord, ParameterRecord, TranslationRecord, group, stored_limit, stored_parameter,
    stored_translation,
};
use crate::error::{corrupt, map_error};

/// One row for each key of the old version `$1` and the new version `$2`.
///
/// `$1` is `NULL` for a first approval, and then the old side has no row. A changed type counts
/// every distinct leaf path of both sides as a limit row (design D4). A key on one side only has
/// no definition row and no translation row. The order is the order of the parameter table.
const SELECT_FLAGS: &str = "\
WITH old_side AS ( \
    SELECT id, parameter_key, unit_id, tag, name_en, description_en, plc_type_id, s7_address, \
        value, ordinal \
    FROM parameter WHERE recipe_version_id = $1::uuid \
), new_side AS ( \
    SELECT id, parameter_key, unit_id, tag, name_en, description_en, plc_type_id, s7_address, \
        value, ordinal \
    FROM parameter WHERE recipe_version_id = $2::uuid \
), pair AS ( \
    SELECT COALESCE(n.parameter_key, o.parameter_key) AS parameter_key, \
        o.id IS NOT NULL AS has_old, \
        n.id IS NOT NULL AS has_new, \
        COALESCE(n.unit_id, o.unit_id) AS unit_id, \
        COALESCE(n.ordinal, o.ordinal) AS ordinal, \
        COALESCE(n.tag, o.tag) AS tag, \
        o.plc_type_id IS DISTINCT FROM n.plc_type_id AS retyped, \
        o.value IS DISTINCT FROM n.value AS value_differs, \
        (o.tag IS DISTINCT FROM n.tag)::integer \
            + (o.unit_id IS DISTINCT FROM n.unit_id)::integer \
            + (o.plc_type_id IS DISTINCT FROM n.plc_type_id)::integer \
            + (o.s7_address IS DISTINCT FROM n.s7_address)::integer \
            + (o.ordinal IS DISTINCT FROM n.ordinal)::integer \
            + (o.name_en IS DISTINCT FROM n.name_en)::integer \
            + (o.description_en IS DISTINCT FROM n.description_en)::integer AS definition_rows \
    FROM old_side o FULL JOIN new_side n ON n.parameter_key = o.parameter_key \
), old_limit AS ( \
    SELECT p.parameter_key, l.leaf_path, l.min_value, l.max_value, l.max_length \
    FROM parameter_limit l JOIN parameter p ON p.id = l.parameter_id \
    WHERE l.recipe_version_id = $1::uuid AND p.recipe_version_id = $1::uuid \
), new_limit AS ( \
    SELECT p.parameter_key, l.leaf_path, l.min_value, l.max_value, l.max_length \
    FROM parameter_limit l JOIN parameter p ON p.id = l.parameter_id \
    WHERE l.recipe_version_id = $2::uuid AND p.recipe_version_id = $2::uuid \
), limit_count AS ( \
    SELECT COALESCE(n.parameter_key, o.parameter_key) AS parameter_key, \
        count(*) AS paths, \
        count(*) FILTER (WHERE o.leaf_path IS NULL OR n.leaf_path IS NULL \
            OR (o.min_value, o.max_value, o.max_length) \
                IS DISTINCT FROM (n.min_value, n.max_value, n.max_length)) AS changes \
    FROM old_limit o FULL JOIN new_limit n \
        ON n.parameter_key = o.parameter_key AND n.leaf_path = o.leaf_path \
    GROUP BY 1 \
), old_translation AS ( \
    SELECT p.parameter_key, t.lang, t.name, t.description \
    FROM parameter_translation t JOIN parameter p ON p.id = t.parameter_id \
    WHERE t.recipe_version_id = $1::uuid AND p.recipe_version_id = $1::uuid \
), new_translation AS ( \
    SELECT p.parameter_key, t.lang, t.name, t.description \
    FROM parameter_translation t JOIN parameter p ON p.id = t.parameter_id \
    WHERE t.recipe_version_id = $2::uuid AND p.recipe_version_id = $2::uuid \
), translation_count AS ( \
    SELECT COALESCE(n.parameter_key, o.parameter_key) AS parameter_key, count(*) AS changes \
    FROM old_translation o FULL JOIN new_translation n \
        ON n.parameter_key = o.parameter_key AND n.lang = o.lang \
    WHERE o.lang IS NULL OR n.lang IS NULL \
        OR (o.name, o.description) IS DISTINCT FROM (n.name, n.description) \
    GROUP BY 1 \
) \
SELECT pr.parameter_key, u.path, pr.ordinal, pr.tag, pr.has_old, pr.has_new, \
    (pr.has_old AND pr.has_new AND (pr.retyped OR pr.value_differs)) AS value_differs, \
    (CASE WHEN pr.has_old AND pr.has_new AND pr.retyped THEN COALESCE(lc.paths, 0) \
        ELSE COALESCE(lc.changes, 0) END) AS limit_rows, \
    (CASE WHEN pr.has_old AND pr.has_new THEN pr.definition_rows ELSE 0 END)::bigint \
        AS definition_rows, \
    (CASE WHEN pr.has_old AND pr.has_new THEN COALESCE(tc.changes, 0) ELSE 0 END) \
        AS translation_rows \
FROM pair pr \
JOIN unit u ON u.id = pr.unit_id \
LEFT JOIN limit_count lc ON lc.parameter_key = pr.parameter_key \
LEFT JOIN translation_count tc ON tc.parameter_key = pr.parameter_key \
ORDER BY string_to_array(substr(u.path, 2), '.')::integer[], pr.ordinal, \
    pr.tag COLLATE \"C\", pr.parameter_key";

/// The parameters of the version `$1` with a key in `$2`.
const SELECT_PARAMETERS: &str = "\
SELECT id, parameter_key, unit_id, tag, name_en, description_en, plc_type_id, s7_address, \
value, ordinal FROM parameter \
WHERE recipe_version_id = $1 AND parameter_key = ANY($2::uuid[])";

/// The limit rows of the parameters of the version `$1` with a key in `$2`.
const SELECT_LIMITS: &str = "\
SELECT l.parameter_id, l.leaf_path, l.min_value, l.max_value, l.max_length \
FROM parameter_limit l JOIN parameter p ON p.id = l.parameter_id \
WHERE l.recipe_version_id = $1 AND p.recipe_version_id = $1 \
AND p.parameter_key = ANY($2::uuid[]) \
ORDER BY l.parameter_id, l.leaf_path COLLATE \"C\"";

/// The translation rows of the parameters of the version `$1` with a key in `$2`.
const SELECT_TRANSLATIONS: &str = "\
SELECT t.parameter_id, t.lang, t.name, t.description \
FROM parameter_translation t JOIN parameter p ON p.id = t.parameter_id \
WHERE t.recipe_version_id = $1 AND p.recipe_version_id = $1 \
AND p.parameter_key = ANY($2::uuid[]) \
ORDER BY t.parameter_id, t.lang COLLATE \"C\"";

/// One row of `SELECT_FLAGS`: the order fields, the sides, and the counts of one key.
#[derive(sqlx::FromRow)]
struct FlagRecord {
    parameter_key: Uuid,
    path: String,
    ordinal: i32,
    tag: String,
    has_old: bool,
    has_new: bool,
    value_differs: bool,
    limit_rows: i64,
    definition_rows: i64,
    translation_rows: i64,
}

/// Convert a row count of the statement to the type of the port.
fn count(value: i64) -> Result<u32, StorageError> {
    u32::try_from(value).map_err(|error| corrupt("parameter", error))
}

/// Build the flags of one key from its row.
fn flag_row(record: FlagRecord) -> Result<ChangeFlags, StorageError> {
    let ordinal = u32::try_from(record.ordinal).map_err(|error| corrupt("parameter", error))?;
    let limit_rows = count(record.limit_rows)?;
    let definition_rows = count(record.definition_rows)?;
    let translation_rows = count(record.translation_rows)?;
    Ok(ChangeFlags {
        key: ParameterKey::from(record.parameter_key),
        unit_path: UnitPath::parse(&record.path).map_err(|error| corrupt("unit", error))?,
        ordinal: Ordinal::parse(ordinal).map_err(|error| corrupt("parameter", error))?,
        tag: ParameterTag::parse(&record.tag).map_err(|error| corrupt("parameter", error))?,
        definition: definition_rows > 0,
        value: record.value_differs,
        limits: limit_rows > 0,
        translations: translation_rows > 0,
        only_old: !record.has_new,
        only_new: !record.has_old,
        limit_rows,
        definition_rows,
        translation_rows,
    })
}

/// Read one row for each key of `old` and `new`, in the order of the parameter table.
pub(super) async fn change_flags(
    connection: &mut PgConnection,
    old: Option<RecipeVersionId>,
    new: RecipeVersionId,
) -> Result<Vec<ChangeFlags>, StorageError> {
    let records = sqlx::query_as::<_, FlagRecord>(SELECT_FLAGS)
        .bind(old.map(|version| *version.as_uuid()))
        .bind(*new.as_uuid())
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    records.into_iter().map(flag_row).collect()
}

/// Read the parameters of `keys` in `version` with their limits and translations.
///
/// The result follows the order of `keys`. A key that the version does not hold gives no
/// parameter, and a key that `keys` repeats gives one parameter.
async fn side(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    keys: &[ParameterKey],
) -> Result<Vec<StoredParameter>, StorageError> {
    let version = *version.as_uuid();
    let bound: Vec<Uuid> = keys.iter().map(|key| *key.as_uuid()).collect();
    let parameters = sqlx::query_as::<_, ParameterRecord>(SELECT_PARAMETERS)
        .bind(version)
        .bind(&bound)
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let limits = sqlx::query_as::<_, LimitRecord>(SELECT_LIMITS)
        .bind(version)
        .bind(&bound)
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let translations = sqlx::query_as::<_, TranslationRecord>(SELECT_TRANSLATIONS)
        .bind(version)
        .bind(&bound)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    let mut limits = group(&limits, LimitRecord::parameter_id, stored_limit)?;
    let mut translations = group(
        &translations,
        TranslationRecord::parameter_id,
        stored_translation,
    )?;
    let mut built: BTreeMap<ParameterKey, StoredParameter> = BTreeMap::new();
    for record in parameters {
        let limits = limits.remove(&record.id()).unwrap_or_default();
        let translations = translations.remove(&record.id()).unwrap_or_default();
        let parameter = stored_parameter(record, limits, translations)?;
        built.insert(parameter.key, parameter);
    }
    Ok(keys.iter().filter_map(|key| built.remove(key)).collect())
}

/// Read the stored parameters of `keys` on each side. A side of `None` reads no row.
pub(super) async fn pairs(
    connection: &mut PgConnection,
    old: Option<RecipeVersionId>,
    new: Option<RecipeVersionId>,
    keys: &[ParameterKey],
) -> Result<DiffSides, StorageError> {
    if keys.is_empty() {
        return Ok(DiffSides::default());
    }
    let old = match old {
        Some(version) => side(&mut *connection, version, keys).await?,
        None => Vec::new(),
    };
    let new = match new {
        Some(version) => side(connection, version, keys).await?,
        None => Vec::new(),
    };
    Ok(DiffSides { old, new })
}
