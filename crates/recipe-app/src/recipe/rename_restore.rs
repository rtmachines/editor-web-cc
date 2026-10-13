//! The use cases `RenameRecipe` and `RestoreRecipe` (`domain-model.md` section 5.2).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::PlantError;
use recipe_domain::recipe::{RecipeId, ReferenceNumber};
use recipe_domain::text::Text;

use super::resolve::read_recipe;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked, NonEmpty};

/// The parsed request of `RenameRecipe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameRecipeInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The new name of the recipe.
    pub name: Text,
}

/// The result of `RenameRecipe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenameRecipeOutput {
    /// The identifier of the renamed recipe.
    pub recipe: RecipeId,
}

/// Renames a recipe.
///
/// The use case solves one problem: a rename must not race with a delete of the same recipe.
/// It locks the recipe `FOR UPDATE` and runs the domain rule on the locked state (design D10).
#[derive(Clone)]
pub struct RenameRecipe {
    ctx: AppContext,
}

impl UseCase for RenameRecipe {
    const OPERATION: Operation = Operation::RenameRecipe;
}

impl RenameRecipe {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Rename the recipe.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `RecipeEdit`.
    /// Returns `NotFound` (`E-WEB-005`) when no recipe has the reference number.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted under the lock.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RenameRecipeInput,
    ) -> Result<RenameRecipeOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let recipe_id = read_recipe(&self.ctx, &input.reference).await?.id();
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let Locked {
            state: mut recipe,
            before,
        } = tx
            .recipes()
            .lock_recipe(recipe_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::Recipe(input.reference.clone()),
            })?;
        recipe.rename(input.name.clone())?;
        let after = tx.recipes().rename_recipe(recipe_id, &input.name).await?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::RecipeRenamed,
            AuditTarget::recipe(recipe_id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(RenameRecipeOutput { recipe: recipe_id })
    }
}

/// The parsed request of `RestoreRecipe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreRecipeInput {
    /// The reference number of the deleted recipe in the URL.
    pub reference: ReferenceNumber,
}

/// The result of `RestoreRecipe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestoreRecipeOutput {
    /// The identifier of the restored recipe.
    pub recipe: RecipeId,
}

/// Restores a deleted recipe.
///
/// The use case solves one problem: a recipe must not come back on a machine that a concurrent
/// command archives. It locks the machine of the recipe `FOR SHARE` and then the recipe
/// `FOR UPDATE` (ADR-0015). It requires the locked machine live (ADR-0021).
#[derive(Clone)]
pub struct RestoreRecipe {
    ctx: AppContext,
}

impl UseCase for RestoreRecipe {
    const OPERATION: Operation = Operation::RestoreRecipe;
}

impl RestoreRecipe {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Restore the recipe.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `RecipeRestore`.
    /// Returns `NotFound` (`E-WEB-005`) when no recipe has the reference number.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived under the lock.
    /// Returns `RecipeError::RecipeNotDeleted` (`E-RCP-025`) when the recipe is not deleted under the lock.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RestoreRecipeInput,
    ) -> Result<RestoreRecipeOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let read = read_recipe(&self.ctx, &input.reference).await?;
        let (recipe_id, machine_id) = (read.id(), read.machine());
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let Locked { state: machine, .. } = tx
            .plant()
            .lock_machine_shared(machine_id)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::MachineId(machine_id),
            })?;
        if !machine.is_live() {
            return Err(PlantError::MachineArchived {
                code: machine.code(),
            }
            .into());
        }
        let Locked {
            state: mut recipe,
            before,
        } = tx
            .recipes()
            .lock_recipe(recipe_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::Recipe(input.reference.clone()),
            })?;
        recipe.restore()?;
        let after = tx.recipes().restore_recipe(recipe_id).await?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::RecipeRestored,
            AuditTarget::recipe(recipe_id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(RestoreRecipeOutput { recipe: recipe_id })
    }
}

#[cfg(test)]
mod tests;
