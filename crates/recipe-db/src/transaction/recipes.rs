//! `RecipeRepository` on the transaction. The reads and the locks call `recipe_rows`, `parameter_rows`, and
//! `version_rows` (task 9.5). The writes call `recipe_writes` and `version_writes` (task 9.6).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{
    InsertedRecipe, Locked, NewVersion, RecipeRepository, RowImage, SharedVersion,
};
use recipe_domain::export::Sha256;
use recipe_domain::identity::UserId;
use recipe_domain::plant::MachineId;
use recipe_domain::recipe::{Recipe, RecipeId, RecipeVersionId, ReferenceNumber, StoredParameter};
use recipe_domain::text::Text;

use super::{
    PgTransaction, parameter_rows, recipe_rows, recipe_writes, version_rows, version_writes,
};
use crate::error::without_lock;

#[async_trait]
impl RecipeRepository for PgTransaction {
    async fn recipe_by_reference(
        &mut self,
        reference: &ReferenceNumber,
    ) -> Result<Option<Recipe>, StorageError> {
        recipe_rows::by_reference(&mut self.tx, reference).await
    }

    async fn recipe_by_id(&mut self, id: RecipeId) -> Result<Option<Recipe>, StorageError> {
        recipe_rows::by_id(&mut self.tx, id).await
    }

    async fn version_parameters(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Vec<StoredParameter>, StorageError> {
        parameter_rows::of_version(&mut self.tx, version).await
    }

    async fn lock_recipe(&mut self, id: RecipeId) -> Result<Option<Locked<Recipe>>, StorageError> {
        recipe_rows::lock(&mut self.tx, id).await
    }

    async fn lock_recipes_of_machine(
        &mut self,
        machine: MachineId,
    ) -> Result<Vec<Recipe>, StorageError> {
        recipe_rows::lock_of_machine(&mut self.tx, machine).await
    }

    async fn lock_versions(
        &mut self,
        recipe: RecipeId,
    ) -> Result<Vec<(RecipeVersionId, RowImage)>, StorageError> {
        let locked = version_rows::lock_all(&mut self.tx, recipe).await?;
        let mut versions = Vec::with_capacity(locked.len());
        for row in locked {
            let version = row.version();
            self.record_row_version(version, row.row_version);
            versions.push((version, row.into_image()));
        }
        Ok(versions)
    }

    async fn lock_version(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<RowImage>, StorageError> {
        let Some(row) = version_rows::lock_one(&mut self.tx, version).await? else {
            return Ok(None);
        };
        self.record_row_version(row.version(), row.row_version);
        Ok(Some(row.into_image()))
    }

    async fn lock_version_shared(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<SharedVersion>, StorageError> {
        version_rows::lock_shared(&mut self.tx, version).await
    }

    async fn insert_recipe(
        &mut self,
        recipe: &Recipe,
        first: NewVersion,
    ) -> Result<InsertedRecipe, StorageError> {
        recipe_writes::insert(&mut self.tx, recipe, &first).await
    }

    async fn rename_recipe(&mut self, id: RecipeId, name: &Text) -> Result<RowImage, StorageError> {
        recipe_writes::rename(&mut self.tx, id, name).await
    }

    async fn mark_recipe_deleted(&mut self, id: RecipeId) -> Result<RowImage, StorageError> {
        recipe_writes::mark_deleted(&mut self.tx, id).await
    }

    async fn restore_recipe(&mut self, id: RecipeId) -> Result<RowImage, StorageError> {
        recipe_writes::restore(&mut self.tx, id).await
    }

    async fn insert_version(&mut self, new: NewVersion) -> Result<RowImage, StorageError> {
        version_writes::insert(&mut self.tx, &new).await
    }

    async fn discard_version(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<RowImage, StorageError> {
        let locked = self.guard(version)?;
        let written = version_writes::discard(&mut self.tx, version, locked).await?;
        Ok(self.keep(version, written))
    }

    async fn retire_version(&mut self, version: RecipeVersionId) -> Result<RowImage, StorageError> {
        let locked = self.guard(version)?;
        let written = version_writes::retire(&mut self.tx, version, locked).await?;
        Ok(self.keep(version, written))
    }

    async fn approve_version(
        &mut self,
        version: RecipeVersionId,
        approver: UserId,
        hash: Sha256,
    ) -> Result<RowImage, StorageError> {
        let locked = self.guard(version)?;
        let written =
            version_writes::approve(&mut self.tx, version, locked, (approver, hash)).await?;
        Ok(self.keep(version, written))
    }
}

impl PgTransaction {
    /// Return the `row_version` that a lock of this transaction read, for the guard of a status
    /// write. A version without a lock gives `E-DB-008` and no statement runs (ADR-0030).
    fn guard(&self, version: RecipeVersionId) -> Result<i64, StorageError> {
        self.recorded_row_version(version)
            .ok_or_else(|| without_lock(version))
    }

    /// Record the new `row_version` of a written version, so a second status write still matches.
    fn keep(&mut self, version: RecipeVersionId, written: version_writes::Written) -> RowImage {
        self.record_row_version(version, written.row_version);
        written.into_image()
    }
}
