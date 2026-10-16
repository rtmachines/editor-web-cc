//! In-memory `DiffReads` (task 6.1, design D4).
//!
//! The SQL of `recipe-db` is the model. The flags compare the stored rows of both versions, as
//! the statement compares the columns with `IS DISTINCT FROM`. A limit row matches by its leaf
//! path, and a translation matches by its language.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use recipe_domain::recipe::{LangCode, ParameterKey, RecipeVersionId};

use super::rows::ParameterRow;
use crate::error::StorageError;
use crate::memory::rows::corrupt_rows;
use crate::memory::{MemoryState, MemoryTransaction};
use crate::ports::{ChangeFlags, DiffReads, DiffSides};

/// Return the parameter rows of `version` by key. A side of `None` gives the empty map.
fn rows_by_key(
    state: &MemoryState,
    version: Option<RecipeVersionId>,
) -> BTreeMap<ParameterKey, &ParameterRow> {
    let Some(version) = version else {
        return BTreeMap::new();
    };
    state
        .parameters
        .values()
        .filter(|row| row.version == version)
        .map(|row| (row.key, row))
        .collect()
}

/// Convert a count to the type of the port.
fn count(value: usize) -> Result<u32, StorageError> {
    u32::try_from(value).map_err(|_| corrupt_rows(&"a diff count does not fit u32"))
}

/// Count the changed fields of the definition: the seven fields of ADR-0050 (c).
fn definition_rows(old: &ParameterRow, new: &ParameterRow) -> usize {
    [
        old.tag != new.tag,
        old.unit != new.unit,
        old.plc_type != new.plc_type,
        old.s7_address != new.s7_address,
        old.ordinal != new.ordinal,
        old.name_en != new.name_en,
        old.description_en != new.description_en,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count()
}

/// Count the limit rows of the limits section. A changed type counts every path of both sides.
fn limit_rows(old: &ParameterRow, new: &ParameterRow) -> usize {
    let paths: BTreeSet<&String> = old.limits.keys().chain(new.limits.keys()).collect();
    if old.plc_type != new.plc_type {
        return paths.len();
    }
    paths
        .into_iter()
        .filter(|path| {
            old.limits.get(*path).map(|row| &row.limit)
                != new.limits.get(*path).map(|row| &row.limit)
        })
        .count()
}

/// Count the languages whose translation differs, also a language on one side only.
fn translation_rows(old: &ParameterRow, new: &ParameterRow) -> usize {
    let langs: BTreeSet<&LangCode> = old
        .translations
        .keys()
        .chain(new.translations.keys())
        .collect();
    langs
        .into_iter()
        .filter(|lang| {
            old.translations.get(*lang).map(|row| &row.translation)
                != new.translations.get(*lang).map(|row| &row.translation)
        })
        .count()
}

/// Build the row of a key with the order fields of `row` and no flag set.
fn base_row(state: &MemoryState, row: &ParameterRow) -> Result<ChangeFlags, StorageError> {
    let unit = state
        .units
        .get(&row.unit)
        .ok_or_else(|| corrupt_rows(&"the unit of a parameter does not exist"))?;
    Ok(ChangeFlags {
        key: row.key,
        unit_path: unit.path.clone(),
        ordinal: row.ordinal,
        tag: row.tag.clone(),
        definition: false,
        value: false,
        limits: false,
        translations: false,
        only_old: false,
        only_new: false,
        limit_rows: 0,
        definition_rows: 0,
        translation_rows: 0,
    })
}

/// Build the row of a key on both sides.
fn both_sides(
    state: &MemoryState,
    old: &ParameterRow,
    new: &ParameterRow,
) -> Result<ChangeFlags, StorageError> {
    let base = base_row(state, new)?;
    let limits = count(limit_rows(old, new))?;
    let definitions = count(definition_rows(old, new))?;
    let translations = count(translation_rows(old, new))?;
    Ok(ChangeFlags {
        definition: definitions > 0,
        value: old.plc_type != new.plc_type || old.value != new.value,
        limits: limits > 0,
        translations: translations > 0,
        limit_rows: limits,
        definition_rows: definitions,
        translation_rows: translations,
        ..base
    })
}

/// Build the row of a key on one side only. `is_old` tells the side.
fn one_side(
    state: &MemoryState,
    row: &ParameterRow,
    is_old: bool,
) -> Result<ChangeFlags, StorageError> {
    let base = base_row(state, row)?;
    let limits = count(row.limits.len())?;
    Ok(ChangeFlags {
        limits: limits > 0,
        limit_rows: limits,
        only_old: is_old,
        only_new: !is_old,
        ..base
    })
}

/// Build the row of each key of both versions, in the order of the parameter table.
fn flag_rows(
    state: &MemoryState,
    old: Option<RecipeVersionId>,
    new: RecipeVersionId,
) -> Result<Vec<ChangeFlags>, StorageError> {
    let old = rows_by_key(state, old);
    let new = rows_by_key(state, Some(new));
    let keys: BTreeSet<ParameterKey> = old.keys().chain(new.keys()).copied().collect();
    let mut rows = keys
        .into_iter()
        .map(|key| match (old.get(&key), new.get(&key)) {
            (Some(old), Some(new)) => both_sides(state, old, new),
            (None, Some(new)) => one_side(state, new, false),
            (Some(old), None) => one_side(state, old, true),
            (None, None) => Err(corrupt_rows(&"a diff key has no side")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    rows.sort_by(|left, right| {
        (&left.unit_path, left.ordinal, &left.tag, left.key).cmp(&(
            &right.unit_path,
            right.ordinal,
            &right.tag,
            right.key,
        ))
    });
    Ok(rows)
}

#[async_trait]
impl DiffReads for MemoryTransaction {
    async fn change_flags(
        &mut self,
        old: Option<RecipeVersionId>,
        new: RecipeVersionId,
    ) -> Result<Vec<ChangeFlags>, StorageError> {
        flag_rows(self.state(), old, new)
    }

    async fn pairs(
        &mut self,
        old: Option<RecipeVersionId>,
        new: Option<RecipeVersionId>,
        keys: &[ParameterKey],
    ) -> Result<DiffSides, StorageError> {
        let state = self.state();
        let side = |version| {
            let rows = rows_by_key(state, version);
            keys.iter()
                .filter_map(|key| rows.get(key).map(|row| row.stored()))
                .collect()
        };
        Ok(DiffSides {
            old: side(old),
            new: side(new),
        })
    }
}

#[cfg(test)]
mod tests;
