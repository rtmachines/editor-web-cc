//! In-memory `RecipeRepository` (task 4.8).

mod build;
mod diff_reads;
mod images;
mod lookup;
mod parameter_image;
mod reads;
#[cfg(test)]
mod reads_tests;
mod rows;
mod seed;
#[cfg(test)]
mod tests;

use async_trait::async_trait;
use recipe_domain::export::Sha256;
use recipe_domain::identity::UserId;
use recipe_domain::plant::MachineId;
use recipe_domain::recipe::{
    ParameterId, Recipe, RecipeId, RecipeVersionId, ReferenceNumber, StoredParameter, VersionState,
    VersionStatus,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use uuid::Uuid;

pub(crate) use build::{limit_rows, parameter_row, translation_row};
use images::{recipe_image, version_image};
pub(crate) use parameter_image::{parameter_image, translation_row_image};
pub(crate) use rows::{ParameterRow, RecipeRow, VersionRow};

use super::rows::{
    check_violation, corrupt_rows, duplicate, illegal_transition, missing_reference, missing_row,
};
use super::{LockMode, MemoryTransaction};
use crate::error::{RECIPE_REFERENCE_UNIQUE, RECIPE_VERSION_ONE_LIVE_DRAFT, StorageError};
use crate::ports::{InsertedRecipe, Locked, NewVersion, RecipeRepository, RowImage, SharedVersion};

/// The table name of the journal for a recipe lock.
const RECIPE_TABLE: &str = "recipe";
/// The table name of the journal for a version lock.
const VERSION_TABLE: &str = "recipe_version";
/// The name of the primary key of `recipe`.
const RECIPE_PKEY: &str = "recipe_pkey";
/// The name of the primary key of `recipe_version`.
const VERSION_PKEY: &str = "recipe_version_pkey";
/// The name of the foreign key from `recipe` to `machine`.
const RECIPE_MACHINE_PLATFORM: &str = "recipe_machine_platform";
/// The name of the foreign key from `recipe_version` to `recipe`.
const VERSION_RECIPE_PLATFORM: &str = "recipe_version_recipe_platform";
/// The name of the unique constraint on the number of a version.
const VERSION_NUMBER_UNIQUE: &str = "recipe_version_number_unique";
/// The name of the foreign key to the base version.
const VERSION_BASED_ON_SAME_RECIPE: &str = "recipe_version_based_on_same_recipe";
/// The name of the partial unique index on the approved version of a recipe.
const VERSION_ONE_APPROVED: &str = "recipe_version_one_approved";
/// The largest version number: the SQL type `INTEGER`.
const VERSION_NUMBER_MAX: u32 = i32::MAX.cast_unsigned();

impl VersionRow {
    /// Emulate the trigger `bump_row_version`: add 1 to `row_version` and set `updated_at`.
    pub(crate) fn bump(&mut self, now: Timestamp) -> Result<(), StorageError> {
        self.row_version = self
            .row_version
            .checked_add(1)
            .ok_or_else(|| corrupt_rows(&"the row version overflows"))?;
        self.updated_at = now;
        Ok(())
    }
}

impl MemoryTransaction {
    fn recipe_row_mut(&mut self, id: RecipeId) -> Result<&mut RecipeRow, StorageError> {
        self.state_mut()
            .recipes
            .get_mut(&id)
            .ok_or_else(|| missing_row(RECIPE_TABLE))
    }

    /// Change a version row in a legal transition and return the row after the change.
    ///
    /// The row must be in the state `from`. Otherwise the trigger `recipe_version_transition`
    /// rejects the update with `E-RCP-033`.
    fn change_version(
        &mut self,
        id: RecipeVersionId,
        from: VersionState,
        change: impl FnOnce(&mut VersionRow, Timestamp),
    ) -> Result<RowImage, StorageError> {
        let now = self.now();
        let row = self
            .state_mut()
            .versions
            .get_mut(&id)
            .ok_or_else(|| missing_row(VERSION_TABLE))?;
        if row.state().map_err(|reason| corrupt_rows(&reason))? != from {
            return Err(illegal_transition());
        }
        change(row, now);
        row.bump(now)?;
        Ok(version_image(row))
    }

    /// Add a version row after the checks of the constraints of `recipe_version`.
    fn put_version(&mut self, new: &NewVersion) -> Result<RowImage, StorageError> {
        let now = self.now();
        let state = self.state();
        let Some(recipe) = state.recipes.get(&new.recipe) else {
            return Err(missing_reference(VERSION_RECIPE_PLATFORM));
        };
        let siblings = || state.versions.values().filter(|v| v.recipe == new.recipe);
        if state.versions.contains_key(&new.version) {
            return Err(duplicate(VERSION_PKEY));
        }
        if new.number == 0 || new.number > VERSION_NUMBER_MAX {
            return Err(check_violation());
        }
        if siblings().any(|v| v.number == new.number) {
            return Err(duplicate(VERSION_NUMBER_UNIQUE));
        }
        let base_is_sibling = new
            .based_on
            .is_none_or(|base| siblings().any(|v| v.id == base));
        if !base_is_sibling {
            return Err(missing_reference(VERSION_BASED_ON_SAME_RECIPE));
        }
        if siblings().any(|v| v.state() == Ok(VersionState::LiveDraft)) {
            return Err(duplicate(RECIPE_VERSION_ONE_LIVE_DRAFT));
        }
        let row = VersionRow {
            id: new.version,
            recipe: new.recipe,
            platform: recipe.platform,
            number: new.number,
            status: VersionStatus::Draft,
            row_version: 1,
            based_on: new.based_on,
            change_note: new.change_note.clone(),
            content_sha256: None,
            created_by: new.creator,
            created_at: now,
            updated_at: now,
            approved_by: None,
            approved_at: None,
            retired_at: None,
            deleted_at: None,
        };
        let copies = new.based_on.map_or_else(Vec::new, |base| {
            state
                .parameters
                .values()
                .filter(|parameter| parameter.version == base)
                .map(|parameter| copy_parameter(parameter, new.version, now))
                .collect()
        });
        let image = version_image(&row);
        self.state_mut().versions.insert(row.id, row);
        for copy in copies {
            self.state_mut().parameters.insert(copy.id, copy);
        }
        Ok(image)
    }
}

/// Copy a parameter row into another version with a new identifier and the same key.
fn copy_parameter(from: &ParameterRow, version: RecipeVersionId, now: Timestamp) -> ParameterRow {
    let mut copy = from.clone();
    copy.id = ParameterId::from(Uuid::new_v4());
    copy.version = version;
    copy.created_at = now;
    for limit in copy.limits.values_mut() {
        limit.created_at = now;
    }
    for translation in copy.translations.values_mut() {
        translation.created_at = now;
    }
    copy
}

#[async_trait]
impl RecipeRepository for MemoryTransaction {
    async fn recipe_by_reference(
        &mut self,
        reference: &ReferenceNumber,
    ) -> Result<Option<Recipe>, StorageError> {
        let found = self
            .state()
            .recipes
            .values()
            .find(|row| row.reference == *reference)
            .map(|row| row.id);
        match found {
            Some(id) => self.state().try_recipe(id),
            None => Ok(None),
        }
    }

    async fn recipe_by_id(&mut self, id: RecipeId) -> Result<Option<Recipe>, StorageError> {
        self.state().try_recipe(id)
    }

    async fn version_parameters(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Vec<StoredParameter>, StorageError> {
        Ok(self.state().parameters_of(version))
    }

    async fn lock_recipe(&mut self, id: RecipeId) -> Result<Option<Locked<Recipe>>, StorageError> {
        let Some(row) = self.state().recipes.get(&id) else {
            return Ok(None);
        };
        let before = recipe_image(row);
        let Some(state) = self.state().try_recipe(id)? else {
            return Ok(None);
        };
        self.lock(RECIPE_TABLE, LockMode::ForUpdate, &id.to_string());
        Ok(Some(Locked { state, before }))
    }

    async fn lock_recipes_of_machine(
        &mut self,
        machine: MachineId,
    ) -> Result<Vec<Recipe>, StorageError> {
        // The recipes come from a `BTreeMap` keyed by id, so they are ordered by id.
        let ids: Vec<RecipeId> = self
            .state()
            .recipes
            .values()
            .filter(|row| row.machine == machine)
            .map(|row| row.id)
            .collect();
        let mut recipes = Vec::new();
        for id in ids {
            if let Some(recipe) = self.state().try_recipe(id)? {
                recipes.push(recipe);
            }
        }
        for recipe in &recipes {
            self.lock(RECIPE_TABLE, LockMode::ForUpdate, &recipe.id().to_string());
        }
        Ok(recipes)
    }

    async fn lock_versions(
        &mut self,
        recipe: RecipeId,
    ) -> Result<Vec<(RecipeVersionId, RowImage)>, StorageError> {
        let locked: Vec<(RecipeVersionId, RowImage)> = self
            .state()
            .versions
            .values()
            .filter(|row| row.recipe == recipe)
            .map(|row| (row.id, version_image(row)))
            .collect();
        for (id, _) in &locked {
            self.lock(VERSION_TABLE, LockMode::ForUpdate, &id.to_string());
        }
        Ok(locked)
    }

    async fn lock_version(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<RowImage>, StorageError> {
        let Some(row) = self.state().versions.get(&version) else {
            return Ok(None);
        };
        let image = version_image(row);
        self.lock(VERSION_TABLE, LockMode::ForUpdate, &version.to_string());
        Ok(Some(image))
    }

    async fn lock_version_shared(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<SharedVersion>, StorageError> {
        let Some(row) = self.state().versions.get(&version) else {
            return Ok(None);
        };
        let shared = SharedVersion {
            recipe: row.recipe,
            number: row.number,
            state: row.state().map_err(|reason| corrupt_rows(&reason))?,
            content_sha256: row.content_sha256,
            approved_by: row.approved_by,
            approved_at: row.approved_at,
        };
        self.lock(VERSION_TABLE, LockMode::ForShare, &version.to_string());
        Ok(Some(shared))
    }

    async fn insert_recipe(
        &mut self,
        recipe: &Recipe,
        first: NewVersion,
    ) -> Result<InsertedRecipe, StorageError> {
        let now = self.now();
        let state = self.state();
        let Some(machine) = state.machines.get(&recipe.machine()) else {
            return Err(missing_reference(RECIPE_MACHINE_PLATFORM));
        };
        if state.recipes.contains_key(&recipe.id()) {
            return Err(duplicate(RECIPE_PKEY));
        }
        if state
            .recipes
            .values()
            .any(|row| row.reference == *recipe.reference())
        {
            return Err(duplicate(RECIPE_REFERENCE_UNIQUE));
        }
        let row = RecipeRow {
            id: recipe.id(),
            machine: recipe.machine(),
            platform: machine.platform,
            reference: recipe.reference().clone(),
            name: recipe.name().clone(),
            deleted_at: recipe.deleted_at(),
            created_by: first.creator,
            created_at: now,
            updated_at: now,
        };
        let image = recipe_image(&row);
        self.state_mut().recipes.insert(row.id, row);
        let version = self.put_version(&first)?;
        Ok(InsertedRecipe {
            recipe: image,
            version,
        })
    }

    async fn rename_recipe(&mut self, id: RecipeId, name: &Text) -> Result<RowImage, StorageError> {
        let now = self.now();
        let row = self.recipe_row_mut(id)?;
        row.name = name.clone();
        row.updated_at = now;
        Ok(recipe_image(row))
    }

    async fn mark_recipe_deleted(&mut self, id: RecipeId) -> Result<RowImage, StorageError> {
        let now = self.now();
        let row = self.recipe_row_mut(id)?;
        row.deleted_at = Some(now);
        row.updated_at = now;
        Ok(recipe_image(row))
    }

    async fn restore_recipe(&mut self, id: RecipeId) -> Result<RowImage, StorageError> {
        let now = self.now();
        let row = self.recipe_row_mut(id)?;
        row.deleted_at = None;
        row.updated_at = now;
        Ok(recipe_image(row))
    }

    async fn insert_version(&mut self, new: NewVersion) -> Result<RowImage, StorageError> {
        if let Some(fault) = self.insert_version_fault() {
            return Err(fault.into());
        }
        self.put_version(&new)
    }

    async fn discard_version(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<RowImage, StorageError> {
        self.change_version(version, VersionState::LiveDraft, |row, now| {
            row.deleted_at = Some(now);
        })
    }

    async fn retire_version(&mut self, version: RecipeVersionId) -> Result<RowImage, StorageError> {
        self.change_version(version, VersionState::Approved, |row, now| {
            row.status = VersionStatus::Retired;
            row.retired_at = Some(now);
        })
    }

    async fn approve_version(
        &mut self,
        version: RecipeVersionId,
        approver: UserId,
        hash: Sha256,
    ) -> Result<RowImage, StorageError> {
        let state = self.state();
        if let Some(row) = state.versions.get(&version) {
            let has_other_approved = state.versions.values().any(|other| {
                other.recipe == row.recipe
                    && other.id != version
                    && other.status == VersionStatus::Approved
            });
            if has_other_approved && row.state() == Ok(VersionState::LiveDraft) {
                return Err(duplicate(VERSION_ONE_APPROVED));
            }
        }
        self.change_version(version, VersionState::LiveDraft, |row, now| {
            row.status = VersionStatus::Approved;
            row.approved_by = Some(approver);
            row.approved_at = Some(now);
            row.content_sha256 = Some(hash);
        })
    }
}
