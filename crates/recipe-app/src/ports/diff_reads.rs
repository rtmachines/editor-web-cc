//! Read port of the diff of two versions (ADR-0050 (f), design D4).

use async_trait::async_trait;
use recipe_domain::plant::UnitPath;
use recipe_domain::recipe::{
    Ordinal, ParameterKey, ParameterTag, RecipeVersionId, StoredParameter,
};

use crate::error::StorageError;

/// The change flags and the row counts of one key in the diff of two versions.
///
/// The type solves one problem: a diff page must find the changed parameters and the counts of
/// the summary without a load of the values. The flags tell the sections of the key. The order
/// fields come from the new side, or from the old side for a key only in the old version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeFlags {
    /// The key of the parameter.
    pub key: ParameterKey,
    /// The unit path of the parameter, also for an archived unit.
    pub unit_path: UnitPath,
    /// The ordinal of the parameter.
    pub ordinal: Ordinal,
    /// The tag of the parameter.
    pub tag: ParameterTag,
    /// A field of the definition differs.
    pub definition: bool,
    /// The value differs, or the type changed.
    pub value: bool,
    /// A limit row differs. A key on one side only, or a changed type, sets it when a limit row exists.
    pub limits: bool,
    /// A translation differs.
    pub translations: bool,
    /// The key is only in the old version.
    pub only_old: bool,
    /// The key is only in the new version.
    pub only_new: bool,
    /// The number of limit rows that the limits section shows for the key.
    ///
    /// A changed type counts every distinct leaf path of both sides. A key on one side only
    /// counts the limit rows of that side.
    pub limit_rows: u32,
    /// The number of changed fields of the definition.
    pub definition_rows: u32,
    /// The number of languages with a changed translation.
    pub translation_rows: u32,
}

impl ChangeFlags {
    /// Return `true` when the key is on both sides and no part differs.
    #[must_use]
    pub const fn is_unchanged(&self) -> bool {
        !(self.definition
            || self.value
            || self.limits
            || self.translations
            || self.only_old
            || self.only_new)
    }
}

/// The stored parameters of the keys of one page, one list for each side.
#[derive(Clone, Debug, Default)]
pub struct DiffSides {
    /// The parameters of the old version, in the order of the requested keys.
    pub old: Vec<StoredParameter>,
    /// The parameters of the new version, in the order of the requested keys.
    pub new: Vec<StoredParameter>,
}

/// Read port of the diff of two versions.
///
/// The port solves one problem: a version can hold 4 000 parameters with values of up to 1 MiB.
/// A diff page compares the versions in the database and loads only the pairs of one page
/// (ADR-0050 (e), (f)). A `RepeatableRead` transaction gives both methods one snapshot.
#[async_trait]
pub trait DiffReads: Send {
    /// Read one row for each key of the old and the new version, in the order of the table.
    ///
    /// The order is the unit path by its numbers, the ordinal, the tag, and the key. `old` is
    /// `None` for a first approval, and then every key is only in the new version.
    async fn change_flags(
        &mut self,
        old: Option<RecipeVersionId>,
        new: RecipeVersionId,
    ) -> Result<Vec<ChangeFlags>, StorageError>;

    /// Read the parameters of `keys` with their limits and translations on each side.
    ///
    /// A side of `None` reads nothing and gives an empty list. A key that a side does not hold
    /// gives no parameter on that side. The caller gives each key only once in `keys`. The
    /// result for a repeated key differs between the adapters.
    async fn pairs(
        &mut self,
        old: Option<RecipeVersionId>,
        new: Option<RecipeVersionId>,
        keys: &[ParameterKey],
    ) -> Result<DiffSides, StorageError>;
}
