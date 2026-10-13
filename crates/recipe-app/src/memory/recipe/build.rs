//! Construction and checks of the parameter rows of the in-memory adapters.

use std::collections::BTreeMap;

use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::{Parameter, RecipeVersionId, StoredLimit, Translation};
use recipe_domain::timestamp::Timestamp;

use super::rows::{LimitRow, ParameterRow, TranslationRow};
use crate::error::StorageError;
use crate::memory::rows::{check_violation, duplicate};

/// The name of the primary key of `parameter_limit`.
const LIMIT_PKEY: &str = "parameter_limit_pkey";
/// The largest value of `parameter_limit.max_length`.
const MAX_LENGTH_LIMIT: u16 = 16_382;

/// Check a limit row against the `CHECK` constraints of `parameter_limit`.
fn check_limit(limit: &StoredLimit) -> Result<(), StorageError> {
    let has_range = limit.min.is_some() || limit.max.is_some();
    let is_empty = !has_range && limit.max_length.is_none();
    let has_both_kinds = has_range && limit.max_length.is_some();
    let is_inverted = matches!((limit.min, limit.max), (Some(min), Some(max)) if min > max);
    let has_bad_length = limit
        .max_length
        .is_some_and(|length| !(1..=MAX_LENGTH_LIMIT).contains(&length));
    if is_empty || has_both_kinds || is_inverted || has_bad_length {
        return Err(check_violation());
    }
    Ok(())
}

/// Build the limit rows of one parameter.
///
/// A repeated leaf path gives `Duplicate` with `parameter_limit_pkey`. A row that breaks a
/// `CHECK` gives `E-DB-003`.
pub(crate) fn limit_rows(
    limits: &[StoredLimit],
    now: Timestamp,
) -> Result<BTreeMap<String, LimitRow>, StorageError> {
    let mut rows = BTreeMap::new();
    for limit in limits {
        check_limit(limit)?;
        let row = LimitRow {
            limit: limit.clone(),
            created_at: now,
        };
        if rows.insert(limit.leaf_path.to_string(), row).is_some() {
            return Err(duplicate(LIMIT_PKEY));
        }
    }
    Ok(rows)
}

/// Build the translation row of one translation.
pub(crate) fn translation_row(translation: &Translation, now: Timestamp) -> TranslationRow {
    TranslationRow {
        translation: translation.clone(),
        created_at: now,
    }
}

/// Build the row of a parameter with its child rows.
pub(crate) fn parameter_row(
    (version, platform): (RecipeVersionId, PlcPlatform),
    parameter: &Parameter,
    now: Timestamp,
) -> Result<ParameterRow, StorageError> {
    Ok(ParameterRow {
        id: parameter.id(),
        version,
        platform,
        key: parameter.key(),
        unit: parameter.unit(),
        tag: parameter.tag().clone(),
        name_en: parameter.name_en().clone(),
        description_en: parameter.description_en().clone(),
        plc_type: parameter.plc_type(),
        s7_address: parameter.s7_address(),
        value: parameter.value().clone(),
        ordinal: parameter.ordinal(),
        created_at: now,
        limits: limit_rows(parameter.limits(), now)?,
        translations: parameter
            .translations()
            .iter()
            .map(|(lang, translation)| (lang.clone(), translation_row(translation, now)))
            .collect(),
    })
}
