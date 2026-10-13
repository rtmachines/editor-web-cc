//! The use case `RemoveParameter` (`domain-model.md` section 5.2, design D10).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::recipe::{ParameterKey, ReferenceNumber};

use super::draft_session::{DraftOutput, DraftTarget, MachineLock, OpenDraft};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::AppError;
use crate::ports::NonEmpty;

/// The parsed request of `RemoveParameter`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveParameterInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// The key of the parameter in the URL.
    pub key: ParameterKey,
}

/// Removes a parameter with its limits and its translations from a live draft.
///
/// The use case solves one problem: the audit entry of a removed parameter must hold the whole
/// deleted row with its child rows, because no separate entry exists for them (R20). It locks
/// the recipe and the version `FOR UPDATE`, compares the row version, and deletes the row.
/// The delete returns the row before the delete (design D9, D10).
#[derive(Clone)]
pub struct RemoveParameter {
    ctx: AppContext,
}

impl UseCase for RemoveParameter {
    const OPERATION: Operation = Operation::RemoveParameter;
}

impl RemoveParameter {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Remove the parameter.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `ParameterEdit`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `RecipeError::ParameterNotFound` (`E-RCP-051`) for an unknown key.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RemoveParameterInput,
    ) -> Result<DraftOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = DraftTarget {
            reference: &input.reference,
            number: input.number,
            expected_row_version: input.expected_row_version,
        };
        let mut open = OpenDraft::open(&self.ctx, target, MachineLock::None).await?;
        let parameter = open.parameter_id(input.key)?;
        open.draft.remove_parameter(input.key)?;
        let mut open = open.touch(user).await?;
        let version = open.draft.version();
        let removed = open
            .tx
            .drafts()
            .delete_parameter(version, input.key)
            .await?;
        let entry = AuditEntry::row(
            RowAction::ParameterRemoved,
            AuditTarget::parameter(open.recipe, parameter),
            RowChange::Delete {
                before: removed.into_value(),
            },
            audit_context(user, meta),
        );
        open.commit(&self.ctx, NonEmpty::new(entry)).await
    }
}

#[cfg(test)]
mod tests;
