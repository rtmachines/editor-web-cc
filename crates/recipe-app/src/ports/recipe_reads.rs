//! Read port of the recipe pages (design D5, ADR-0042, ADR-0043).

use async_trait::async_trait;
use recipe_domain::identity::Login;
use recipe_domain::plant::{MachineCode, PlcTypeId, S7Address, UnitId, UnitPath};
use recipe_domain::recipe::{
    Ordinal, ParameterKey, ParameterTag, RecipeId, RecipeVersionId, ReferenceNumber,
    StoredParameter, VersionState,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;

use crate::error::StorageError;

/// The filter of `RecipeReads::list_recipes`.
///
/// The type solves one problem: the recipe list has four independent choices, and a struct names
/// each choice at the call site.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecipeFilter {
    /// A text that the reference number or the name must contain, with no difference of case.
    /// `None` keeps every recipe.
    pub text: Option<Text>,
    /// The code of the live machine of the recipes. An archived machine with the same code keeps
    /// none of its recipes (A32). `None` keeps every machine.
    pub machine: Option<MachineCode>,
    /// A state that at least one version of the recipe must have. `None` keeps every recipe.
    pub status: Option<VersionState>,
    /// Whether the list also holds the deleted recipes.
    pub include_deleted: bool,
}

/// One row of the list of recipes.
///
/// The type solves one problem: the list page needs six columns of a recipe and not the whole
/// root with its version headers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeSummary {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The name of the recipe.
    pub name: Text,
    /// The code of the machine of the recipe.
    pub machine_code: MachineCode,
    /// Whether the recipe is deleted.
    pub deleted: bool,
    /// The number of the approved version, or `None` when the recipe has none.
    pub approved: Option<u32>,
    /// The number of the live draft, or `None` when the recipe has none.
    pub draft: Option<u32>,
}

/// One row of the list of versions of a recipe.
///
/// The type solves one problem: the version table shows six columns and needs no parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionSummary {
    /// The number of the version in its recipe.
    pub number: u32,
    /// The state of the version.
    pub state: VersionState,
    /// The change note of the version.
    pub change_note: LongText,
    /// The instant when the version was created.
    pub created_at: Timestamp,
    /// The instant of the approval, or `None` for a version that was never approved.
    pub approved_at: Option<Timestamp>,
}

/// The header of one version with the logins of its actors.
///
/// The type solves one problem: the version page shows the actors by login, and a page must not
/// join the user table by itself. `row_version` and `updated_at` come from the same row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionDetail {
    /// The identifier of the version.
    pub id: RecipeVersionId,
    /// The recipe of the version.
    pub recipe: RecipeId,
    /// The number of the version in its recipe.
    pub number: u32,
    /// The state of the version.
    pub state: VersionState,
    /// The change note of the version.
    pub change_note: LongText,
    /// The login of the user who created the version.
    pub created_by: Login,
    /// The login of the user who approved the version, or `None` for a version never approved.
    pub approved_by: Option<Login>,
    /// The instant when the version was created.
    pub created_at: Timestamp,
    /// The instant of the approval, or `None` for a version that was never approved.
    pub approved_at: Option<Timestamp>,
    /// The instant of the last change of the version row.
    pub updated_at: Timestamp,
    /// The `row_version` of the version row (ADR-0008).
    pub row_version: i64,
}

/// One row of the parameter table of a version.
///
/// The type solves one problem: the table shows the columns of a parameter and not its value, its
/// limit rows, and its translation rows. A value can have 1 MiB. `parameter_by_key` loads them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterRow {
    /// The key of the parameter in a URL.
    pub key: ParameterKey,
    /// The unit that holds the parameter.
    pub unit: UnitId,
    /// The path of that unit.
    pub unit_path: UnitPath,
    /// The tag of the parameter.
    pub tag: ParameterTag,
    /// The English name of the parameter.
    pub name_en: Text,
    /// The position of the parameter among the parameters of its unit.
    pub ordinal: Ordinal,
    /// The type of the parameter.
    pub plc_type: PlcTypeId,
    /// The S7 address of the parameter, or `None` on a platform without addresses.
    pub s7_address: Option<S7Address>,
}

/// The count of the parameters of one unit in a version.
///
/// The type solves one problem: a draft with many parameters shows one unit per page, and the
/// page links to every unit with its count (`api.md` section 2.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitCount {
    /// The unit that holds the parameters.
    pub unit: UnitId,
    /// The path of that unit.
    pub path: UnitPath,
    /// The number of parameters of the version in that unit.
    pub count: u32,
}

/// Read port of the recipe pages.
///
/// The port solves one problem: the query use cases of the recipe pages read lists, headers, and
/// parameter rows without a lock and without the whole aggregate. Each method runs a fixed number
/// of statements that does not grow with the count of rows. A `RepeatableRead` transaction gives
/// all methods of one query the same snapshot (ADR-0042).
#[async_trait]
pub trait RecipeReads: Send {
    /// Read one page of the recipes without a lock (`/recipes`).
    ///
    /// A row matches `filter.text` when the reference number or the name contains the text with
    /// no difference of case. A row matches `filter.machine` when its machine is live and has that
    /// code. A row matches `filter.status` when one of its versions has that
    /// state. A deleted recipe matches only with `filter.include_deleted`. The rows come in the
    /// byte order of the reference number. The page starts after `offset` rows and holds at
    /// most `limit` rows.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Other` (`E-DB-008`) when `offset` is above `i64::MAX`.
    async fn list_recipes(
        &mut self,
        filter: &RecipeFilter,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<RecipeSummary>, StorageError>;

    /// Read one page of the versions of `recipe`, the newest number first.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Other` (`E-DB-008`) when `offset` is above `i64::MAX`.
    async fn version_page(
        &mut self,
        recipe: RecipeId,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<VersionSummary>, StorageError>;

    /// Read the header of `version` with the logins of its actors.
    ///
    /// A missing version gives `None`.
    async fn version_detail(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<VersionDetail>, StorageError>;

    /// Read the login of the actor of the last change of `recipe` at the instant `at`.
    ///
    /// `at` is `updated_at` of the version row (ADR-0043 (m)). The method matches `at` with the
    /// commit time of the audit entries of the recipe. A time without an entry, or an entry
    /// without an actor, gives `None`. Two entries at one instant give the entry that was written last.
    async fn last_change(
        &mut self,
        recipe: RecipeId,
        at: Timestamp,
    ) -> Result<Option<Login>, StorageError>;

    /// Read one page of the parameters of `version`.
    ///
    /// `unit` keeps the parameters of one unit. The rows come in the order of the unit path by
    /// its numbers (`U1.2` before `U1.10`), the ordinal, and the tag. The page starts after `offset` rows and holds at most
    /// `limit` rows.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Other` (`E-DB-008`) when `offset` is above `i64::MAX`.
    async fn parameter_page(
        &mut self,
        version: RecipeVersionId,
        unit: Option<UnitId>,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<ParameterRow>, StorageError>;

    /// Count the parameters of `version` in each unit that holds one.
    ///
    /// The rows come in the order of the unit path by its numbers (`U1.2` before `U1.10`).
    async fn unit_counts(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Vec<UnitCount>, StorageError>;

    /// Read the parameter with `key` in `version`, with its limits and its translations.
    ///
    /// A missing parameter gives `None`.
    async fn parameter_by_key(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
    ) -> Result<Option<StoredParameter>, StorageError>;
}
