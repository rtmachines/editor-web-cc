//! The use case `ReplaceLimits` (`domain-model.md` section 5.2, design D10).

use recipe_domain::audit::DiffAction;
use recipe_domain::recipe::{LimitList, ParameterKey, ReferenceNumber};

use super::draft_session::{
    DraftOutput, DraftTarget, MachineLock, OpenDraft, parameter_diff_entry,
};
use crate::audit::limit_diff;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;
use crate::ports::NonEmpty;

/// The parsed request of `ReplaceLimits`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceLimitsInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// The key of the parameter in the URL.
    pub key: ParameterKey,
    /// The new limit rows. An empty list removes every limit.
    pub limits: LimitList,
}

/// Replaces the limit rows of one parameter of a live draft.
///
/// The use case solves one problem: a new limit must hold for the current value, and the
/// command must not touch the value or the translations. It locks the recipe and the version
/// `FOR UPDATE`, compares the row version, and writes the limit rows only (design D10).
#[derive(Clone)]
pub struct ReplaceLimits {
    ctx: AppContext,
}

impl UseCase for ReplaceLimits {
    const OPERATION: Operation = Operation::ReplaceLimits;
}

impl ReplaceLimits {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Replace the limits.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `ParameterEdit`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `RecipeError::ParameterNotFound` (`E-RCP-051`) for an unknown key.
    /// Returns `E-RCP-004`, `E-RCP-006`, `E-RCP-017`, `E-RCP-018`, `E-RCP-019`, or `E-RCP-021`
    /// when the limits or the draft break a rule.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ReplaceLimitsInput,
    ) -> Result<DraftOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = DraftTarget {
            reference: &input.reference,
            number: input.number,
            expected_row_version: input.expected_row_version,
        };
        let mut open = OpenDraft::open(&self.ctx, target, MachineLock::None).await?;
        let parameter = open.parameter_id(input.key)?;
        let change = open
            .draft
            .replace_limits(input.key, &input.limits, &open.trees)?;
        let diff = limit_diff(&change);
        let mut open = open.touch(user).await?;
        let version = open.draft.version();
        let limits = open.draft.parameter(input.key)?.limits();
        open.tx
            .drafts()
            .replace_limits(version, input.key, limits)
            .await?;
        let entry = parameter_diff_entry(
            DiffAction::LimitsReplaced,
            (open.recipe, parameter),
            diff,
            (user, meta),
        );
        open.commit(&self.ctx, NonEmpty::new(entry)).await
    }
}

#[cfg(test)]
mod tests;
