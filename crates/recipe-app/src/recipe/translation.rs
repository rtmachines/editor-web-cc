//! The use cases `UpsertTranslation` and `RemoveTranslation` (`domain-model.md` section 5.2).
//!
//! `E-RCP-027` comes from `LangCode::parse` in `recipe-web` (`api.md` section 1). The input of
//! both use cases holds a parsed `LangCode`, so neither use case has an `E-RCP-027` path.

use recipe_domain::audit::{AuditEntry, AuditTarget, DiffAction, RowAction, RowChange};
use recipe_domain::recipe::{LangCode, ParameterKey, ReferenceNumber, Translation};

use super::draft_session::{
    DraftOutput, DraftTarget, MachineLock, OpenDraft, parameter_diff_entry,
};
use crate::audit::translation_diff;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, StorageError};
use crate::ports::NonEmpty;

/// The parsed request of `UpsertTranslation`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpsertTranslationInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// The key of the parameter in the URL.
    pub key: ParameterKey,
    /// The translation with its language, name, and description.
    pub translation: Translation,
}

/// The parsed request of `RemoveTranslation`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveTranslationInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// The key of the parameter in the URL.
    pub key: ParameterKey,
    /// The language of the translation to remove.
    pub lang: LangCode,
}

/// Adds the translation of a parameter or replaces the translation of the same language.
///
/// The use case solves one problem: a parameter has one translation for each language, and the
/// command must touch no other row of the parameter. It locks the recipe and the version
/// `FOR UPDATE`, compares the row version, and writes one translation row (design D10).
#[derive(Clone)]
pub struct UpsertTranslation {
    ctx: AppContext,
}

impl UseCase for UpsertTranslation {
    const OPERATION: Operation = Operation::UpsertTranslation;
}

impl UpsertTranslation {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Add or replace the translation.
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
        input: UpsertTranslationInput,
    ) -> Result<DraftOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = DraftTarget {
            reference: &input.reference,
            number: input.number,
            expected_row_version: input.expected_row_version,
        };
        let mut open = OpenDraft::open(&self.ctx, target, MachineLock::None).await?;
        let parameter = open.parameter_id(input.key)?;
        let lang = input.translation.lang.clone();
        let change = open
            .draft
            .upsert_translation(input.key, input.translation)?;
        let diff = translation_diff(&change);
        let mut open = open.touch(user).await?;
        let version = open.draft.version();
        let stored = open
            .draft
            .parameter(input.key)?
            .translations()
            .get(&lang)
            .ok_or_else(|| {
                StorageError::invariant("the draft lacks the translation that the domain stored")
            })?;
        open.tx
            .drafts()
            .upsert_translation(version, input.key, stored)
            .await?;
        let entry = parameter_diff_entry(
            DiffAction::TranslationUpserted,
            (open.recipe, parameter),
            diff,
            (user, meta),
        );
        open.commit(&self.ctx, NonEmpty::new(entry)).await
    }
}

/// Removes the translation of one language from a parameter.
///
/// The use case solves one problem: the audit entry of a removed translation must hold the
/// deleted row, and the command must touch no other row. It locks the recipe and the version
/// `FOR UPDATE`, compares the row version, and deletes one translation row (design D10).
#[derive(Clone)]
pub struct RemoveTranslation {
    ctx: AppContext,
}

impl UseCase for RemoveTranslation {
    const OPERATION: Operation = Operation::RemoveTranslation;
}

impl RemoveTranslation {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Remove the translation.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `ParameterEdit`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `RecipeError::ParameterNotFound` (`E-RCP-051`) for an unknown key.
    /// Returns `RecipeError::TranslationNotFound` (`E-RCP-050`) for a language without a translation.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RemoveTranslationInput,
    ) -> Result<DraftOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = DraftTarget {
            reference: &input.reference,
            number: input.number,
            expected_row_version: input.expected_row_version,
        };
        let mut open = OpenDraft::open(&self.ctx, target, MachineLock::None).await?;
        let parameter = open.parameter_id(input.key)?;
        open.draft.remove_translation(input.key, &input.lang)?;
        let mut open = open.touch(user).await?;
        let version = open.draft.version();
        let removed = open
            .tx
            .drafts()
            .delete_translation(version, input.key, &input.lang)
            .await?;
        let entry = AuditEntry::row(
            RowAction::TranslationRemoved,
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
