//! Reads of the recipe tables from the in-memory state.

use recipe_domain::identity::UserId;
use recipe_domain::recipe::{
    Recipe, RecipeId, RecipeVersionId, StoredParameter, StoredRecipe, StoredVersionHeader,
};

use super::rows::ParameterRow;
use crate::error::StorageError;
use crate::memory::MemoryState;
use crate::memory::rows::corrupt_rows;

impl MemoryState {
    /// Build the recipe root from its rows, or `None` when the recipe does not exist.
    ///
    /// A failure means that the rows break a rule of the domain.
    pub(crate) fn try_recipe(&self, id: RecipeId) -> Result<Option<Recipe>, StorageError> {
        let Some(row) = self.recipes.get(&id) else {
            return Ok(None);
        };
        let mut versions = Vec::new();
        for version in self.versions.values().filter(|v| v.recipe == id) {
            versions.push(StoredVersionHeader {
                id: version.id,
                number: version.number,
                state: version.state().map_err(|reason| corrupt_rows(&reason))?,
                row_version: version.row_version,
            });
        }
        Recipe::try_from(StoredRecipe {
            id: row.id,
            machine: row.machine,
            reference: row.reference.clone(),
            name: row.name.clone(),
            deleted_at: row.deleted_at,
            versions,
        })
        .map(Some)
        .map_err(|reason| corrupt_rows(&reason))
    }

    /// Return the recipe with `id`, deleted or not. Corrupt rows give `None`.
    #[must_use]
    pub fn recipe(&self, id: RecipeId) -> Option<Recipe> {
        self.try_recipe(id).ok().flatten()
    }

    /// Return the row version of the version with `id`.
    #[must_use]
    pub fn version_row_version(&self, id: RecipeVersionId) -> Option<i64> {
        self.versions.get(&id).map(|row| row.row_version)
    }

    /// Return the parameters of a version, ordered by tag, with their limits and translations.
    #[must_use]
    pub fn parameters_of(&self, version: RecipeVersionId) -> Vec<StoredParameter> {
        let mut rows: Vec<&ParameterRow> = self
            .parameters
            .values()
            .filter(|row| row.version == version)
            .collect();
        rows.sort_by(|a, b| a.tag.cmp(&b.tag));
        rows.into_iter().map(ParameterRow::stored).collect()
    }

    /// Return the users who changed the version, in the order of their identifiers.
    #[must_use]
    pub fn editors_of(&self, version: RecipeVersionId) -> Vec<UserId> {
        self.editors
            .keys()
            .filter(|(v, _)| *v == version)
            .map(|(_, user)| *user)
            .collect()
    }
}
