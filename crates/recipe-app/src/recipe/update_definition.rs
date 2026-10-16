//! The use case `UpdateParameterDefinition` (`domain-model.md` section 5.2, design D17).

use recipe_domain::audit::DiffAction;
use recipe_domain::plant::{S7Address, TypeExpr, UnitPath};
use recipe_domain::recipe::{
    DefinitionUpdate, LimitList, Ordinal, ParameterKey, ParameterTag, ReferenceNumber, Retype,
    ValueInput,
};
use recipe_domain::text::{LongText, Text};

use super::definition_steps::{lock_live_unit, map_tag_error, resolve_type};
use super::draft_session::{
    DraftOutput, DraftTarget, MachineLock, OpenDraft, parameter_diff_entry,
};
use crate::audit::definition_diff;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::AppError;
use crate::ports::NonEmpty;

/// A new value and new limits of an `UpdateParameterDefinition` request.
///
/// The type solves one problem: a type change needs both parts (D17), and a request without a
/// type change can carry both (ADR-0029). One optional struct keeps the two parts together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetypeInput {
    /// The new value as JSON text or as one text per leaf. The domain checks it against the tree.
    pub value: ValueInput,
    /// The new process limits. The list can be empty.
    pub limits: LimitList,
}

impl From<RetypeInput> for Retype {
    fn from(input: RetypeInput) -> Self {
        Self {
            value: input.value,
            limits: input.limits,
        }
    }
}

/// The parsed request of `UpdateParameterDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateParameterDefinitionInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// The key of the parameter in the URL.
    pub key: ParameterKey,
    /// The path of the unit that holds the parameter.
    pub unit_path: UnitPath,
    /// The new tag of the parameter.
    pub tag: ParameterTag,
    /// The English name.
    pub name_en: Text,
    /// The English description.
    pub description_en: LongText,
    /// The type of the parameter.
    pub type_expr: TypeExpr,
    /// The S7 address, or `None` when the parameter has none.
    pub s7_address: Option<S7Address>,
    /// The position of the parameter in its draft.
    pub ordinal: Ordinal,
    /// The new value and limits, or `None` to keep them. A type change requires them.
    pub retype: Option<RetypeInput>,
}

/// Changes the definition of one parameter of a live draft.
///
/// The use case solves one problem: a definition change must reference a live unit and a type
/// row that exist under the locks, and a type change must not keep an old value. It locks
/// `machine FOR SHARE`, the recipe, the version, and the unit, then the class-2 advisory keys of
/// the type, and writes the parameter and its limit rows (D17, ADR-0029).
#[derive(Clone)]
pub struct UpdateParameterDefinition {
    ctx: AppContext,
}

impl UseCase for UpdateParameterDefinition {
    const OPERATION: Operation = Operation::UpdateParameterDefinition;
}

impl UpdateParameterDefinition {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Change the definition.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `ParameterEdit`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `RecipeError::ParameterNotFound` (`E-RCP-051`) for an unknown key.
    /// Returns `PlantError::UnitNotFound` (`E-PLT-020`) or `UnitArchived` (`E-PLT-009`) for the unit.
    /// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) or `TypeTooLarge` (`E-PLT-018`) for the type.
    /// Returns `RecipeError::ValueShapeMismatch` (`E-RCP-005`) for a type change without a value.
    /// Returns `RecipeError::DuplicateTag` (`E-RCP-014`) and the other domain errors of the draft.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: UpdateParameterDefinitionInput,
    ) -> Result<DraftOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = DraftTarget {
            reference: &input.reference,
            number: input.number,
            expected_row_version: input.expected_row_version,
        };
        let mut open = OpenDraft::open(&self.ctx, target, MachineLock::Shared).await?;
        let parameter = open.parameter_id(input.key)?;
        let unit = lock_live_unit(&mut open, &input.unit_path).await?;
        let resolved = resolve_type(&mut open, &input.type_expr, audit_context(user, meta)).await?;
        let update = DefinitionUpdate {
            unit,
            tag: input.tag.clone(),
            name_en: input.name_en,
            description_en: input.description_en,
            plc_type: resolved.type_id,
            s7_address: input.s7_address,
            ordinal: input.ordinal,
        };
        let has_retype = input.retype.is_some();
        let retype = input.retype.map(Retype::from);
        let change = open
            .draft
            .set_definition(input.key, update, retype, &resolved.trees)?;
        let diff = definition_diff(&change, &resolved.trees)?;
        let mut open = open.touch(user).await?;
        let version = open.draft.version();
        let written = open.draft.parameter(input.key)?;
        let drafts = open.tx.drafts();
        drafts
            .update_definition(version, written)
            .await
            .map_err(|error| map_tag_error(error, &input.tag, input.number))?;
        if has_retype {
            drafts
                .set_value(version, input.key, written.value())
                .await?;
            drafts
                .replace_limits(version, input.key, written.limits())
                .await?;
        }
        let updated = parameter_diff_entry(
            DiffAction::ParameterDefinitionUpdated,
            (open.recipe, parameter),
            diff,
            (user, meta),
        );
        open.commit(&self.ctx, NonEmpty::with_last(resolved.created, updated))
            .await
    }
}

#[cfg(test)]
mod errors;
#[cfg(test)]
mod requests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod texts_tests;
