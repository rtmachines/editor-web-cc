//! The snapshot check of a version (design D12).

mod check;
#[cfg(test)]
pub(in crate::recipe) mod fixtures;
mod item;
#[cfg(test)]
mod tests;

pub(in crate::recipe) use check::check_snapshot;
pub use item::{SnapshotError, SnapshotItem};

use crate::plant::{LiveUnits, TypeTrees};
use crate::recipe::{Parameter, ParameterTag, Recipe, RecipeError, StoredParameter};

/// The parameters of any version, as storage returns them.
///
/// The type solves one problem: `CreateVersion` checks the base version before it copies the
/// parameters, and the base version has no draft. The parameters keep their stored form.
#[derive(Clone, Debug)]
pub struct VersionSnapshot {
    parameters: Vec<Parameter>,
}

impl From<Vec<StoredParameter>> for VersionSnapshot {
    /// Build a snapshot from the rows. The snapshot orders the parameters by tag.
    fn from(rows: Vec<StoredParameter>) -> Self {
        let mut parameters: Vec<Parameter> = rows.into_iter().map(Parameter::from).collect();
        parameters.sort_by(|a, b| a.tag().cmp(b.tag()));
        Self { parameters }
    }
}

impl VersionSnapshot {
    /// Return the parameters ordered by tag.
    ///
    /// The approval check of `CreateVersion` reads the parameters through this accessor.
    #[must_use]
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    /// Check the snapshot against the recipe, the units, and the type trees (I6, I19).
    ///
    /// The check collects every problem. The parameter items come first, in tag order. The items
    /// of the machine and of the recipe follow.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-020` with one item for each problem. The items carry `E-PLT-009`,
    /// `E-PLT-010`, `E-PLT-020`, `E-RCP-004`, `E-RCP-005`, `E-RCP-006`, `E-RCP-012`,
    /// `E-RCP-013`, `E-RCP-017`, `E-RCP-018`, `E-RCP-019`, or the `PlantError` of a failed
    /// resolution.
    pub fn check(
        &self,
        recipe: &Recipe,
        units: &LiveUnits,
        trees: &TypeTrees,
    ) -> Result<Vec<ParameterTag>, RecipeError> {
        check_snapshot(&self.parameters, Some(recipe), units, trees)
    }
}
