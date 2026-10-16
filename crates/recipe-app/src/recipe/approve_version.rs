//! The use case `ApproveVersion` (`domain-model.md` section 5.2, design D12).

mod prepare;
mod write;

use recipe_domain::recipe::{ApprovalOutcome, ReferenceNumber};

use self::prepare::{Prepared, prepare};
use self::write::approve_in_transaction;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;

/// The parsed request of `ApproveVersion`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApproveVersionInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// When `true`, the approval accepts parameters of an outdated type revision (A29).
    pub allow_outdated: bool,
}

/// The result of `ApproveVersion` (ADR-0049 (e)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApproveVersionOutput {
    /// The decision of the domain method, or `AlreadyApproved` for a replay.
    pub outcome: ApprovalOutcome,
    /// The number of the version that this call retired. It is `None` when this call retired
    /// no version, also for a replay.
    pub retired: Option<u32>,
}

/// Approves the live draft of a recipe and retires the previous approved version.
///
/// The use case solves one problem: the content hash and the snapshot check take much CPU time,
/// but the approval must run under the locks of the recipe and the version. It checks and hashes
/// the draft with `Blocking` before the transaction. Then it locks the root and the version and
/// runs the domain method on the locked state (design D10, D12, ADR-0029).
#[derive(Clone)]
pub struct ApproveVersion {
    ctx: AppContext,
}

impl UseCase for ApproveVersion {
    const OPERATION: Operation = Operation::ApproveVersion;
}

impl ApproveVersion {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Approve the version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionApprove`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotDraft` (`E-RCP-007`) when the version is not a live draft.
    /// Returns the errors of `Draft::check_for_approval` (`E-RCP-008`, `E-RCP-016`, `E-RCP-020`,
    /// `E-RCP-022`, `E-RCP-023`, `E-RCP-024`).
    /// Returns `AppError::Timeout` (`E-WEB-009`) or `AppError::TaskFailed` (`E-WEB-011`)
    /// when the check does not complete.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ApproveVersionInput,
    ) -> Result<ApproveVersionOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let ready = match prepare(&self.ctx, &input).await? {
            Prepared::Done(outcome) => {
                return Ok(ApproveVersionOutput {
                    outcome,
                    retired: None,
                });
            }
            Prepared::Ready(ready) => ready,
        };
        approve_in_transaction(&self.ctx, (user, meta), &input, ready).await
    }
}

#[cfg(test)]
mod tests;
