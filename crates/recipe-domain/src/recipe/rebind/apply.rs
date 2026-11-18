//! The change of a draft by a rebind plan (design D13, step 9).

use std::collections::BTreeMap;

use super::{RebindPlan, RebindTargetReason};
use crate::error_codes::ErrorCode;
use crate::plant::{MemberName, PlcPlatform, PlcTypeId, RuleSet, TypeNode, TypeTree, TypeTrees};
use crate::recipe::draft::{check_definition, check_faults, stored_limits_under, stored_value};
use crate::recipe::{
    Draft, DraftError, Parameter, ParameterChange, ParameterKey, ProcessLimit, RecipeError,
    StoredDefect,
};

/// Return `true` when two trees have the same shape.
///
/// The leaves, the struct names, the struct revisions, the member names, and the array
/// dimensions must be equal. The identifiers of array types and struct types may differ, because
/// the plan holds placeholders for the new array types. The walk uses an explicit stack.
fn same_shape(planned: &TypeTree, real: &TypeTree) -> bool {
    if planned.platform() != real.platform() {
        return false;
    }
    let mut stack = vec![(planned.root(), real.root())];
    while let Some((left, right)) = stack.pop() {
        let Some(children) = matching_children(left, right) else {
            return false;
        };
        stack.extend(children);
    }
    true
}

/// Compare two nodes without their children. Return the pairs of children to compare next.
///
/// The function returns `None` when the two nodes differ.
fn matching_children<'a>(
    planned: &'a TypeNode,
    real: &'a TypeNode,
) -> Option<Vec<(&'a TypeNode, &'a TypeNode)>> {
    match (planned, real) {
        (left @ TypeNode::Leaf { .. }, right @ TypeNode::Leaf { .. }) => {
            (left == right).then(Vec::new)
        }
        (
            TypeNode::Struct {
                name: left_name,
                revision: left_revision,
                members: left_members,
                ..
            },
            TypeNode::Struct {
                name: right_name,
                revision: right_revision,
                members: right_members,
                ..
            },
        ) => {
            if left_name != right_name || left_revision != right_revision {
                return None;
            }
            paired_members(left_members, right_members)
        }
        (
            TypeNode::Array {
                dimensions: left_dimensions,
                element: left,
                ..
            },
            TypeNode::Array {
                dimensions: right_dimensions,
                element: right,
                ..
            },
        ) => (left_dimensions == right_dimensions).then(|| vec![(left.as_ref(), right.as_ref())]),
        _ => None,
    }
}

/// Pair the members of two structs by position. Return `None` when the lists differ in length
/// or in a member name.
fn paired_members<'a>(
    planned: &'a [(MemberName, TypeNode)],
    real: &'a [(MemberName, TypeNode)],
) -> Option<Vec<(&'a TypeNode, &'a TypeNode)>> {
    if planned.len() != real.len() {
        return None;
    }
    planned
        .iter()
        .zip(real)
        .map(|((left_member, left), (right_member, right))| {
            (left_member == right_member).then_some((left, right))
        })
        .collect()
}

/// Everything that `apply_rebind` needs to check one affected parameter.
struct Candidate<'a> {
    platform: PlcPlatform,
    key: ParameterKey,
    planned: &'a TypeTree,
    plan: &'a RebindPlan,
    new_types: &'a BTreeMap<ParameterKey, PlcTypeId>,
}

impl Draft {
    /// Change the draft by a complete plan (design D13, step 9).
    ///
    /// Each planned parameter must exist with the type that it had when the plan was made. The
    /// new type of a struct parameter is the target of the plan. The new type of an array
    /// parameter comes from `new_types`. The tree of each new type must have the shape of the
    /// planned tree. The method reads the planned value and the planned limits again against
    /// that tree. For each rebound parameter, the method runs the platform, width, alignment,
    /// data block, end, and standard-access rules (ADR-0178 (d)). Then it checks the size budgets
    /// and the S7 layout of the changed draft. The draft changes only after every check passes.
    /// The translations stay.
    ///
    /// The result holds one change for each parameter, in tag order.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform. Returns
    /// `E-RCP-052` with the reason `PlanMismatch` when the draft or a new tree does not match
    /// the plan. Returns `E-RCP-005` when a planned stored value does not match the new tree.
    /// Returns the code of a planned limit row that does not match the new tree, such as
    /// `E-RCP-019`. These errors keep their codes and are not `Stored`.
    ///
    /// Returns `Stored` with the key of the parameter when the planned value fails a kept limit,
    /// such as `E-RCP-004`. A rebound parameter that breaks the platform rule (`E-RCP-003` or
    /// `E-RCP-053`), the width rule (`E-RCP-054`), or the alignment rule (`E-RCP-023`) also gives
    /// `Stored` (ADR-0195 (m)). The application layer gives `E-DB-008` for a `Stored` result.
    ///
    /// Returns `E-RCP-057` when a rebound parameter has a data block number that is too high.
    /// Returns `E-RCP-056` when a rebound parameter ends after byte 65535 of its data block.
    /// Returns `E-RCP-058` when a rebound parameter has an array that a data block with standard
    /// access cannot hold. Returns `E-PLT-018` when the layout of a rebound parameter is too large.
    /// These errors keep their codes.
    ///
    /// Returns `Stored` with the key of the first parameter of the draft that breaks the platform,
    /// width, or alignment rule. It also returns `Stored` when a parameter layout is too large
    /// (`E-PLT-018`). This check covers every parameter of the draft (ADR-0195 (i)).
    /// Returns `E-RCP-021` when the changed draft is above a size budget. Returns `E-RCP-022` for
    /// the first overlap that holds a rebound parameter.
    pub fn apply_rebind(
        &mut self,
        plan: RebindPlan,
        new_types: &BTreeMap<ParameterKey, PlcTypeId>,
        trees: &TypeTrees,
    ) -> Result<Vec<ParameterChange>, DraftError> {
        self.check_trees_belong(trees, plan.from)?;
        let pairs = self.rebound_pairs(&plan, new_types, trees)?;
        let replacements = pairs.iter().map(|(_, new)| new.clone()).collect();
        self.replace_parameters(replacements, trees)?;
        Ok(pairs
            .into_iter()
            .map(|(old, new)| ParameterChange::new(Some(old), Some(new)))
            .collect())
    }

    /// Pair each planned parameter with its rebound form, in tag order.
    fn rebound_pairs(
        &self,
        plan: &RebindPlan,
        new_types: &BTreeMap<ParameterKey, PlcTypeId>,
        trees: &TypeTrees,
    ) -> Result<Vec<(Parameter, Parameter)>, DraftError> {
        let mut pairs = Vec::with_capacity(plan.parameters.len());
        for (key, planned) in &plan.parameters {
            let candidate = Candidate {
                platform: self.platform(),
                key: *key,
                planned,
                plan,
                new_types,
            };
            let old = self
                .parameter(*key)
                .ok()
                .filter(|old| plan.old_types.get(key) == Some(&old.plc_type()))
                .ok_or_else(|| plan_mismatch(plan, trees))?;
            pairs.push((old.clone(), rebound_parameter(old, &candidate, trees)?));
        }
        pairs.sort_by(|(left, _), (right, _)| left.tag().cmp(right.tag()));
        Ok(pairs)
    }
}

/// Build the error for a draft or a tree that does not match the plan.
fn plan_mismatch(plan: &RebindPlan, trees: &TypeTrees) -> DraftError {
    DraftError::from(RecipeError::RebindTargetInvalid {
        type_name: trees.label(plan.from),
        reason: RebindTargetReason::PlanMismatch,
    })
}

/// Return `true` for the errors that show a stored defect of the address of a parameter.
///
/// These are the platform rule, the width rule, and the alignment rule (ADR-0195 (m)). Every other
/// error of `check_definition` keeps its code.
fn is_stored_address_rule(error: &DraftError) -> bool {
    [
        ErrorCode::RCP_003,
        ErrorCode::RCP_053,
        ErrorCode::RCP_054,
        ErrorCode::RCP_023,
    ]
    .contains(&error.code())
}

/// Build the parameter after the rebind and check it against the real tree.
fn rebound_parameter(
    old: &Parameter,
    candidate: &Candidate<'_>,
    trees: &TypeTrees,
) -> Result<Parameter, DraftError> {
    let plan = candidate.plan;
    let mismatch = || plan_mismatch(plan, trees);
    let new_type = if matches!(candidate.planned.root(), TypeNode::Struct { .. }) {
        plan.to
    } else {
        *candidate
            .new_types
            .get(&candidate.key)
            .ok_or_else(mismatch)?
    };
    let real = trees
        .tree(new_type)
        .filter(|real| same_shape(candidate.planned, real))
        .ok_or_else(mismatch)?;
    let stored = plan.values.get(&candidate.key).ok_or_else(mismatch)?;
    let rows = plan.limits.get(&candidate.key).ok_or_else(mismatch)?;
    // ADR-0195 (l) applies in `plan_rebind`. These two checks repeat its rules on a tree of the
    // same shape. Only a plan that does not match the trees reaches an error here. They keep plain
    // codes.
    let value = stored_value(old.tag(), stored, real)?;
    let limits: Vec<ProcessLimit> = stored_limits_under(old.tag(), rows, real, RuleSet::Earlier)?;
    let earlier = (real, RuleSet::Earlier);
    let stored = |error: DraftError| {
        DraftError::Stored(Box::new(StoredDefect::of_parameter(candidate.key, error)))
    };
    // The planned value mixes request leaves and stored leaves. An I7 fault between it and a kept
    // limit gives `Stored` (ADR-0195 (l)).
    if let Some(fault) = check_faults(old.tag(), &value, &limits, earlier)
        .into_iter()
        .next()
    {
        return Err(stored(DraftError::from(fault)));
    }
    // One order for every parameter command: platform, data block, width, alignment, end, then
    // standard access (ADR-0178 (d)). The rule errors of the data block, the end, and the standard
    // access keep their codes (ADR-0195 (m)).
    check_definition(candidate.platform, old.tag(), old.s7_address(), real).map_err(|error| {
        if is_stored_address_rule(&error) {
            stored(error)
        } else {
            error
        }
    })?;
    let rows = limits.iter().map(ProcessLimit::to_stored).collect();
    Ok(old.rebound(new_type, value.to_stored(), rows))
}

#[cfg(test)]
mod stored_platform_tests;
#[cfg(test)]
mod tests;
