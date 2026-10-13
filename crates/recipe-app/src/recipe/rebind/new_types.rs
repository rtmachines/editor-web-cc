//! The new array types of a rebind (design D14, step 4).

use std::collections::BTreeMap;

use recipe_domain::audit::{AuditContext, AuditEntry};
use recipe_domain::plant::{ArrayDimensions, PlcTypeId, TypeNode, TypeTree, TypeTrees};
use recipe_domain::recipe::{
    ArrayShape, ParameterKey, RebindPlan, RebindTargetReason, RecipeError,
};

use crate::error::AppError;
use crate::plant::DerivedTypes;
use crate::recipe::draft_session::OpenDraft;

/// The types that `Draft::apply_rebind` needs, and the entries of the rows that the rebind created.
///
/// The type solves one problem: the apply step needs the new type of each moved parameter and
/// the trees of these types. The commit needs the `DerivedTypeCreated` entries.
pub(super) struct NewTypes {
    /// The new type of each moved parameter.
    pub(super) types: BTreeMap<ParameterKey, PlcTypeId>,
    /// The trees of the types of the draft, of the two revisions, and of the new array types.
    pub(super) trees: TypeTrees,
    /// The `DerivedTypeCreated` entries, in creation order.
    pub(super) created: Vec<AuditEntry>,
}

/// Create the array row of each shape of the plan and build the map of the new types.
///
/// The caller took the key `ARRAY` before this call. The function reads the type rows after
/// that lock, so a row that another command created is found and not created twice.
///
/// # Errors
///
/// Returns `PlantError::TypeTooLarge` (`E-PLT-018`) when a new array type is too large.
/// The shape comes from a valid tree (ADR-0029), so no request gives `E-PLT-013`.
/// Returns `AppError::Storage` on a database failure.
pub(super) async fn ensure_new_types(
    open: &mut OpenDraft,
    plan: &RebindPlan,
    context: AuditContext,
) -> Result<NewTypes, AppError> {
    let machine = open.draft.machine();
    let rows = open.tx.types().type_rows(machine).await?;
    let mut derived = DerivedTypes::new(machine, open.draft.platform(), rows, context);
    let mut ensured = Vec::new();
    for shape in plan.array_shapes() {
        let id = derived.ensure_shape(open.tx.types(), &shape).await?;
        ensured.push((shape, id));
    }
    let types = new_types_of(plan, &ensured)?;
    let platform_types = derived.platform_types()?;
    let trees = TypeTrees::resolve(
        open.draft
            .parameters()
            .iter()
            .map(|parameter| parameter.plc_type())
            .chain([plan.from(), plan.to()])
            .chain(ensured.iter().map(|(_, id)| *id)),
        &platform_types,
    );
    let (_, created) = derived.into_parts();
    Ok(NewTypes {
        types,
        trees,
        created,
    })
}

/// Give each moved parameter its new type.
///
/// A parameter of the struct gets the target of the plan. An array parameter gets the row of the
/// shape with the same dimensions, because the plan lists one shape for each distinct chain of
/// dimensions and the element of every shape is the target.
fn new_types_of(
    plan: &RebindPlan,
    ensured: &[(ArrayShape, PlcTypeId)],
) -> Result<BTreeMap<ParameterKey, PlcTypeId>, AppError> {
    plan.parameters()
        .iter()
        .map(|(key, tree)| {
            let chain = array_chain(tree);
            let id = if chain.is_empty() {
                Some(plan.to())
            } else {
                ensured
                    .iter()
                    .find(|(shape, _)| shape.dimensions().iter().eq(chain.iter().copied()))
                    .map(|(_, id)| *id)
            };
            id.map(|id| (*key, id))
                .ok_or_else(|| plan_mismatch(plan.from()))
        })
        .collect()
}

/// List the dimensions of the array levels of a planned tree, from the outer to the inner level.
fn array_chain(tree: &TypeTree) -> Vec<&ArrayDimensions> {
    let mut chain = Vec::new();
    let mut node = tree.root();
    while let TypeNode::Array {
        dimensions,
        element,
        ..
    } = node
    {
        chain.push(dimensions);
        node = element;
    }
    chain
}

/// The error for a plan that lists no shape for one of its parameters.
///
/// The function takes the source type of the plan, so the caller can keep the plan moved.
pub(super) fn plan_mismatch(from: PlcTypeId) -> AppError {
    RecipeError::RebindTargetInvalid {
        type_name: from.to_string(),
        reason: RebindTargetReason::PlanMismatch,
    }
    .into()
}
