//! The use case `SetParameterValue` (`domain-model.md` section 5.2, design D10).

use recipe_domain::audit::DiffAction;
use recipe_domain::recipe::{ParameterKey, ReferenceNumber, ValueInput};

use super::draft_session::{
    DraftOutput, DraftTarget, MachineLock, OpenDraft, parameter_diff_entry,
};
use crate::audit::value_diff;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;
use crate::ports::NonEmpty;

/// The parsed request of `SetParameterValue`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetParameterValueInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// The key of the parameter in the URL.
    pub key: ParameterKey,
    /// The new value as JSON text or as one text per leaf. The domain checks it against the tree.
    pub value: ValueInput,
}

/// Sets the value of one parameter of a live draft.
///
/// The use case solves one problem: a value change must keep the limits and the translations
/// of the parameter and must not race with another edit of the draft. It locks the recipe and
/// the version `FOR UPDATE`, compares the row version, and writes the value row only
/// (design D10, `data-model.md` section 6).
#[derive(Clone)]
pub struct SetParameterValue {
    ctx: AppContext,
}

impl UseCase for SetParameterValue {
    const OPERATION: Operation = Operation::SetParameterValue;
}

impl SetParameterValue {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Set the value.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `ParameterEdit`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `RecipeError::ParameterNotFound` (`E-RCP-051`) for an unknown key.
    /// Returns `E-RCP-005`, `E-RCP-004`, or `E-RCP-021` when the value or the draft breaks a rule.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: SetParameterValueInput,
    ) -> Result<DraftOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = DraftTarget {
            reference: &input.reference,
            number: input.number,
            expected_row_version: input.expected_row_version,
        };
        let mut open = OpenDraft::open(&self.ctx, target, MachineLock::None).await?;
        let parameter = open.parameter_id(input.key)?;
        let change = open.draft.set_value(input.key, &input.value, &open.trees)?;
        let diff = value_diff(&change, &open.trees)?;
        let mut open = open.touch(user).await?;
        let version = open.draft.version();
        let value = open.draft.parameter(input.key)?.value();
        open.tx
            .drafts()
            .set_value(version, input.key, value)
            .await?;
        let entry = parameter_diff_entry(
            DiffAction::ParameterValueSet,
            (open.recipe, parameter),
            diff,
            (user, meta),
        );
        open.commit(&self.ctx, NonEmpty::new(entry)).await
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod texts_tests;
