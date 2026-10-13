//! Step 2 of design D10 for the commands that start from a reference number.

use recipe_domain::plant::MachineId;
use recipe_domain::recipe::{Recipe, RecipeId, RecipeVersionId, ReferenceNumber};

use crate::context::AppContext;
use crate::error::{AppError, Missing};
use crate::ports::Isolation;

/// The identifiers of a version that a URL names by reference number and version number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ResolvedVersion {
    /// The recipe of the version.
    pub(super) recipe: RecipeId,
    /// The machine of the recipe. A command that needs `machine FOR SHARE` reads it here.
    pub(super) machine: MachineId,
    /// The version.
    pub(super) version: RecipeVersionId,
}

/// Read the recipe with `reference`, deleted or not, in a `ReadOnly` transaction.
///
/// The function solves one problem: a URL names a recipe by its reference number, but a lock
/// needs its identifier. The transaction takes no lock and ends before the function returns.
/// The caller reads identifiers from the result. It never runs a domain method on this state
/// (design D10).
///
/// # Errors
///
/// Returns `AppError::NotFound` (`E-WEB-005`) when no recipe has the reference number.
/// Returns `AppError::Storage` when the read fails.
pub(super) async fn read_recipe(
    ctx: &AppContext,
    reference: &ReferenceNumber,
) -> Result<Recipe, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    let recipe = tx.recipes().recipe_by_reference(reference).await?;
    recipe.ok_or_else(|| AppError::NotFound {
        entity: Missing::Recipe(reference.clone()),
    })
}

/// Resolve the version `number` of the recipe with `reference` in a `ReadOnly` transaction.
///
/// The function solves one problem: the domain gives `E-RCP-007` or `E-RCP-009` for an unknown
/// version number, but a URL value that names nothing is `E-WEB-005` (`api.md` section 1).
///
/// # Errors
///
/// Returns `AppError::NotFound` (`E-WEB-005`) when the recipe or the version does not exist.
/// Returns `AppError::Storage` when the read fails.
pub(super) async fn resolve_version(
    ctx: &AppContext,
    reference: &ReferenceNumber,
    number: u32,
) -> Result<ResolvedVersion, AppError> {
    let recipe = read_recipe(ctx, reference).await?;
    let header = recipe.version(number).ok_or_else(|| AppError::NotFound {
        entity: Missing::Version {
            reference: reference.clone(),
            number,
        },
    })?;
    Ok(ResolvedVersion {
        recipe: recipe.id(),
        machine: recipe.machine(),
        version: header.id(),
    })
}
