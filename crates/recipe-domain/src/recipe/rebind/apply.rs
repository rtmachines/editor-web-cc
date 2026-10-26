//! The change of a draft by a rebind plan (design D13, step 9).

use std::collections::BTreeMap;

use super::{RebindPlan, RebindTargetReason};
use crate::plant::{MemberName, PlcTypeId, TypeNode, TypeTree, TypeTrees};
use crate::recipe::draft::{check_value, stored_limits, stored_value};
use crate::recipe::{
    Draft, DraftError, Parameter, ParameterChange, ParameterKey, ProcessLimit, RecipeError,
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
    /// that tree. Then it checks the size budgets and the S7 rules of the changed draft. The
    /// draft changes only after every check passes. The translations stay.
    ///
    /// The result holds one change for each parameter, in tag order.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform. Returns
    /// `E-RCP-052` with the reason `PlanMismatch` when the draft or a new tree does not match
    /// the plan. Returns `E-RCP-005`, `E-RCP-004`, or a limit error when the planned value or a
    /// planned limit fails against the new tree. Returns `E-RCP-021` when the changed draft is
    /// above a size budget.
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
    let value = stored_value(old.tag(), stored, real)?;
    let limits: Vec<ProcessLimit> = stored_limits(old.tag(), rows, real)?;
    check_value(old.tag(), &value, &limits, real)?;
    let rows = limits.iter().map(ProcessLimit::to_stored).collect();
    Ok(old.rebound(new_type, value.to_stored(), rows))
}

#[cfg(test)]
mod tests;
