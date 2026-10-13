//! The use case `AddParameter` (`domain-model.md` section 5.2, design D10, D11).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{S7Address, TypeExpr, UnitPath};
use recipe_domain::recipe::{
    LimitList, NewParameter, Ordinal, ParameterId, ParameterKey, ParameterTag, ReferenceNumber,
};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use super::definition_steps::{lock_live_unit, map_tag_error, resolve_type};
use super::draft_session::{DraftOutput, DraftTarget, MachineLock, OpenDraft};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::AppError;
use crate::ports::NonEmpty;

/// The parsed request of `AddParameter`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddParameterInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// The path of the unit that holds the parameter.
    pub unit_path: UnitPath,
    /// The tag of the new parameter.
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
    /// The value as JSON text. The domain parses it against the type tree.
    pub value_json: String,
    /// The process limits of the parameter. The list can be empty.
    pub limits: LimitList,
}

/// The result of `AddParameter`.
///
/// The type solves one problem: the response needs the new row version for `HX-Trigger` and the
/// server-generated key for the URL of the new parameter (A40).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddParameterOutput {
    /// The changed version and its new row version.
    pub draft: DraftOutput,
    /// The key that the server generated for the new parameter.
    pub key: ParameterKey,
}

/// Adds one parameter to a live draft.
///
/// The use case solves one problem: a new parameter must reference a live unit and a type row
/// that exist under the locks, and the audit log must show the derived types that the command
/// created. It locks `machine FOR SHARE`, the recipe, the version, and the unit, then the
/// class-2 advisory keys of the type, and writes the parameter with its limit rows.
#[derive(Clone)]
pub struct AddParameter {
    ctx: AppContext,
}

impl UseCase for AddParameter {
    const OPERATION: Operation = Operation::AddParameter;
}

impl AddParameter {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Add the parameter.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `ParameterEdit`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `PlantError::UnitNotFound` (`E-PLT-020`) or `UnitArchived` (`E-PLT-009`) for the unit.
    /// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) or `TypeTooLarge` (`E-PLT-018`) for the type.
    /// Returns `RecipeError::DuplicateTag` (`E-RCP-014`) and the other domain errors of the draft.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: AddParameterInput,
    ) -> Result<AddParameterOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = DraftTarget {
            reference: &input.reference,
            number: input.number,
            expected_row_version: input.expected_row_version,
        };
        let mut open = OpenDraft::open(&self.ctx, target, MachineLock::Shared).await?;
        let unit = lock_live_unit(&mut open, &input.unit_path).await?;
        let resolved = resolve_type(&mut open, &input.type_expr, audit_context(user, meta)).await?;
        let key = ParameterKey::from(Uuid::new_v4());
        let id = ParameterId::from(Uuid::new_v4());
        let tag = input.tag.clone();
        let new = NewParameter {
            id,
            key,
            unit,
            tag: input.tag,
            name_en: input.name_en,
            description_en: input.description_en,
            plc_type: resolved.type_id,
            s7_address: input.s7_address,
            ordinal: input.ordinal,
            value_json: input.value_json,
            limits: input.limits,
        };
        open.draft.add_parameter(new, &resolved.trees)?;
        let mut open = open.touch(user).await?;
        let version = open.draft.version();
        let image = open
            .tx
            .drafts()
            .insert_parameter(version, open.draft.parameter(key)?)
            .await
            .map_err(|error| map_tag_error(error, &tag, input.number))?;
        let added = AuditEntry::row(
            RowAction::ParameterAdded,
            AuditTarget::parameter(open.recipe, id),
            RowChange::Create {
                after: image.into_value(),
            },
            audit_context(user, meta),
        );
        let draft = open
            .commit(&self.ctx, NonEmpty::with_last(resolved.created, added))
            .await?;
        Ok(AddParameterOutput { draft, key })
    }
}

#[cfg(test)]
mod tests;
