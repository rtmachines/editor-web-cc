//! Port of the Recipe root (`data-model.md` section 6, ADR-0016, design D5).

use async_trait::async_trait;
use recipe_domain::export::Sha256;
use recipe_domain::identity::UserId;
use recipe_domain::plant::MachineId;
use recipe_domain::recipe::{
    Recipe, RecipeId, RecipeVersionId, ReferenceNumber, StoredParameter, VersionState,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;

use super::{Locked, RowImage};
use crate::error::StorageError;

/// The values of a version row that a command inserts.
///
/// The type solves one problem: `CreateRecipe` and `CreateVersion` insert a version row
/// with the same eight values, and a bare argument list would swap identifiers by mistake.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewVersion {
    /// The recipe of the new version.
    pub recipe: RecipeId,
    /// The identifier of the new version.
    pub version: RecipeVersionId,
    /// The number of the new version in its recipe.
    pub number: u32,
    /// The version whose parameters the insert copies, or `None` for an empty first version.
    pub based_on: Option<RecipeVersionId>,
    /// The change note of the version.
    pub change_note: LongText,
    /// The user who creates the version.
    pub creator: UserId,
}

/// The row images that `CreateRecipe` writes.
///
/// The type solves one problem: `CreateRecipe` writes one `RecipeCreated` entry and one
/// `VersionCreated` entry, and each entry needs its own image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertedRecipe {
    /// The image of the new recipe row.
    pub recipe: RowImage,
    /// The image of the new version row with the number 1.
    pub version: RowImage,
}

/// What `ExportVersion` reads under `recipe_version FOR SHARE`.
///
/// The type solves one problem: the export compares the hash of the rebuilt body with the
/// stored hash, and it needs the state of the version before it builds the body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedVersion {
    /// The recipe of the version.
    pub recipe: RecipeId,
    /// The number of the version in its recipe.
    pub number: u32,
    /// The state of the version.
    pub state: VersionState,
    /// The stored hash of the exported body, or `None` for a draft.
    pub content_sha256: Option<Sha256>,
    /// The user who approved the version, or `None` for a draft.
    pub approved_by: Option<UserId>,
    /// The instant of the approval, or `None` for a draft.
    pub approved_at: Option<Timestamp>,
}

/// Repository of the Recipe root: recipes and the headers of their versions.
///
/// The port solves one problem: the Recipe commands need the locks, the writes, and the row
/// images of `data-model.md` section 6 without a database in the use case. A lock method
/// takes the SQL lock in its name and returns the locked state with the row image before the
/// change. A read method takes no lock. A write method names its intent and returns the row
/// image after the change. Every write of a version row adds 1 to its `row_version`.
#[async_trait]
pub trait RecipeRepository: Send {
    /// Read a recipe, deleted or not, by its reference number without a lock.
    async fn recipe_by_reference(
        &mut self,
        reference: &ReferenceNumber,
    ) -> Result<Option<Recipe>, StorageError>;

    /// Read a recipe, deleted or not, by its identifier without a lock.
    ///
    /// A recipe command reads `recipe.machine_id` this way before its machine lock.
    async fn recipe_by_id(&mut self, id: RecipeId) -> Result<Option<Recipe>, StorageError>;

    /// Read the parameters of any version without a lock (`CreateVersion`, `ExportVersion`).
    ///
    /// The result is ordered by tag. Each parameter holds its limits and translations.
    async fn version_parameters(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Vec<StoredParameter>, StorageError>;

    /// Take `recipe FOR UPDATE` and return the recipe with the row image before the change.
    ///
    /// A missing recipe gives `None` and takes no lock.
    async fn lock_recipe(&mut self, id: RecipeId) -> Result<Option<Locked<Recipe>>, StorageError>;

    /// Take `recipe FOR UPDATE` on every recipe of `machine`, ordered by `id` (`ArchiveMachine`).
    ///
    /// The use case finds a live draft from the headers of the result.
    async fn lock_recipes_of_machine(
        &mut self,
        machine: MachineId,
    ) -> Result<Vec<Recipe>, StorageError>;

    /// Take `recipe_version FOR UPDATE` on every version of `recipe`, ordered by `id` (`DeleteRecipe`).
    ///
    /// Each entry holds the row image before the change.
    async fn lock_versions(
        &mut self,
        recipe: RecipeId,
    ) -> Result<Vec<(RecipeVersionId, RowImage)>, StorageError>;

    /// Take `recipe_version FOR UPDATE` and return the row image before the change.
    ///
    /// `DiscardDraft`, `RetireVersion`, and `ApproveVersion` use it. A missing version gives
    /// `None` and takes no lock.
    async fn lock_version(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<RowImage>, StorageError>;

    /// Take `recipe_version FOR SHARE` (`ExportVersion`).
    ///
    /// A missing version gives `None` and takes no lock.
    async fn lock_version_shared(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<SharedVersion>, StorageError>;

    /// Run `INSERT recipe` and `INSERT recipe_version` for version 1 (`CreateRecipe`).
    ///
    /// A known reference number gives `Duplicate` with `recipe_reference_unique`.
    async fn insert_recipe(
        &mut self,
        recipe: &Recipe,
        first: NewVersion,
    ) -> Result<InsertedRecipe, StorageError>;

    /// Run `UPDATE recipe SET name` (`RenameRecipe`) and return the row after the change.
    async fn rename_recipe(&mut self, id: RecipeId, name: &Text) -> Result<RowImage, StorageError>;

    /// Run `UPDATE recipe SET deleted_at = now()` (`DeleteRecipe`) and return the row after the change.
    async fn mark_recipe_deleted(&mut self, id: RecipeId) -> Result<RowImage, StorageError>;

    /// Run `UPDATE recipe SET deleted_at = NULL` (`RestoreRecipe`) and return the row after the change.
    async fn restore_recipe(&mut self, id: RecipeId) -> Result<RowImage, StorageError>;

    /// Run `INSERT recipe_version` and copy the parameters of `based_on` (`CreateVersion`).
    ///
    /// Each copied parameter has a new identifier and the same key. The limit rows and the
    /// translation rows follow it. A second live draft gives `Duplicate` with
    /// `recipe_version_one_live_draft`.
    async fn insert_version(&mut self, new: NewVersion) -> Result<RowImage, StorageError>;

    /// Run `UPDATE recipe_version SET deleted_at = now()` (`DiscardDraft`, `DeleteRecipe`).
    ///
    /// A version that is not a live draft gives `Constraint` with `E-RCP-033`.
    async fn discard_version(&mut self, version: RecipeVersionId)
    -> Result<RowImage, StorageError>;

    /// Run `UPDATE recipe_version SET status = 'retired'` (`RetireVersion`, `ApproveVersion`).
    ///
    /// A version that is not approved gives `Constraint` with `E-RCP-033`.
    async fn retire_version(&mut self, version: RecipeVersionId) -> Result<RowImage, StorageError>;

    /// Run `UPDATE recipe_version SET status = 'approved'` (`ApproveVersion`).
    ///
    /// The statement sets `approved_by`, `approved_at`, and `content_sha256`. A version that
    /// is not a live draft gives `Constraint` with `E-RCP-033`. A second approved version
    /// gives `Duplicate` with `recipe_version_one_approved`.
    async fn approve_version(
        &mut self,
        version: RecipeVersionId,
        approver: UserId,
        hash: Sha256,
    ) -> Result<RowImage, StorageError>;
}
