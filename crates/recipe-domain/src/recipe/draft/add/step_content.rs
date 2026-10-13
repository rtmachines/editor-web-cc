//! The limits, the value, and the budgets of a new parameter (design D6, steps 2 and 3).

use crate::plant::{LeafPath, PlantError, TypeTree, TypeTrees};
use crate::recipe::draft::{BudgetCounts, Draft, check_budgets};
use crate::recipe::{
    DraftError, LimitFault, LimitList, ParameterTag, ParameterValue, ProcessLimit, RecipeError,
    StoredLimit, StoredValue, ValueFault,
};

/// The text of `E-RCP-005` when the value belongs to another root type than the tree.
const WRONG_TREE_EXPECTED: &str = "a value of the type of the parameter";

/// Parse the limits and the value of a new parameter and check the value against the limits.
///
/// The order is the limits in the order of the request. Then the shape of the value. Then the
/// value against the limits (design D6, step 2).
pub(in crate::recipe::draft) fn parse_content(
    tag: &ParameterTag,
    value_json: &str,
    rows: &LimitList,
    tree: &TypeTree,
) -> Result<(ParameterValue, Vec<ProcessLimit>), RecipeError> {
    let limits = parse_limits(tag, rows, tree)?;
    let value = parse_value(tag, value_json, &limits, tree)?;
    Ok((value, limits))
}

/// Parse the limit rows of a request against the type tree, in the order of the request.
pub(in crate::recipe::draft) fn parse_limits(
    tag: &ParameterTag,
    rows: &LimitList,
    tree: &TypeTree,
) -> Result<Vec<ProcessLimit>, RecipeError> {
    rows.rows()
        .iter()
        .map(|row| ProcessLimit::parse(row, tree))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|fault| limit_error(tag, fault))
}

/// Parse the value of a request and check it against the limits (design D6, step 2).
pub(in crate::recipe::draft) fn parse_value(
    tag: &ParameterTag,
    value_json: &str,
    limits: &[ProcessLimit],
    tree: &TypeTree,
) -> Result<ParameterValue, RecipeError> {
    let value = ParameterValue::parse(value_json, tree).map_err(|fault| value_error(tag, fault))?;
    check_value(tag, &value, limits, tree)?;
    Ok(value)
}

/// Check a parsed value against the limits.
pub(in crate::recipe) fn check_value(
    tag: &ParameterTag,
    value: &ParameterValue,
    limits: &[ProcessLimit],
    tree: &TypeTree,
) -> Result<(), RecipeError> {
    value
        .check(tree, limits)
        .map_err(|fault| value_error(tag, fault))
}

/// Check the stored limit rows of a parameter against its type tree.
///
/// A row that the tree does not accept maps as in a request.
pub(in crate::recipe) fn stored_limits(
    tag: &ParameterTag,
    rows: &[StoredLimit],
    tree: &TypeTree,
) -> Result<Vec<ProcessLimit>, RecipeError> {
    rows.iter()
        .map(|row| ProcessLimit::try_from((row, tree)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|fault| limit_error(tag, fault))
}

/// Check the stored value of a parameter against its type tree.
///
/// A fault of the stored form gives `E-RCP-005`.
pub(in crate::recipe) fn stored_value(
    tag: &ParameterTag,
    value: &StoredValue,
    tree: &TypeTree,
) -> Result<ParameterValue, RecipeError> {
    ParameterValue::try_from((value, tree)).map_err(|fault| value_error(tag, fault))
}

/// Wrap the fault of a limit row with the tag of the parameter.
fn limit_error(tag: &ParameterTag, fault: LimitFault) -> RecipeError {
    let tag = tag.clone();
    match fault {
        LimitFault::NoLeaf { leaf } => RecipeError::LeafPathNotFound { tag, leaf },
        LimitFault::NotApplicable { leaf } => RecipeError::LimitNotApplicable { tag, leaf },
        LimitFault::OutsideTypeRange { leaf } => RecipeError::LimitOutsideTypeRange { tag, leaf },
        LimitFault::MinGreaterThanMax { leaf, min, max } => RecipeError::MinGreaterThanMax {
            tag,
            leaf,
            min,
            max,
        },
    }
}

/// Wrap the fault of a value with the tag of the parameter.
fn value_error(tag: &ParameterTag, fault: ValueFault) -> RecipeError {
    let tag = tag.clone();
    match fault {
        ValueFault::Mismatch { leaf, expected } => RecipeError::ValueShapeMismatch {
            tag,
            leaf,
            expected,
        },
        ValueFault::WrongTree => RecipeError::ValueShapeMismatch {
            tag,
            leaf: LeafPath::whole(),
            expected: WRONG_TREE_EXPECTED.to_owned(),
        },
        ValueFault::LimitConflict { leaf } => RecipeError::LimitNotApplicable { tag, leaf },
        ValueFault::OutOfLimit {
            leaf,
            value,
            min,
            max,
        } => RecipeError::ValueOutOfLimit {
            tag,
            leaf,
            value,
            min,
            max,
        },
    }
}

/// Add the counts of the values. A sum that overflows is above every budget.
fn sum(mut counts: impl Iterator<Item = u64>) -> u64 {
    counts.try_fold(0, u64::checked_add).unwrap_or(u64::MAX)
}

/// Convert a length to a count. A length above `u64` is above every budget.
fn count(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

impl Draft {
    /// Check the size budgets of the whole draft as it is (design D11).
    ///
    /// A command that changes a parameter calls this method on the changed copy of the draft.
    /// A parameter whose type tree failed to resolve adds no leaves to the count.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees do not hold the type of a parameter.
    /// Returns `E-RCP-021` for the first count above its limit.
    pub(in crate::recipe::draft) fn check_budgets_total(
        &self,
        trees: &TypeTrees,
    ) -> Result<(), DraftError> {
        let leaves = self.leaf_total(trees)?;
        let limit_rows = self.parameters.values().map(|p| count(p.limits().len()));
        check_budgets(BudgetCounts {
            parameters: count(self.parameters.len()),
            leaves,
            limit_rows: sum(limit_rows),
        })
        .map_err(DraftError::from)
    }

    /// Count the leaves of all parameters whose type tree resolved.
    fn leaf_total(&self, trees: &TypeTrees) -> Result<u64, PlantError> {
        let mut leaves = Vec::with_capacity(self.parameters.len());
        for parameter in self.parameters.values() {
            if let Some(tree) = self.held_tree(trees, parameter.plc_type())? {
                leaves.push(u64::from(tree.leaf_count()));
            }
        }
        Ok(sum(leaves.into_iter()))
    }
}
