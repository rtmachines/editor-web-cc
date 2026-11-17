//! In-memory `DraftRepository` (task 4.9).

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::UserId;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::{
    LIMIT_ROWS_PER_VERSION, LangCode, PARAMETERS_PER_VERSION, Parameter, ParameterId, ParameterKey,
    RecipeVersionId, StoredDraft, StoredLimit, StoredValue, Translation, VersionState,
};

use super::recipe::{
    ParameterRow, limit_rows, parameter_image, parameter_row, translation_row,
    translation_row_image,
};
use super::rows::{
    check_violation, corrupt_rows, duplicate, illegal_transition, missing_reference, missing_row,
    version_not_live_draft,
};
use super::{LockMode, MemoryState, MemoryTransaction, WriteKind};
use crate::error::{PARAMETER_TAG_UNIQUE, StorageError};
use crate::ports::{DraftRepository, RowImage};

/// The table name of the journal for a version lock.
const VERSION_TABLE: &str = "recipe_version";
/// The table name of the missing-row errors for a parameter.
const PARAMETER_TABLE: &str = "parameter";
/// The table name of the missing-row errors for a translation.
const TRANSLATION_TABLE: &str = "parameter_translation";
/// The name of the primary key of `parameter`.
const PARAMETER_PKEY: &str = "parameter_pkey";
/// The name of the unique constraint on the key of a parameter in a version.
const PARAMETER_KEY_UNIQUE: &str = "parameter_key_unique";
/// The name of the foreign key from `parameter` to `recipe_version`.
const PARAMETER_VERSION_PLATFORM: &str = "parameter_version_platform";
/// The name of the foreign key from `parameter` to `unit`.
const PARAMETER_UNIT_FKEY: &str = "parameter_unit_id_fkey";
/// The name of the foreign key from `parameter` to `plc_type`.
const PARAMETER_TYPE_FKEY: &str = "parameter_plc_type_id_fkey";
/// The message of `E-PLT-031`.
const TYPE_NOT_ON_PLATFORM: &str = "The type does not exist on the platform of the version.";
/// The message of `E-PLT-032`.
const TYPE_OF_ANOTHER_MACHINE: &str = "The type belongs to another machine.";
/// The message of `E-RCP-032`.
const UNIT_OF_ANOTHER_MACHINE: &str = "The unit does not belong to the machine of the recipe.";
/// The message of `E-RCP-035`.
const TOO_MANY_PARAMETERS: &str = "The version has more than 4000 parameters.";
/// The message of `E-RCP-036`.
const TOO_MANY_LIMIT_ROWS: &str = "The version has more than 32768 limit rows.";

impl MemoryTransaction {
    /// Require that `version` exists and is a live draft, as trigger 4.1 does.
    fn require_live_draft(&self, version: RecipeVersionId) -> Result<(), StorageError> {
        let Some(row) = self.state().versions.get(&version) else {
            return Err(missing_reference(PARAMETER_VERSION_PLATFORM));
        };
        let state = row.state().map_err(|reason| corrupt_rows(&reason))?;
        if state != VersionState::LiveDraft {
            return Err(version_not_live_draft());
        }
        Ok(())
    }

    /// Find the identifier of the parameter with `key` in `version`.
    fn parameter_id(
        &self,
        version: RecipeVersionId,
        key: ParameterKey,
    ) -> Result<ParameterId, StorageError> {
        self.state()
            .parameters
            .values()
            .find(|row| row.version == version && row.key == key)
            .map(|row| row.id)
            .ok_or_else(|| missing_row(PARAMETER_TABLE))
    }

    /// Find the row of a parameter that a draft command can change.
    fn editable_parameter(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
    ) -> Result<&mut ParameterRow, StorageError> {
        let id = self.parameter_id(version, key)?;
        self.require_live_draft(version)?;
        self.state_for_write()?
            .parameters
            .get_mut(&id)
            .ok_or_else(|| missing_row(PARAMETER_TABLE))
    }

    /// Check the triggers, the platform `CHECK`, the foreign keys, and the budget of a parameter
    /// that is written to `version`.
    ///
    /// The checks run in the order of PostgreSQL: the trigger `enforce_type_platform`, the trigger
    /// `enforce_unit_machine`, the `CHECK` `parameter_s7_address_platform`, the keys and the tag,
    /// and the statement trigger `version_size_budget`. The row `except` is the parameter itself.
    /// It does not count for the tag. A parameter with `except` is a definition change, and the
    /// budget does not apply to it, because no row is inserted.
    fn check_parameter_references(
        &self,
        version: RecipeVersionId,
        parameter: &Parameter,
        except: Option<ParameterId>,
    ) -> Result<(), StorageError> {
        let state = self.state();
        let Some(unit) = state.units.get(&parameter.unit()) else {
            return Err(missing_reference(PARAMETER_UNIT_FKEY));
        };
        let Some(plc_type) = state.types.get(&parameter.plc_type()) else {
            return Err(missing_reference(PARAMETER_TYPE_FKEY));
        };
        let version_row = state
            .versions
            .get(&version)
            .ok_or_else(|| missing_reference(PARAMETER_VERSION_PLATFORM))?;
        let recipe_machine = state
            .recipes
            .get(&version_row.recipe)
            .map(|recipe| recipe.machine);
        if !plc_type
            .plc_type
            .platforms()
            .contains(&version_row.platform)
        {
            return Err(trigger(ErrorCode::PLT_031, TYPE_NOT_ON_PLATFORM));
        }
        if plc_type
            .plc_type
            .machine()
            .is_some_and(|machine| Some(machine) != recipe_machine)
        {
            return Err(trigger(ErrorCode::PLT_032, TYPE_OF_ANOTHER_MACHINE));
        }
        if Some(unit.machine) != recipe_machine {
            return Err(trigger(ErrorCode::RCP_032, UNIT_OF_ANOTHER_MACHINE));
        }
        if !address_fits(version_row.platform, parameter.s7_address().is_some()) {
            return Err(check_violation());
        }
        let tag_taken = state.parameters.values().any(|row| {
            row.version == version && Some(row.id) != except && row.tag == *parameter.tag()
        });
        if tag_taken {
            return Err(duplicate(PARAMETER_TAG_UNIQUE));
        }
        if except.is_none() {
            self.check_insert_budget(version, parameter)?;
        }
        Ok(())
    }

    /// Count the rows of `version` after the insert of `parameter`, as the statement triggers do.
    fn check_insert_budget(
        &self,
        version: RecipeVersionId,
        parameter: &Parameter,
    ) -> Result<(), StorageError> {
        let rows = || {
            self.state()
                .parameters
                .values()
                .filter(move |row| row.version == version)
        };
        // The new parameter is the row after the existing rows, so the count exceeds the budget
        // when the existing rows already fill it.
        if rows().count() >= usize_of(PARAMETERS_PER_VERSION) {
            return Err(trigger(ErrorCode::RCP_035, TOO_MANY_PARAMETERS));
        }
        let limit_rows: usize = rows()
            .map(|row| row.limits.len())
            .chain(std::iter::once(parameter.limits().len()))
            .sum();
        check_limit_budget(limit_rows)
    }
}

/// The CHECK `parameter_s7_address_platform`: an S7 300 needs an address, a platform without S7
/// forbids it, and the two other S7 platforms allow both (I5, A31).
const fn address_fits(platform: PlcPlatform, has_address: bool) -> bool {
    match platform {
        PlcPlatform::S7_300 => has_address,
        PlcPlatform::S7_1200 | PlcPlatform::S7_1500 => true,
        PlcPlatform::CodesysIec | PlcPlatform::TwinCat3 => !has_address,
    }
}

/// A count as `usize`. A count above the range of `usize` cannot occur, so it is the maximum.
fn usize_of(count: u64) -> usize {
    usize::try_from(count).unwrap_or(usize::MAX)
}

/// The statement trigger `version_size_budget` on `parameter_limit` gives `E-RCP-036` for a
/// version with more limit rows than the budget.
fn check_limit_budget(limit_rows: usize) -> Result<(), StorageError> {
    if limit_rows > usize_of(LIMIT_ROWS_PER_VERSION) {
        return Err(trigger(ErrorCode::RCP_036, TOO_MANY_LIMIT_ROWS));
    }
    Ok(())
}

/// A trigger rejects the write with its code.
fn trigger(code: ErrorCode, message: &str) -> StorageError {
    StorageError::Constraint {
        code,
        message: message.to_owned(),
    }
}

/// Build the stored draft of `version` from `state`, or `None` when a row is missing.
fn stored_draft(
    state: &MemoryState,
    version: RecipeVersionId,
) -> Result<Option<StoredDraft>, StorageError> {
    let Some(row) = state.versions.get(&version) else {
        return Ok(None);
    };
    let Some(recipe) = state.recipes.get(&row.recipe) else {
        return Ok(None);
    };
    Ok(Some(StoredDraft {
        reference: recipe.reference.clone(),
        machine: recipe.machine,
        platform: row.platform,
        version,
        number: row.number,
        state: row.state().map_err(|reason| corrupt_rows(&reason))?,
        row_version: row.row_version,
        created_by: row.created_by,
        editors: state.editors_of(version),
        parameters: state.parameters_of(version),
    }))
}

#[async_trait]
impl DraftRepository for MemoryTransaction {
    async fn lock_draft(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<StoredDraft>, StorageError> {
        let stored = stored_draft(self.state(), version)?;
        if let Some(draft) = &stored {
            self.lock(VERSION_TABLE, LockMode::ForUpdate, &version.to_string());
            self.record_row_version(version, draft.row_version);
        }
        Ok(stored)
    }

    /// Read the rows that the isolation level of the transaction shows, as PostgreSQL does.
    ///
    /// A `READ COMMITTED` transaction reads the latest committed rows. A `RepeatableRead`
    /// transaction reads the state at `begin` (ADR-0050 (f)). `ApproveVersion` uses the first
    /// level, and `GetApprovalPreview` uses the second level. A hook from
    /// `MemoryStore::on_read_draft` runs first and models a request that commits between the
    /// earlier reads of the transaction and this statement.
    async fn read_draft(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<StoredDraft>, StorageError> {
        self.run_read_draft_hook();
        stored_draft(&self.committed_state(), version)
    }

    async fn touch_draft(
        &mut self,
        version: RecipeVersionId,
        editor: UserId,
    ) -> Result<i64, StorageError> {
        let now = self.now();
        let state = self.state_for_write()?;
        let row = state
            .versions
            .get_mut(&version)
            .ok_or_else(|| missing_row(VERSION_TABLE))?;
        if row.state().map_err(|reason| corrupt_rows(&reason))? != VersionState::LiveDraft {
            return Err(illegal_transition());
        }
        row.bump(now)?;
        let row_version = row.row_version;
        state.editors.entry((version, editor)).or_insert(now);
        self.record_row_version(version, row_version);
        Ok(row_version)
    }

    async fn insert_parameter(
        &mut self,
        version: RecipeVersionId,
        parameter: &Parameter,
    ) -> Result<RowImage, StorageError> {
        self.require_live_draft(version)?;
        self.check_parameter_references(version, parameter, None)?;
        let state = self.state();
        if state.parameters.contains_key(&parameter.id()) {
            return Err(duplicate(PARAMETER_PKEY));
        }
        if state
            .parameters
            .values()
            .any(|row| row.version == version && row.key == parameter.key())
        {
            return Err(duplicate(PARAMETER_KEY_UNIQUE));
        }
        let platform = state
            .versions
            .get(&version)
            .map(|row| row.platform)
            .ok_or_else(|| missing_reference(PARAMETER_VERSION_PLATFORM))?;
        let row = parameter_row((version, platform), parameter, self.now())?;
        let image = parameter_image(&row);
        self.state_for_write()?.parameters.insert(row.id, row);
        Ok(image)
    }

    async fn update_definition(
        &mut self,
        version: RecipeVersionId,
        parameter: &Parameter,
    ) -> Result<(), StorageError> {
        let id = self.parameter_id(version, parameter.key())?;
        self.require_live_draft(version)?;
        self.check_parameter_references(version, parameter, Some(id))?;
        let row = self.editable_parameter(version, parameter.key())?;
        row.unit = parameter.unit();
        row.tag = parameter.tag().clone();
        row.name_en = parameter.name_en().clone();
        row.description_en = parameter.description_en().clone();
        row.plc_type = parameter.plc_type();
        row.s7_address = parameter.s7_address();
        row.ordinal = parameter.ordinal();
        self.record_draft_write(WriteKind::Definition, parameter.key());
        Ok(())
    }

    async fn set_value(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        value: &StoredValue,
    ) -> Result<(), StorageError> {
        self.editable_parameter(version, key)?.value = value.clone();
        self.record_draft_write(WriteKind::Value, key);
        Ok(())
    }

    async fn replace_limits(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        limits: &[StoredLimit],
    ) -> Result<(), StorageError> {
        let rows = limit_rows(limits, self.now())?;
        let id = self.parameter_id(version, key)?;
        self.require_live_draft(version)?;
        // PostgreSQL deletes the old rows first, so only the other parameters and the new rows count.
        let after: usize = self
            .state()
            .parameters
            .values()
            .filter(|row| row.version == version && row.id != id)
            .map(|row| row.limits.len())
            .chain(std::iter::once(rows.len()))
            .sum();
        check_limit_budget(after)?;
        self.editable_parameter(version, key)?.limits = rows;
        self.record_draft_write(WriteKind::Limits, key);
        Ok(())
    }

    async fn upsert_translation(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        translation: &Translation,
    ) -> Result<(), StorageError> {
        let now = self.now();
        let row = self.editable_parameter(version, key)?;
        let mut new = translation_row(translation, now);
        if let Some(old) = row.translations.get(&translation.lang) {
            new.created_at = old.created_at;
        }
        row.translations.insert(translation.lang.clone(), new);
        Ok(())
    }

    async fn delete_translation(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        lang: &LangCode,
    ) -> Result<RowImage, StorageError> {
        let row = self.editable_parameter(version, key)?;
        let removed = row
            .translations
            .remove(lang)
            .ok_or_else(|| missing_row(TRANSLATION_TABLE))?;
        Ok(RowImage::new(translation_row_image(row, &removed)))
    }

    async fn delete_parameter(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
    ) -> Result<RowImage, StorageError> {
        let id = self.parameter_id(version, key)?;
        self.require_live_draft(version)?;
        let row = self
            .state_for_write()?
            .parameters
            .remove(&id)
            .ok_or_else(|| missing_row(PARAMETER_TABLE))?;
        Ok(parameter_image(&row))
    }
}
