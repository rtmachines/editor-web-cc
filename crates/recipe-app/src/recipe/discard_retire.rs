//! The use cases `DiscardDraft` and `RetireVersion` (`domain-model.md` section 5.2, ADR-0028).

use recipe_domain::audit::RowAction;
use recipe_domain::recipe::{Recipe, RecipeVersionId, ReferenceNumber};

use super::resolve::{ResolvedVersion, resolve_version};
use crate::audit::version_entry;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, conflict};
use crate::ports::{Isolation, Locked, NonEmpty, RowImage, Transaction};

/// The parsed request of `DiscardDraft`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscardDraftInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
}

/// The parsed request of `RetireVersion`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetireVersionInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
}

/// The result of `DiscardDraft` and `RetireVersion`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionOutput {
    /// The identifier of the changed version.
    pub version: RecipeVersionId,
}

/// The state that the locks of a version command return.
struct LockedTarget {
    recipe: Recipe,
    before: RowImage,
}

/// Take `recipe FOR UPDATE` and then `recipe_version FOR UPDATE` (`data-model.md` section 6).
///
/// The function solves one problem: `DiscardDraft` and `RetireVersion` lock the same two rows
/// in the same order and need the same before image. A missing row gives `E-WEB-005`.
async fn lock_target(
    tx: &mut dyn Transaction,
    target: ResolvedVersion,
    (reference, number): (&ReferenceNumber, u32),
) -> Result<LockedTarget, AppError> {
    let Locked { state: recipe, .. } =
        tx.recipes()
            .lock_recipe(target.recipe)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::Recipe(reference.clone()),
            })?;
    let before = tx
        .recipes()
        .lock_version(target.version)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::version(reference, number),
        })?;
    Ok(LockedTarget { recipe, before })
}

/// Discards the live draft of a recipe.
///
/// The use case solves one problem: two users must not discard and edit a draft at the same time.
/// It locks the recipe and then the version `FOR UPDATE`, and the domain method compares the
/// row version on the locked state (design D10, ADR-0008).
#[derive(Clone)]
pub struct DiscardDraft {
    ctx: AppContext,
}

impl UseCase for DiscardDraft {
    const OPERATION: Operation = Operation::DiscardDraft;
}

impl DiscardDraft {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Discard the draft.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionDiscard`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `RecipeError::VersionNotDraft` (`E-RCP-007`) when the version is not a live draft.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: DiscardDraftInput,
    ) -> Result<VersionOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = resolve_version(&self.ctx, &input.reference, input.number).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let LockedTarget { mut recipe, before } =
            lock_target(tx.as_mut(), target, (&input.reference, input.number)).await?;
        let outcome = recipe.discard(input.number, input.expected_row_version)?;
        let version = conflict(outcome, input.reference, input.number)?;
        let after = tx.recipes().discard_version(version).await?;
        let entries = NonEmpty::new(version_entry(
            RowAction::DraftDiscarded,
            (target.recipe, version),
            (before, after.into_value()),
            (user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(VersionOutput { version })
    }
}

/// Retires an approved version without a replacement.
///
/// The use case solves one problem: two users must not retire and approve around the same version
/// at the same time. It locks the recipe and then the version `FOR UPDATE`, and the domain
/// method compares the row version on the locked state (design D10, ADR-0028).
#[derive(Clone)]
pub struct RetireVersion {
    ctx: AppContext,
}

impl UseCase for RetireVersion {
    const OPERATION: Operation = Operation::RetireVersion;
}

impl RetireVersion {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Retire the version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionRetire`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `RecipeError::VersionNotApproved` (`E-RCP-009`) when the version is not approved.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RetireVersionInput,
    ) -> Result<VersionOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = resolve_version(&self.ctx, &input.reference, input.number).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let LockedTarget { mut recipe, before } =
            lock_target(tx.as_mut(), target, (&input.reference, input.number)).await?;
        let outcome = recipe.retire(input.number, input.expected_row_version)?;
        let version = conflict(outcome, input.reference, input.number)?;
        let after = tx.recipes().retire_version(version).await?;
        let entries = NonEmpty::new(version_entry(
            RowAction::VersionRetired,
            (target.recipe, version),
            (before, after.into_value()),
            (user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(VersionOutput { version })
    }
}

#[cfg(test)]
mod tests;
