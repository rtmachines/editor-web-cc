//! The use case `DeleteRecipe` (`domain-model.md` section 5.2, ADR-0021, design D16).

use recipe_domain::audit::{AuditContext, AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::Role;
use recipe_domain::recipe::{RecipeDeleter, RecipeId, RecipeVersionId, ReferenceNumber};

use super::resolve::read_recipe;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{Isolation, Locked, NonEmpty, RowImage, Transaction};

/// The parsed request of `DeleteRecipe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteRecipeInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
}

/// The result of `DeleteRecipe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteRecipeOutput {
    /// The identifier of the deleted recipe.
    pub recipe: RecipeId,
}

/// Deletes a recipe and discards its live draft.
///
/// The use case solves one problem: an approval must not commit while the recipe is deleted.
/// It locks the recipe `FOR UPDATE` and then every version `FOR UPDATE` in `id` order. The
/// domain rule runs on the locked state, so an approval that commits after the read of step 2
/// gives `E-RCP-010` (design D10, ADR-0021).
#[derive(Clone)]
pub struct DeleteRecipe {
    ctx: AppContext,
}

impl UseCase for DeleteRecipe {
    const OPERATION: Operation = Operation::DeleteRecipe;
}

/// Map the role of the user to the role of the delete rule (design D16).
///
/// The function solves one problem: the domain knows no `Role`. An operator has no
/// `RecipeDelete`, and `authorize` stops it first. The map still returns `Forbidden` for it.
fn deleter_of(role: Role) -> Result<RecipeDeleter, AppError> {
    match role {
        Role::Administrator => Ok(RecipeDeleter::Administrator),
        Role::ProcessEngineer => Ok(RecipeDeleter::ProcessEngineer),
        Role::Operator => Err(AppError::Forbidden {
            operation: DeleteRecipe::OPERATION,
        }),
    }
}

/// Discard the live draft `draft` and build its audit entry.
///
/// The before image comes from the version locks of the transaction.
async fn discard_draft_entry(
    tx: &mut dyn Transaction,
    (recipe, draft): (RecipeId, RecipeVersionId),
    versions: Vec<(RecipeVersionId, RowImage)>,
    context: AuditContext,
) -> Result<AuditEntry, AppError> {
    let before = versions
        .into_iter()
        .find_map(|(id, image)| (id == draft).then_some(image))
        .ok_or_else(|| StorageError::invariant("the draft is not among the locked versions"))?;
    let after = tx.recipes().discard_version(draft).await?;
    Ok(AuditEntry::row(
        RowAction::DraftDiscarded,
        AuditTarget::recipe_version(recipe, draft),
        RowChange::Update {
            before: before.into_value(),
            after: after.into_value(),
        },
        context,
    ))
}

impl DeleteRecipe {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Delete the recipe.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `RecipeDelete`.
    /// Returns `NotFound` (`E-WEB-005`) when no recipe has the reference number.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted under the lock.
    /// Returns `RecipeError::RecipeHasApprovedVersion` (`E-RCP-010`) when a version is approved.
    /// Returns `RecipeError::RecipeHasRetiredVersion` (`E-RCP-026`) when a process engineer
    /// deletes a recipe with a retired version.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: DeleteRecipeInput,
    ) -> Result<DeleteRecipeOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let deleter = deleter_of(user.role())?;
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
        let versions = tx.recipes().lock_versions(recipe_id).await?;
        let discarded = recipe.delete(deleter, self.ctx.clock.now())?;
        let context = audit_context(user, meta);
        let draft_entry = match discarded {
            Some(draft) => Some(
                discard_draft_entry(tx.as_mut(), (recipe_id, draft), versions, context.clone())
                    .await?,
            ),
            None => None,
        };
        let after = tx.recipes().mark_recipe_deleted(recipe_id).await?;
        let deleted = AuditEntry::row(
            RowAction::RecipeDeleted,
            AuditTarget::recipe(recipe_id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            context,
        );
        let entries = NonEmpty::with_last(draft_entry.into_iter().collect(), deleted);
        self.ctx.uow.commit(tx, entries).await?;
        Ok(DeleteRecipeOutput { recipe: recipe_id })
    }
}

#[cfg(test)]
mod tests;
