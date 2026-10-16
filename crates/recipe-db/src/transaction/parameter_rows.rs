//! SQL of `version_parameters` (task 9.5, `data-model.md` sections 3.12 to 3.14, design D6).
//!
//! The read runs three set queries: the parameters, the limits, and the translations of the
//! version. Rust joins them. An order of tags, leaf paths, or languages uses `COLLATE "C"`, so
//! the byte order does not depend on the collation of the cluster.

use std::collections::BTreeMap;

use recipe_app::error::StorageError;
use recipe_domain::plant::{LeafPath, PlcTypeId, S7Address, UnitId};
use recipe_domain::recipe::{
    LangCode, Ordinal, ParameterId, ParameterKey, ParameterTag, RecipeVersionId, StoredLimit,
    StoredParameter, StoredValue, Translation,
};
use recipe_domain::text::{LongText, Text};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{corrupt, map_error};

const SELECT_PARAMETERS: &str = "\
SELECT id, parameter_key, unit_id, tag, name_en, description_en, plc_type_id, s7_address, \
value, ordinal FROM parameter WHERE recipe_version_id = $1 ORDER BY tag COLLATE \"C\"";

const SELECT_LIMITS: &str = "\
SELECT parameter_id, leaf_path, min_value, max_value, max_length FROM parameter_limit \
WHERE recipe_version_id = $1 ORDER BY parameter_id, leaf_path COLLATE \"C\"";

const SELECT_TRANSLATIONS: &str = "\
SELECT parameter_id, lang, name, description FROM parameter_translation \
WHERE recipe_version_id = $1 ORDER BY parameter_id, lang COLLATE \"C\"";

/// One `parameter` row. `parameter_reads` reads the same columns for one key.
#[derive(sqlx::FromRow)]
pub(super) struct ParameterRecord {
    id: Uuid,
    parameter_key: Uuid,
    unit_id: Uuid,
    tag: String,
    name_en: String,
    description_en: String,
    plc_type_id: Uuid,
    s7_address: Option<String>,
    value: Value,
    ordinal: i32,
}

impl ParameterRecord {
    /// Return the primary key of the row. The child rows name it.
    pub(super) const fn id(&self) -> Uuid {
        self.id
    }
}

/// One `parameter_limit` row.
#[derive(sqlx::FromRow)]
pub(super) struct LimitRecord {
    parameter_id: Uuid,
    leaf_path: String,
    min_value: Option<Decimal>,
    max_value: Option<Decimal>,
    max_length: Option<i16>,
}

/// One `parameter_translation` row.
#[derive(sqlx::FromRow)]
pub(super) struct TranslationRecord {
    parameter_id: Uuid,
    lang: String,
    name: String,
    description: String,
}

/// Build the stored limit of a limit row. A corrupt row gives a storage error.
pub(super) fn stored_limit(record: &LimitRecord) -> Result<StoredLimit, StorageError> {
    Ok(StoredLimit {
        leaf_path: LeafPath::parse(&record.leaf_path)
            .map_err(|error| corrupt("parameter_limit", error))?,
        min: record.min_value,
        max: record.max_value,
        max_length: record
            .max_length
            .map(u16::try_from)
            .transpose()
            .map_err(|error| corrupt("parameter_limit", error))?,
    })
}

/// Build the translation of a translation row. An empty description is `None`.
pub(super) fn stored_translation(record: &TranslationRecord) -> Result<Translation, StorageError> {
    let description = if record.description.is_empty() {
        None
    } else {
        Some(
            LongText::parse(&record.description)
                .map_err(|error| corrupt("parameter_translation", error))?,
        )
    };
    Ok(Translation {
        lang: LangCode::parse(&record.lang)
            .map_err(|error| corrupt("parameter_translation", error))?,
        name: Text::parse(&record.name).map_err(|error| corrupt("parameter_translation", error))?,
        description,
    })
}

/// Group the rows of a child table by the parameter that owns them. The order of rows stays.
fn group<T, R>(
    rows: &[R],
    owner: impl Fn(&R) -> Uuid,
    build: impl Fn(&R) -> Result<T, StorageError>,
) -> Result<BTreeMap<Uuid, Vec<T>>, StorageError> {
    let mut grouped: BTreeMap<Uuid, Vec<T>> = BTreeMap::new();
    for row in rows {
        grouped.entry(owner(row)).or_default().push(build(row)?);
    }
    Ok(grouped)
}

/// Build the stored parameter of its row with its limits and translations.
pub(super) fn stored_parameter(
    record: ParameterRecord,
    limits: Vec<StoredLimit>,
    translations: Vec<Translation>,
) -> Result<StoredParameter, StorageError> {
    Ok(StoredParameter {
        id: ParameterId::from(record.id),
        key: ParameterKey::from(record.parameter_key),
        unit: UnitId::from(record.unit_id),
        tag: ParameterTag::parse(&record.tag).map_err(|error| corrupt("parameter", error))?,
        name_en: Text::parse(&record.name_en).map_err(|error| corrupt("parameter", error))?,
        description_en: LongText::parse(&record.description_en)
            .map_err(|error| corrupt("parameter", error))?,
        translations,
        plc_type: PlcTypeId::from(record.plc_type_id),
        s7_address: record
            .s7_address
            .as_deref()
            .map(S7Address::parse)
            .transpose()
            .map_err(|error| corrupt("parameter", error))?,
        limits,
        value: StoredValue::from(record.value),
        ordinal: Ordinal::parse(
            u32::try_from(record.ordinal).map_err(|error| corrupt("parameter", error))?,
        )
        .map_err(|error| corrupt("parameter", error))?,
    })
}

/// Read the parameters of a version with their limits and translations.
pub(super) async fn of_version(
    connection: &mut PgConnection,
    version: RecipeVersionId,
) -> Result<Vec<StoredParameter>, StorageError> {
    let id = *version.as_uuid();
    let parameters = sqlx::query_as::<_, ParameterRecord>(SELECT_PARAMETERS)
        .bind(id)
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let limits = sqlx::query_as::<_, LimitRecord>(SELECT_LIMITS)
        .bind(id)
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let translations = sqlx::query_as::<_, TranslationRecord>(SELECT_TRANSLATIONS)
        .bind(id)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    let mut limits = group(&limits, |row| row.parameter_id, stored_limit)?;
    let mut translations = group(&translations, |row| row.parameter_id, stored_translation)?;
    parameters
        .into_iter()
        .map(|record| {
            let limits = limits.remove(&record.id).unwrap_or_default();
            let translations = translations.remove(&record.id).unwrap_or_default();
            stored_parameter(record, limits, translations)
        })
        .collect()
}
