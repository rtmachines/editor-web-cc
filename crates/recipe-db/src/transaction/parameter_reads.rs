//! SQL of `parameter_page`, `unit_counts`, and `parameter_by_key` of `RecipeReads` (design D5,
//! `data-model.md` sections 3.2, 3.12 to 3.14).
//!
//! The page and the counts run one statement each. The read by key runs three set statements,
//! as `parameter_rows::of_version` does, and reuses its row mapping. The unit path orders by its
//! numbers, as `UnitPath::cmp` does, so `U1.2` comes before `U1.10`. `COLLATE "C"` orders the
//! tag by bytes, whatever the collation of the database is.

use recipe_app::error::StorageError;
use recipe_app::ports::{ParameterRow, UnitCount};
use recipe_domain::plant::{PlcTypeId, S7Address, UnitId, UnitPath};
use recipe_domain::recipe::{
    Ordinal, ParameterKey, ParameterTag, RecipeVersionId, StoredParameter,
};
use recipe_domain::text::Text;
use sqlx::PgConnection;
use uuid::Uuid;

use super::parameter_rows::{
    LimitRecord, ParameterRecord, TranslationRecord, stored_limit, stored_parameter,
    stored_translation,
};
use super::window::bind_window;
use crate::error::{corrupt, map_error};

/// One page of the parameters of the version `$1`. The unit `$2` is optional. The order is the
/// numbers of the unit path, the ordinal, and the tag.
const SELECT_PAGE: &str = "\
SELECT p.parameter_key, p.unit_id, u.path, p.tag, p.name_en, p.ordinal, p.plc_type_id, \
    p.s7_address \
FROM parameter p JOIN unit u ON u.id = p.unit_id \
WHERE p.recipe_version_id = $1 AND ($2::uuid IS NULL OR p.unit_id = $2) \
ORDER BY string_to_array(substr(u.path, 2), '.')::integer[], p.ordinal, p.tag COLLATE \"C\" \
OFFSET $3 LIMIT $4";

/// The count of the parameters of the version `$1` in each unit, in the order of the path.
const SELECT_COUNTS: &str = "\
SELECT u.id, u.path, count(*) AS count \
FROM parameter p JOIN unit u ON u.id = p.unit_id \
WHERE p.recipe_version_id = $1 \
GROUP BY u.id, u.path \
ORDER BY string_to_array(substr(u.path, 2), '.')::integer[]";

const SELECT_PARAMETER: &str = "\
SELECT id, parameter_key, unit_id, tag, name_en, description_en, plc_type_id, s7_address, \
value, ordinal FROM parameter WHERE recipe_version_id = $1 AND parameter_key = $2";

const SELECT_LIMITS: &str = "\
SELECT parameter_id, leaf_path, min_value, max_value, max_length FROM parameter_limit \
WHERE recipe_version_id = $1 AND parameter_id = $2 ORDER BY leaf_path COLLATE \"C\"";

const SELECT_TRANSLATIONS: &str = "\
SELECT parameter_id, lang, name, description FROM parameter_translation \
WHERE recipe_version_id = $1 AND parameter_id = $2 ORDER BY lang COLLATE \"C\"";

#[derive(sqlx::FromRow)]
struct PageRecord {
    parameter_key: Uuid,
    unit_id: Uuid,
    path: String,
    tag: String,
    name_en: String,
    ordinal: i32,
    plc_type_id: Uuid,
    s7_address: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CountRecord {
    id: Uuid,
    path: String,
    count: i64,
}

/// Parse the path of a `unit` row. A row that fails the grammar is corrupt.
fn unit_path(text: &str) -> Result<UnitPath, StorageError> {
    UnitPath::parse(text).map_err(|error| corrupt("unit", error))
}

/// Build the table row of a stored row.
fn table_row(record: PageRecord) -> Result<ParameterRow, StorageError> {
    Ok(ParameterRow {
        key: ParameterKey::from(record.parameter_key),
        unit: UnitId::from(record.unit_id),
        unit_path: unit_path(&record.path)?,
        tag: ParameterTag::parse(&record.tag).map_err(|error| corrupt("parameter", error))?,
        name_en: Text::parse(&record.name_en).map_err(|error| corrupt("parameter", error))?,
        ordinal: Ordinal::parse(
            u32::try_from(record.ordinal).map_err(|error| corrupt("parameter", error))?,
        )
        .map_err(|error| corrupt("parameter", error))?,
        plc_type: PlcTypeId::from(record.plc_type_id),
        s7_address: record
            .s7_address
            .as_deref()
            .map(S7Address::parse)
            .transpose()
            .map_err(|error| corrupt("parameter", error))?,
    })
}

/// Build the count row of a stored row.
fn unit_count(record: CountRecord) -> Result<UnitCount, StorageError> {
    Ok(UnitCount {
        unit: UnitId::from(record.id),
        path: unit_path(&record.path)?,
        count: u32::try_from(record.count).map_err(|error| corrupt("parameter", error))?,
    })
}

/// Read one page of the parameters of `version`, in the order of the table.
pub(super) async fn page(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    unit: Option<UnitId>,
    offset: u64,
    limit: u32,
) -> Result<Vec<ParameterRow>, StorageError> {
    let (offset, limit) = bind_window(offset, limit)?;
    let records = sqlx::query_as::<_, PageRecord>(SELECT_PAGE)
        .bind(*version.as_uuid())
        .bind(unit.map(|unit| *unit.as_uuid()))
        .bind(offset)
        .bind(limit)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    records.into_iter().map(table_row).collect()
}

/// Count the parameters of `version` in each unit that holds one.
pub(super) async fn counts(
    connection: &mut PgConnection,
    version: RecipeVersionId,
) -> Result<Vec<UnitCount>, StorageError> {
    let records = sqlx::query_as::<_, CountRecord>(SELECT_COUNTS)
        .bind(*version.as_uuid())
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    records.into_iter().map(unit_count).collect()
}

/// Read the limit rows and the translation rows of one parameter and build it.
async fn with_children(
    connection: &mut PgConnection,
    version: Uuid,
    record: ParameterRecord,
) -> Result<StoredParameter, StorageError> {
    let id = *record.id().as_uuid();
    let limits = sqlx::query_as::<_, LimitRecord>(SELECT_LIMITS)
        .bind(version)
        .bind(id)
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let translations = sqlx::query_as::<_, TranslationRecord>(SELECT_TRANSLATIONS)
        .bind(version)
        .bind(id)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    let limits = limits.iter().map(stored_limit).collect::<Result<_, _>>()?;
    let translations = translations
        .iter()
        .map(stored_translation)
        .collect::<Result<_, _>>()?;
    stored_parameter(record, limits, translations)
}

/// Read the parameter with `key` in `version`, with its limits and its translations.
///
/// A missing parameter gives `None` after one statement.
pub(super) async fn by_key(
    connection: &mut PgConnection,
    version: RecipeVersionId,
    key: ParameterKey,
) -> Result<Option<StoredParameter>, StorageError> {
    let version = *version.as_uuid();
    let record = sqlx::query_as::<_, ParameterRecord>(SELECT_PARAMETER)
        .bind(version)
        .bind(*key.as_uuid())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    match record {
        Some(record) => with_children(connection, version, record).await.map(Some),
        None => Ok(None),
    }
}
