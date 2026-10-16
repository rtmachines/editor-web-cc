//! Queries of Recipe Management (design D4 of the change `web-recipe-editor`).
//!
//! Each query is a struct over an `AppContext`. Its first statement checks the permission. Then it
//! reads in one transaction, and it ends that transaction before it resolves a type tree or
//! builds the result (`reliability.md` section 7.4). `GetVersion` reads in one `RepeatableRead`
//! transaction (ADR-0042). The other queries here use `ReadOnly`.

use recipe_domain::plant::Machine;
use recipe_domain::recipe::{Recipe, RecipeError, ReferenceNumber};

use crate::auth::{CurrentUser, Operation};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{PlantRepository, Transaction};

#[cfg(test)]
mod count_tests;
mod outdated;
mod parameter_form;
#[cfg(test)]
mod parameter_form_tests;
#[cfg(test)]
mod parameter_reads_tests;
mod parameters;
#[cfg(test)]
mod parameters_tests;
mod preview;
#[cfg(test)]
mod preview_tests;
mod recipes;
#[cfg(test)]
mod recipes_tests;
#[cfg(test)]
mod source_tests;
#[cfg(test)]
mod test_world;
#[cfg(test)]
mod version_tests;
mod versions;
#[cfg(test)]
mod versions_tests;

pub use outdated::{OutdatedParameter, StructRevision};
pub use parameter_form::{GetParameterForm, GetParameterFormInput, ParameterForm};
pub use parameters::{
    GetLimits, GetParameter, GetTranslations, LimitsView, ListParameters, ListParametersInput,
    ParameterList, ParameterRef, ParameterView, TranslationsView,
};
pub use recipes::{
    GetRecipe, GetRecipeForm, GetRecipeInput, ListDeletedRecipes, ListRecipes, RecipeForm,
    RecipeFormInput, RecipeList, RecipeListInput, RecipeView,
};
pub use versions::{
    GetVersion, GetVersionForm, GetVersionFormInput, GetVersionInput, LastChange, ListVersions,
    ListVersionsInput, VersionChoice, VersionForm, VersionList, VersionView,
};

/// Compute the offset and the limit of a page. The limit is one row more than the page.
///
/// The extra row tells whether a next page exists.
///
/// # Errors
///
/// Returns a `StorageError` when the offset has no product or the page size has no successor.
fn page_window(page: u32, size: u32) -> Result<(u64, u32), StorageError> {
    // The product of two `u32` values always fits `u64`, so `None` never happens.
    let offset = u64::from(page)
        .checked_mul(u64::from(size))
        .ok_or_else(|| StorageError::invariant("the page offset has no product"))?;
    let limit = size
        .checked_add(1)
        .ok_or_else(|| StorageError::invariant("the page size has no successor"))?;
    Ok((offset, limit))
}

/// Cut a window of `size + 1` rows to the page and tell whether a next page exists.
fn cut_page<T>(mut rows: Vec<T>, size: u32) -> (Vec<T>, bool) {
    let size = usize::try_from(size).unwrap_or(usize::MAX);
    let has_next = rows.len() > size;
    rows.truncate(size);
    (rows, has_next)
}

/// Read the recipe with `reference`, deleted or not, or return `NotFound` (`E-WEB-005`).
async fn read_recipe(
    tx: &mut dyn Transaction,
    reference: &ReferenceNumber,
) -> Result<Recipe, AppError> {
    let found = tx.recipes().recipe_by_reference(reference).await?;
    found.ok_or_else(|| AppError::NotFound {
        entity: Missing::Recipe(reference.clone()),
    })
}

/// Refuse a deleted recipe to every role that cannot restore it (ADR-0043 (e)).
///
/// The rule solves one problem: a deleted recipe stays out of view for the roles that cannot
/// restore it (ADR-0021). The role that holds the permission of `RestoreRecipe` may read it.
///
/// # Errors
///
/// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted and the role
/// cannot restore it.
fn require_visible(user: &CurrentUser, recipe: &Recipe) -> Result<(), AppError> {
    if recipe.is_deleted() && !Operation::RestoreRecipe.requirement().allows(user.role()) {
        return Err(AppError::Recipe(RecipeError::RecipeDeleted {
            reference: recipe.reference().clone(),
        }));
    }
    Ok(())
}

/// Read the machine of `recipe`, live or archived.
///
/// A recipe always has a machine (foreign key), so a missing machine is a defect of the storage.
async fn read_machine(
    plant: &mut dyn PlantRepository,
    recipe: &Recipe,
) -> Result<Machine, AppError> {
    let found = plant.machine_by_id(recipe.machine()).await?;
    found.ok_or_else(|| StorageError::invariant("the machine of a recipe does not exist").into())
}
