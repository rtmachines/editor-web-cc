//! In-memory `RecipeReads` (design D5).
//!
//! The statements of `recipe-db` are the model. The text filter folds case with `to_lowercase` on
//! both sides. The rows come in the same order as the `ORDER BY` of those statements.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use recipe_domain::audit::AuditEntry;
use recipe_domain::identity::{Login, UserId};
use recipe_domain::plant::{MachineCode, UnitId, UnitPath};
use recipe_domain::recipe::{
    LangCode, ParameterKey, RecipeId, RecipeVersionId, StoredParameter, StoredValue, VersionState,
};
use recipe_domain::timestamp::Timestamp;

use super::rows::{ParameterRow as StoredRow, RecipeRow, VersionRow};
use crate::error::StorageError;
use crate::memory::plant::check_offset;
use crate::memory::rows::{MachineRow, UnitRow, corrupt_rows};
use crate::memory::{MemoryState, MemoryTransaction};
use crate::ports::{
    ExportableRecipe, ExportableRecipes, ParameterRow, RecipeFilter, RecipeReads, RecipeSummary,
    UnitCount, VersionDetail, VersionSummary,
};

/// Skip `offset` rows and keep at most `limit` rows.
///
/// A count that does not fit `usize` is larger than any list, so it keeps its meaning.
fn window<T>(
    rows: impl Iterator<Item = T>,
    (offset, limit): (u64, u32),
) -> impl Iterator<Item = T> {
    let skip = usize::try_from(offset).unwrap_or(usize::MAX);
    let take = usize::try_from(limit).unwrap_or(usize::MAX);
    rows.skip(skip).take(take)
}

/// Tell whether the reference number or the name of the row contains `needle` in lower case.
fn contains(row: &RecipeRow, needle: &str) -> bool {
    row.reference.to_string().to_lowercase().contains(needle)
        || row.name.as_str().to_lowercase().contains(needle)
}

/// Tell whether the recipe has a version in `state`.
fn has_version_in(state: &MemoryState, recipe: RecipeId, wanted: VersionState) -> bool {
    state
        .versions
        .values()
        .any(|version| version.recipe == recipe && version.state() == Ok(wanted))
}

/// Return the number of the version of `recipe` that is in `wanted`, or `None`.
fn number_in(state: &MemoryState, recipe: RecipeId, wanted: VersionState) -> Option<u32> {
    state
        .versions
        .values()
        .find(|version| version.recipe == recipe && version.state() == Ok(wanted))
        .map(|version| version.number)
}

/// Tell whether the recipe row passes every part of `filter`.
fn passes(
    state: &MemoryState,
    row: &RecipeRow,
    filter: &RecipeFilter,
    needle: Option<&str>,
) -> bool {
    let machine_matches = filter.machine.is_none_or(|code| {
        state
            .machines
            .get(&row.machine)
            .is_some_and(|machine| machine.code == code && machine.archived_at.is_none())
    });
    (filter.include_deleted || row.deleted_at.is_none())
        && machine_matches
        && needle.is_none_or(|text| contains(row, text))
        && filter
            .status
            .is_none_or(|wanted| has_version_in(state, row.id, wanted))
}

/// Build the list row of a recipe.
fn summary(state: &MemoryState, row: &RecipeRow) -> Result<RecipeSummary, StorageError> {
    let machine = state
        .machines
        .get(&row.machine)
        .ok_or_else(|| corrupt_rows(&"the machine of a recipe does not exist"))?;
    Ok(RecipeSummary {
        reference: row.reference.clone(),
        name: row.name.clone(),
        machine_code: machine.code,
        deleted: row.deleted_at.is_some(),
        approved: number_in(state, row.id, VersionState::Approved),
        draft: number_in(state, row.id, VersionState::LiveDraft),
    })
}

/// Return one page of the recipes that match `filter`, in the byte order of the reference number.
fn recipe_page(
    state: &MemoryState,
    filter: &RecipeFilter,
    page: (u64, u32),
) -> Result<Vec<RecipeSummary>, StorageError> {
    let needle = filter
        .text
        .as_ref()
        .map(|text| text.as_str().to_lowercase());
    let mut rows: Vec<&RecipeRow> = state
        .recipes
        .values()
        .filter(|row| passes(state, row, filter, needle.as_deref()))
        .collect();
    rows.sort_by(|left, right| left.reference.cmp(&right.reference));
    window(rows.into_iter(), page)
        .map(|row| summary(state, row))
        .collect()
}

/// Return one page of the versions of `recipe`, the newest number first.
fn versions_of(
    state: &MemoryState,
    recipe: RecipeId,
    page: (u64, u32),
) -> Result<Vec<VersionSummary>, StorageError> {
    let mut rows: Vec<&VersionRow> = state
        .versions
        .values()
        .filter(|row| row.recipe == recipe)
        .collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.number));
    window(rows.into_iter(), page)
        .map(|row| {
            Ok(VersionSummary {
                number: row.number,
                state: row.state().map_err(|reason| corrupt_rows(&reason))?,
                change_note: row.change_note.clone(),
                created_at: row.created_at,
                approved_at: row.approved_at,
            })
        })
        .collect()
}

/// Return the login of the user `id`. A missing row breaks the foreign key, so the storage is corrupt.
fn login_of(state: &MemoryState, id: UserId) -> Result<Login, StorageError> {
    state
        .users
        .get(&id)
        .map(|user| user.login.clone())
        .ok_or_else(|| corrupt_rows(&"the user of a version does not exist"))
}

/// Return the number of the base version `id`. A missing row breaks the foreign key, so the
/// storage is corrupt.
fn number_of(state: &MemoryState, id: RecipeVersionId) -> Result<u32, StorageError> {
    state
        .versions
        .get(&id)
        .map(|base| base.number)
        .ok_or_else(|| corrupt_rows(&"the base of a version does not exist"))
}

/// Build the detail of a version row.
fn detail(state: &MemoryState, row: &VersionRow) -> Result<VersionDetail, StorageError> {
    Ok(VersionDetail {
        id: row.id,
        recipe: row.recipe,
        number: row.number,
        state: row.state().map_err(|reason| corrupt_rows(&reason))?,
        change_note: row.change_note.clone(),
        content_sha256: row.content_sha256,
        based_on: row
            .based_on
            .map(|base| number_of(state, base))
            .transpose()?,
        created_by: login_of(state, row.created_by)?,
        approved_by: row
            .approved_by
            .map(|approver| login_of(state, approver))
            .transpose()?,
        created_at: row.created_at,
        approved_at: row.approved_at,
        updated_at: row.updated_at,
        row_version: row.row_version,
    })
}

/// Return the login of the actor of the entry of `recipe` that was committed at `at`.
///
/// Two entries at one instant give the entry that was written last, as the SQL statement does.
fn actor_at(
    state: &MemoryState,
    audit: &[(Timestamp, AuditEntry)],
    (recipe, at): (RecipeId, Timestamp),
) -> Result<Option<Login>, StorageError> {
    let newest = audit
        .iter()
        .rev()
        .find(|(time, entry)| *time == at && entry.target().aggregate_id() == recipe.as_uuid());
    newest
        .and_then(|(_, entry)| entry.actor())
        .map(|actor| login_of(state, actor))
        .transpose()
}

/// Join the parameter rows of `version` with their unit rows, in the order of the table.
///
/// The order is the unit path by its numbers (`U1.2` before `U1.10`), the ordinal, and the tag.
fn table_rows(
    state: &MemoryState,
    version: RecipeVersionId,
    unit: Option<UnitId>,
) -> Result<Vec<(&StoredRow, &UnitRow)>, StorageError> {
    let mut rows = Vec::new();
    for row in state.parameters.values() {
        if row.version != version || unit.is_some_and(|wanted| wanted != row.unit) {
            continue;
        }
        let unit_row = state
            .units
            .get(&row.unit)
            .ok_or_else(|| corrupt_rows(&"the unit of a parameter does not exist"))?;
        rows.push((row, unit_row));
    }
    rows.sort_by_cached_key(|(row, unit_row)| {
        (unit_row.path.clone(), row.ordinal, row.tag.clone())
    });
    Ok(rows)
}

/// Build the table row of a parameter.
///
/// `lang` selects the stored translation whose tag equals it. `None` reads no translation.
fn table_row(row: &StoredRow, unit: &UnitRow, lang: Option<&LangCode>) -> ParameterRow {
    ParameterRow {
        key: row.key,
        unit: row.unit,
        unit_path: unit.path.clone(),
        tag: row.tag.clone(),
        name_en: row.name_en.clone(),
        translated_name: lang
            .and_then(|lang| row.translations.get(lang))
            .map(|stored| stored.translation.name.clone()),
        ordinal: row.ordinal,
        plc_type: row.plc_type,
        s7_address: row.s7_address,
    }
}

/// Count the parameters of `version` in each unit, in the numeric order of the unit path.
fn counts_of(
    state: &MemoryState,
    version: RecipeVersionId,
) -> Result<Vec<UnitCount>, StorageError> {
    let rows = table_rows(state, version, None)?;
    let units: BTreeMap<(&UnitPath, UnitId), &UnitRow> = rows
        .iter()
        .map(|(_, unit)| ((&unit.path, unit.id), *unit))
        .collect();
    units
        .into_values()
        .map(|unit| {
            let count = rows.iter().filter(|(_, other)| other.id == unit.id).count();
            Ok(UnitCount {
                unit: unit.id,
                path: unit.path.clone(),
                count: u32::try_from(count)
                    .map_err(|_| corrupt_rows(&"a unit holds more parameters than u32"))?,
            })
        })
        .collect()
}

/// Return the machine row that `code` names for the transfer routes (ADR-0109 (j)).
///
/// The live machine wins. Without one, the machine with the latest `archived_at` wins, and the
/// higher id breaks a tie, as `ORDER BY archived_at DESC, id DESC` does.
pub(in crate::memory) fn resolve_machine(
    state: &MemoryState,
    code: MachineCode,
) -> Option<&MachineRow> {
    let same_code = state.machines.values().filter(|row| row.code == code);
    let live = same_code.clone().find(|row| row.archived_at.is_none());
    live.or_else(|| same_code.max_by_key(|row| (row.archived_at, row.id)))
}

/// Build the row of a recipe for the list of exportable recipes, or `None` for a recipe without
/// an `Approved` version.
fn exportable_row(
    state: &MemoryState,
    recipe: &RecipeRow,
) -> Result<Option<ExportableRecipe>, StorageError> {
    let approved = state.versions.values().find(|version| {
        version.recipe == recipe.id && version.state() == Ok(VersionState::Approved)
    });
    approved
        .map(|version| {
            let (Some(approved_at), Some(content_sha256)) =
                (version.approved_at, version.content_sha256)
            else {
                return Err(corrupt_rows(&"an approved version has no approval or hash"));
            };
            Ok(ExportableRecipe {
                reference: recipe.reference.clone(),
                name: recipe.name.clone(),
                version_number: version.number,
                approved_at,
                content_sha256,
            })
        })
        .transpose()
}

/// Read the list of exportable recipes of the machine that `code` names.
fn exportable_of(
    state: &MemoryState,
    code: MachineCode,
) -> Result<Option<ExportableRecipes>, StorageError> {
    let Some(machine) = resolve_machine(state, code) else {
        return Ok(None);
    };
    let mut recipes: Vec<&RecipeRow> = state
        .recipes
        .values()
        .filter(|row| row.machine == machine.id && row.deleted_at.is_none())
        .collect();
    recipes.sort_by(|left, right| left.reference.cmp(&right.reference));
    let recipes = recipes
        .into_iter()
        .filter_map(|row| exportable_row(state, row).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(ExportableRecipes {
        code: machine.code,
        platform: machine.platform,
        archived: machine.archived_at.is_some(),
        recipes,
    }))
}

#[async_trait]
impl RecipeReads for MemoryTransaction {
    async fn list_recipes(
        &mut self,
        filter: &RecipeFilter,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<RecipeSummary>, StorageError> {
        check_offset(offset)?;
        recipe_page(self.state(), filter, (offset, limit))
    }

    async fn version_page(
        &mut self,
        recipe: RecipeId,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<VersionSummary>, StorageError> {
        check_offset(offset)?;
        versions_of(self.state(), recipe, (offset, limit))
    }

    async fn version_detail(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<VersionDetail>, StorageError> {
        let state = self.state();
        state
            .versions
            .get(&version)
            .map(|row| detail(state, row))
            .transpose()
    }

    async fn last_change(
        &mut self,
        recipe: RecipeId,
        at: Timestamp,
    ) -> Result<Option<Login>, StorageError> {
        actor_at(self.state(), &self.audit_at_begin(), (recipe, at))
    }

    async fn parameter_page(
        &mut self,
        version: RecipeVersionId,
        unit: Option<UnitId>,
        lang: Option<&LangCode>,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<ParameterRow>, StorageError> {
        check_offset(offset)?;
        let rows = table_rows(self.state(), version, unit)?;
        Ok(window(rows.into_iter(), (offset, limit))
            .map(|(row, unit_row)| table_row(row, unit_row, lang))
            .collect())
    }

    async fn parameter_values(
        &mut self,
        version: RecipeVersionId,
        keys: &[ParameterKey],
    ) -> Result<BTreeMap<ParameterKey, StoredValue>, StorageError> {
        let wanted: BTreeSet<ParameterKey> = keys.iter().copied().collect();
        Ok(self
            .state()
            .parameters
            .values()
            .filter(|row| row.version == version && wanted.contains(&row.key))
            .map(|row| (row.key, row.value.clone()))
            .collect())
    }

    async fn unit_counts(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Vec<UnitCount>, StorageError> {
        counts_of(self.state(), version)
    }

    async fn parameter_by_key(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
    ) -> Result<Option<StoredParameter>, StorageError> {
        Ok(self
            .state()
            .parameters
            .values()
            .find(|row| row.version == version && row.key == key)
            .map(StoredRow::stored))
    }

    async fn exportable_recipes(
        &mut self,
        machine: MachineCode,
    ) -> Result<Option<ExportableRecipes>, StorageError> {
        exportable_of(self.state(), machine)
    }
}
