//! Port of the Draft aggregate (`data-model.md` section 6, ADR-0016, design D5).

use async_trait::async_trait;
use recipe_domain::identity::UserId;
use recipe_domain::recipe::{
    LangCode, Parameter, ParameterKey, RecipeVersionId, StoredDraft, StoredLimit, StoredValue,
    Translation,
};

use super::RowImage;
use crate::error::StorageError;

/// Repository of the Draft aggregate: the parameters of a version.
///
/// The port solves one problem: the eight draft commands need the same lock, the touch
/// statement, and narrow writes that never touch more rows than the command changes. A write
/// of a parameter row fails with `Constraint` and `E-RCP-031` when the version is not a live
/// draft. The statement names the parameter by `parameter_key` inside the version.
#[async_trait]
pub trait DraftRepository: Send {
    /// Take `recipe_version FOR UPDATE` and read the draft with its editors and parameters.
    ///
    /// The result holds a version in any state. `Draft::load` rejects a state that is not a
    /// live draft. A missing version gives `None` and takes no lock.
    async fn lock_draft(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<StoredDraft>, StorageError>;

    /// Read the draft with its editors and parameters without a lock (`ApproveVersion`).
    ///
    /// The result holds a version in any state. A missing version gives `None`. The caller
    /// reads before its transaction and never runs a command on the result (design D10).
    async fn read_draft(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<StoredDraft>, StorageError>;

    /// Run the touch statement and the `draft_editor` insert, and return the new `row_version`.
    ///
    /// The touch statement is `UPDATE recipe_version SET updated_at = now()`. Its trigger adds
    /// 1 to `row_version`. The insert has `ON CONFLICT DO NOTHING`, so an editor appears once.
    async fn touch_draft(
        &mut self,
        version: RecipeVersionId,
        editor: UserId,
    ) -> Result<i64, StorageError>;

    /// Run `INSERT parameter` and one `INSERT parameter_limit` for each limit (`AddParameter`).
    ///
    /// The result is the image of the parameter row with the arrays `limits` and `translations`.
    /// A known tag gives `Duplicate` with `parameter_tag_unique`.
    async fn insert_parameter(
        &mut self,
        version: RecipeVersionId,
        parameter: &Parameter,
    ) -> Result<RowImage, StorageError>;

    /// Run `UPDATE parameter` for the definition columns (`UpdateParameterDefinition`, `RebindTypeRevision`).
    ///
    /// The statement writes only the unit, the tag, the names, the type, the address, and the
    /// ordinal. It leaves the value, the limit rows, and the translation rows unchanged
    /// (`data-model.md` section 6, ADR-0029). A caller that changes the type calls
    /// [`DraftRepository::set_value`] and [`DraftRepository::replace_limits`] after it.
    /// The same method writes a rebound parameter, so the port has no separate method for the type.
    async fn update_definition(
        &mut self,
        version: RecipeVersionId,
        parameter: &Parameter,
    ) -> Result<(), StorageError>;

    /// Run `UPDATE parameter SET value` (`SetParameterValue`).
    ///
    /// The statement touches no limit row and no translation row.
    async fn set_value(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        value: &StoredValue,
    ) -> Result<(), StorageError>;

    /// Run `DELETE parameter_limit` and `INSERT parameter_limit` for the parameter (`ReplaceLimits`).
    ///
    /// The statements touch only the limit rows of this parameter.
    async fn replace_limits(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        limits: &[StoredLimit],
    ) -> Result<(), StorageError>;

    /// Run `INSERT parameter_translation ... ON CONFLICT DO UPDATE` (`UpsertTranslation`).
    async fn upsert_translation(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        translation: &Translation,
    ) -> Result<(), StorageError>;

    /// Run `DELETE parameter_translation` for one language (`RemoveTranslation`).
    ///
    /// The result is the image of the deleted translation row before the delete, as
    /// `DELETE ... RETURNING` gives it. It holds the keys `parameter_id`, `lang`, `name`, and
    /// `description`. A language without a row gives a storage error.
    async fn delete_translation(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
        lang: &LangCode,
    ) -> Result<RowImage, StorageError>;

    /// Run `DELETE parameter` with its limit rows and translation rows (`RemoveParameter`).
    ///
    /// The result is the image of the deleted parameter row before the delete, as
    /// `DELETE ... RETURNING` gives it. It holds the arrays `limits` and `translations`.
    async fn delete_parameter(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
    ) -> Result<RowImage, StorageError>;
}
