//! The use case `RebindTypeRevision` (`domain-model.md` section 5.2, design D14).

use std::collections::BTreeMap;

use recipe_domain::audit::DiffAction;
use recipe_domain::plant::{LeafPath, PlatformTypes, PlcTypeId, TypeTrees};
use recipe_domain::recipe::{
    MemberValue, ParameterKey, RebindPlan, RebindRequest, ReferenceNumber,
};

use self::new_types::{ensure_new_types, plan_mismatch};
use super::draft_session::{
    DraftOutput, DraftTarget, MachineLock, OpenDraft, parameter_diff_entry,
};
use crate::audit::rebind_diff;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::AppError;
use crate::plant::rebind_locks;
use crate::ports::NonEmpty;

mod new_types;

/// The parsed request of `RebindTypeRevision`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebindTypeRevisionInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
    /// The row version that the form sent.
    pub expected_row_version: i64,
    /// The struct revision that the parameters use now.
    pub from: PlcTypeId,
    /// The newer revision of the same struct.
    pub to: PlcTypeId,
    /// The values of the new leaves, by parameter and concrete leaf path, as JSON or form text.
    pub new_member_values: BTreeMap<(ParameterKey, LeafPath), MemberValue>,
}

/// Moves every parameter of a live draft from one struct revision to a newer one.
///
/// The use case solves one problem: after `DefineUdt` adds a revision, the parameters of a draft
/// keep the old one. The rebind maps the values and limits of all affected parameters in one
/// command, and a failure of one parameter changes none of them (ADR-0018, D14).
#[derive(Clone)]
pub struct RebindTypeRevision {
    ctx: AppContext,
}

impl UseCase for RebindTypeRevision {
    const OPERATION: Operation = Operation::RebindTypeRevision;
}

impl RebindTypeRevision {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Move the parameters to the newer revision.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `ParameterEdit`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns `RecipeError::RebindTargetInvalid` (`E-RCP-052`) for a target that the rules reject.
    /// Returns one list of `RebindNeedsValue` (`E-RCP-028`) and `RebindDropsLimit` (`E-RCP-029`).
    /// Returns `RecipeError::ValueShapeMismatch` (`E-RCP-005`) for a sent value that does not fit.
    /// Returns `PlantError::TypeTooLarge` (`E-PLT-018`) for a target tree above a size limit.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RebindTypeRevisionInput,
    ) -> Result<DraftOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let (mut open, plan) = self.plan(input).await?;
        let context = audit_context(user, meta);
        let new_types = ensure_new_types(&mut open, &plan, context).await?;
        let from = plan.from();
        let changes = open
            .draft
            .apply_rebind(plan, &new_types.types, &new_types.trees)?;
        let mut open = open.touch(user).await?;
        let version = open.draft.version();
        let mut entries = new_types.created;
        for change in &changes {
            let Some(after) = change.after() else {
                return Err(plan_mismatch(from));
            };
            let drafts = open.tx.drafts();
            drafts.update_definition(version, after).await?;
            drafts
                .set_value(version, after.key(), after.value())
                .await?;
            drafts
                .replace_limits(version, after.key(), after.limits())
                .await?;
            entries.push(parameter_diff_entry(
                DiffAction::TypeRevisionRebound,
                (open.recipe, after.id()),
                rebind_diff(change, &new_types.trees)?,
                (user, meta),
            ));
        }
        let entries = NonEmpty::try_from(entries).map_err(|_| plan_mismatch(from))?;
        open.commit(&self.ctx, entries).await
    }

    /// Lock the draft, plan the rebind, and take the key `ARRAY` when a moved parameter is an array.
    ///
    /// This is design D14, steps 1 to 3. The plan checks the target, maps the values and limits
    /// of every moved parameter, and changes nothing.
    async fn plan(
        &self,
        input: RebindTypeRevisionInput,
    ) -> Result<(OpenDraft, RebindPlan), AppError> {
        let target = DraftTarget {
            reference: &input.reference,
            number: input.number,
            expected_row_version: input.expected_row_version,
        };
        let mut open = OpenDraft::open(&self.ctx, target, MachineLock::Shared).await?;
        let machine = open.draft.machine();
        let rows = open.tx.types().type_rows(machine).await?;
        let types = PlatformTypes::collect(machine, open.draft.platform(), rows)?;
        // A target without a row stays out of the trees, so the domain reports `TargetNotFound`.
        let known_to = types.get(input.to).map(|_| input.to);
        let trees = TypeTrees::resolve(
            open.draft
                .parameters()
                .iter()
                .map(|parameter| parameter.plc_type())
                .chain([input.from])
                .chain(known_to),
            &types,
        );
        let request = RebindRequest {
            from: input.from,
            to: input.to,
            new_member_values: input.new_member_values,
        };
        let plan = open.draft.plan_rebind(&request, &trees)?;
        let keys = rebind_locks(&plan.array_shapes());
        if !keys.is_empty() {
            open.tx.types().lock_derived_types(machine, &keys).await?;
        }
        Ok((open, plan))
    }
}

#[cfg(test)]
pub(super) mod fixtures;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod texts_tests;
#[cfg(test)]
mod write_tests;
