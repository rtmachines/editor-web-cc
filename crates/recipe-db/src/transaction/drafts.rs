//! `DraftRepository` on the transaction. The reads and the touch call `draft_rows` (task 9.7).
//! The parameter writes call `draft_writes` (tasks 9.7 and 9.8).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{DraftRepository, RowImage};
use recipe_domain::identity::UserId;
use recipe_domain::recipe::{
    LangCode, Parameter, ParameterKey, RecipeVersionId, StoredDraft, StoredLimit, StoredValue,
    Translation,
};

use super::{PgTransaction, draft_rows, draft_writes};

#[async_trait]
impl DraftRepository for PgTransaction {
    /// Lock the version and record the `row_version` that the statement read (ADR-0030).
    async fn lock_draft(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<StoredDraft>, StorageError> {
        let draft = draft_rows::lock(&mut self.tx, version).await?;
        if let Some(draft) = &draft {
            self.record_row_version(version, draft.row_version);
        }
        Ok(draft)
    }

    async fn read_draft(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<StoredDraft>, StorageError> {
        draft_rows::read(&mut self.tx, version).await
    }

    /// Touch the version and record the new `row_version`, so a later status write of the same
    /// transaction still passes its guard (ADR-0030).
    async fn touch_draft(
        &mut self,
        version: RecipeVersionId,
        editor: UserId,
    ) -> Result<i64, StorageError> {
        let row_version = draft_rows::touch(&mut self.tx, version, editor).await?;
        self.record_row_version(version, row_version);
        Ok(row_version)
    }

    async fn insert_parameter(
        &mut self,
        version: RecipeVersionId,
        parameter: &Parameter,
    ) -> Result<RowImage, StorageError> {
        draft_writes::insert(&mut self.tx, version, parameter).await
    }

    async fn update_definition(
        &mut self,
        version: RecipeVersionId,
        parameter: &Parameter,
    ) -> Result<(), StorageError> {
        draft_writes::update_definition(&mut self.tx, version, parameter).await
    }

    async fn set_value(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        value: &StoredValue,
    ) -> Result<(), StorageError> {
        draft_writes::set_value(&mut self.tx, version, key, value).await
    }

    async fn replace_limits(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        limits: &[StoredLimit],
    ) -> Result<(), StorageError> {
        draft_writes::replace_limits(&mut self.tx, version, key, limits).await
    }

    async fn upsert_translation(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        translation: &Translation,
    ) -> Result<(), StorageError> {
        draft_writes::upsert_translation(&mut self.tx, version, key, translation).await
    }

    async fn delete_translation(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        lang: &LangCode,
    ) -> Result<RowImage, StorageError> {
        draft_writes::delete_translation(&mut self.tx, version, key, lang).await
    }

    async fn delete_parameter(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
    ) -> Result<RowImage, StorageError> {
        draft_writes::delete(&mut self.tx, version, key).await
    }
}
