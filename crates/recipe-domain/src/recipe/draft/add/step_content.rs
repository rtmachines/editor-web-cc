//! The limits, the value, and the budgets of a new parameter (design D6, steps 2 and 3).

use crate::plant::{LeafPath, PlantError, RuleSet, TypeTree, TypeTrees};
use crate::recipe::draft::{BudgetCounts, Draft, check_budgets};
use crate::recipe::value::LeafFault;
use crate::recipe::{
    DraftError, Expected, LimitFault, LimitList, ParameterTag, ParameterValue, PrecisionField,
    ProcessLimit, RecipeError, RecipeErrors, StoredLimit, StoredValue, ValueFault, ValueFaults,
    ValueInput,
};

/// Parse the limits and the value of a parameter and check the value against the limits.
///
/// The function collects every fault (ADR-0041). The faults of the value come first, in the
/// preorder of the leaves. The faults of the limit rows follow in the order of the request. The
/// value is checked against the rows that parsed (design D6, step 2).
pub(in crate::recipe::draft) fn parse_content(
    tag: &ParameterTag,
    value: &ValueInput,
    rows: &LimitList,
    tree: &TypeTree,
) -> Result<(ParameterValue, Vec<ProcessLimit>), RecipeErrors> {
    let (limits, row_errors) = parse_limits(tag, rows, tree);
    match (
        checked_value(tag, value, &limits, tree),
        RecipeErrors::from_vec(row_errors),
    ) {
        (Ok(value), None) => Ok((value, limits)),
        (Ok(_), Some(errors)) => Err(errors),
        (Err(mut errors), tail) => {
            errors.extend(tail.into_iter().flat_map(RecipeErrors::into_vec));
            Err(errors)
        }
    }
}

/// Parse the limit rows of a request against the type tree, in the order of the request.
///
/// The function returns the rows that parsed and the errors of each row that did not. A row with
/// too many digits in both bounds gives two errors, one for each bound (ADR-0041).
pub(in crate::recipe::draft) fn parse_limits(
    tag: &ParameterTag,
    rows: &LimitList,
    tree: &TypeTree,
) -> (Vec<ProcessLimit>, Vec<RecipeError>) {
    let mut limits = Vec::with_capacity(rows.rows().len());
    let mut errors = Vec::new();
    for row in rows.rows() {
        match ProcessLimit::parse_all(row, tree) {
            Ok(limit) => limits.push(limit),
            Err(faults) => errors.extend(
                faults
                    .into_vec()
                    .into_iter()
                    .map(|fault| limit_error(tag, fault)),
            ),
        }
    }
    (limits, errors)
}

/// Parse the value of a request and check it against the limits (design D6, step 2).
///
/// The texts of a form give every fault of the shape and every fault against the limits, in
/// the preorder of the leaves (ADR-0041 (b)). A JSON value stops at its first fault of the shape.
pub(in crate::recipe::draft) fn checked_value(
    tag: &ParameterTag,
    input: &ValueInput,
    limits: &[ProcessLimit],
    tree: &TypeTree,
) -> Result<ParameterValue, RecipeErrors> {
    match input {
        ValueInput::Json(json) => ParameterValue::parse(json, tree)
            .map_err(ValueFaults::from)
            .and_then(|value| value.check_all(tree, limits).map(|()| value)),
        ValueInput::Texts(texts) => ParameterValue::from_texts_checked(tree, texts, limits),
    }
    .map_err(|faults| value_errors(tag, faults))
}

/// Check a parsed value against the limits under the rule set `rules` and return every fault.
///
/// The rule set bounds a leaf whose limit has no `min` or no `max` (ADR-0120 (e)).
pub(in crate::recipe) fn check_faults(
    tag: &ParameterTag,
    value: &ParameterValue,
    limits: &[ProcessLimit],
    (tree, rules): (&TypeTree, RuleSet),
) -> Vec<RecipeError> {
    value
        .check_all_under(tree, limits, rules)
        .err()
        .map(|faults| value_errors(tag, faults).into_vec())
        .unwrap_or_default()
}

/// Check the stored limit rows of a parameter against its type tree.
///
/// A row that the tree does not accept maps as in a request. A bound with too many significant
/// digits gives `E-RCP-055`, because the check runs in the current rule set (ADR-0117 (e)).
pub(in crate::recipe) fn stored_limits(
    tag: &ParameterTag,
    rows: &[StoredLimit],
    tree: &TypeTree,
) -> Result<Vec<ProcessLimit>, RecipeError> {
    stored_limits_under(tag, rows, tree, RuleSet::Current)
}

/// Check the stored limit rows of a parameter against its type tree under the rule set `rules`.
///
/// The snapshot check of `CreateVersion` uses `Earlier` (ADR-0120 (g)). A bound with too many
/// significant digits gives `E-RCP-055` in the current set only.
pub(in crate::recipe) fn stored_limits_under(
    tag: &ParameterTag,
    rows: &[StoredLimit],
    tree: &TypeTree,
    rules: RuleSet,
) -> Result<Vec<ProcessLimit>, RecipeError> {
    rows.iter()
        .map(|row| ProcessLimit::from_stored(row, tree, rules))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|fault| limit_error(tag, fault))
}

/// Check the stored value of a parameter against its type tree under the earlier rule set.
///
/// The caller reads a value that it does not write (ADR-0120 (d)). A fault of the stored form
/// gives `E-RCP-005`.
pub(in crate::recipe) fn stored_value(
    tag: &ParameterTag,
    value: &StoredValue,
    tree: &TypeTree,
) -> Result<ParameterValue, RecipeError> {
    stored_value_under(tag, value, tree, RuleSet::Earlier)
}

/// Check the stored value of a parameter against its type tree under the rule set `rules`.
///
/// A fault of the stored form gives `E-RCP-005`. A number with too many significant digits gives
/// `E-RCP-055` in the current set (ADR-0117 (e)).
pub(in crate::recipe) fn stored_value_under(
    tag: &ParameterTag,
    value: &StoredValue,
    tree: &TypeTree,
    rules: RuleSet,
) -> Result<ParameterValue, RecipeError> {
    ParameterValue::from_stored(value, tree, rules).map_err(|fault| value_error(tag, fault))
}

/// Wrap the fault of one leaf with the tag of the parameter and the path of the leaf.
pub(in crate::recipe) fn leaf_error(
    tag: &ParameterTag,
    leaf: LeafPath,
    fault: LeafFault,
) -> RecipeError {
    value_error(tag, ValueFault::from_leaf(leaf, fault))
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
        LimitFault::PrecisionExceeded {
            leaf,
            field,
            type_name,
            max_digits,
        } => RecipeError::PrecisionExceeded {
            tag,
            leaf,
            field,
            type_name: type_name.to_string(),
            max_digits,
        },
    }
}

/// Wrap every fault of a value with the tag of the parameter, in the order of the faults.
fn value_errors(tag: &ParameterTag, faults: ValueFaults) -> RecipeErrors {
    let mut errors = RecipeErrors::from(value_error(tag, faults.first().clone()));
    errors.extend(
        faults
            .into_vec()
            .into_iter()
            .skip(1)
            .map(|fault| value_error(tag, fault)),
    );
    errors
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
            expected: Expected::ParameterType,
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
        ValueFault::Precision {
            leaf,
            type_name,
            max_digits,
        } => RecipeError::PrecisionExceeded {
            tag,
            leaf,
            field: PrecisionField::Value,
            type_name: type_name.to_string(),
            max_digits,
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
