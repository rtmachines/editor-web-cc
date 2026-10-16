//! In-memory `DraftRepository` (task 4.9).

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use recipe_domain::identity::UserId;
use recipe_domain::recipe::{
    LangCode, Parameter, ParameterId, ParameterKey, RecipeVersionId, StoredDraft, StoredLimit,
    StoredValue, Translation, VersionState,
};

use super::recipe::{
    ParameterRow, limit_rows, parameter_image, parameter_row, translation_row,
    translation_row_image,
};
use super::rows::{
    corrupt_rows, duplicate, illegal_transition, missing_reference, missing_row,
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
        self.state_mut()
            .parameters
            .get_mut(&id)
            .ok_or_else(|| missing_row(PARAMETER_TABLE))
    }

    /// Check the foreign keys and the tag of a parameter that is written to `version`.
    ///
    /// The row `except` is the parameter itself and does not count for the tag.
    fn check_parameter_references(
        &self,
        version: RecipeVersionId,
        parameter: &Parameter,
        except: Option<ParameterId>,
    ) -> Result<(), StorageError> {
        let state = self.state();
        if !state.units.contains_key(&parameter.unit()) {
            return Err(missing_reference(PARAMETER_UNIT_FKEY));
        }
        if !state.types.contains_key(&parameter.plc_type()) {
            return Err(missing_reference(PARAMETER_TYPE_FKEY));
        }
        let tag_taken = state.parameters.values().any(|row| {
            row.version == version && Some(row.id) != except && row.tag == *parameter.tag()
        });
        if tag_taken {
            return Err(duplicate(PARAMETER_TAG_UNIQUE));
        }
        Ok(())
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
        if stored.is_some() {
            self.lock(VERSION_TABLE, LockMode::ForUpdate, &version.to_string());
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
        let state = self.state_mut();
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
        self.state_mut().parameters.insert(row.id, row);
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
            .state_mut()
            .parameters
            .remove(&id)
            .ok_or_else(|| missing_row(PARAMETER_TABLE))?;
        Ok(parameter_image(&row))
    }
}
