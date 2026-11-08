//! The read check of a stored parameter (ADR-0118, design D7).
//!
//! A page that shows a stored value, limit, or address must tell two cases apart. A part that
//! breaks only the current rules is a rule fault. The page shows the stored data and a field line.
//! A part that breaks the earlier rules is a structure fault. The caller keeps `500 E-DB-008`
//! (ADR-0118 (d)).

use std::slice;

use crate::error_codes::ErrorCode;
use crate::plant::{LeafPath, RuleSet, S7Address, TypeTree};
use crate::recipe::draft::{
    check_data_block_rule, check_end_rule, check_faults, leaf_error, stored_limits_under,
    stored_value_under,
};
use crate::recipe::{
    DraftError, Expected, Parameter, ParameterTag, ParameterValue, PrecisionField, RecipeError,
    StoredLimit,
};

/// The part of a stored parameter that carries a rule fault.
///
/// The type solves one problem: the page puts the field line under the input of the part. A value
/// and a limit bound name their leaf. The address has no leaf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleField {
    /// The value of the leaf.
    Value(LeafPath),
    /// The lower bound of the limit row of the leaf.
    LimitMin(LeafPath),
    /// The upper bound of the limit row of the leaf.
    LimitMax(LeafPath),
    /// The S7 address of the parameter.
    Address,
}

impl RuleField {
    /// Return the leaf of the part, or `None` for the address.
    #[must_use]
    pub const fn leaf(&self) -> Option<&LeafPath> {
        match self {
            Self::Value(leaf) | Self::LimitMin(leaf) | Self::LimitMax(leaf) => Some(leaf),
            Self::Address => None,
        }
    }
}

/// A stored part that passes the earlier rule set and breaks the current rule set (ADR-0118 (c)).
///
/// The error holds the code and the values of the field line. Its code is `E-RCP-005`,
/// `E-RCP-006`, `E-RCP-055`, `E-RCP-056`, or `E-RCP-057`.
///
/// The check of a whole version adds `E-RCP-022` on the address. The version check finds an
/// overlap of two addresses, so the single-parameter read check never returns that code
/// (ADR-0149 (e), (g)).
#[derive(Debug)]
pub struct RuleFault {
    field: RuleField,
    error: RecipeError,
}

impl RuleFault {
    /// Make the rule fault of an address that overlaps another address (`E-RCP-022`).
    ///
    /// The overlap check of a version needs it, because only that check sees two parameters
    /// (ADR-0149 (g)).
    pub(crate) const fn overlap(
        tag: ParameterTag,
        other_tag: ParameterTag,
        address: S7Address,
    ) -> Self {
        Self {
            field: RuleField::Address,
            error: RecipeError::S7AddressOverlap {
                tag,
                other_tag,
                address,
            },
        }
    }

    /// Return the part that carries the fault.
    #[must_use]
    pub const fn field(&self) -> &RuleField {
        &self.field
    }

    /// Return the error of the broken rule.
    #[must_use]
    pub const fn error(&self) -> &RecipeError {
        &self.error
    }

    /// Return the stable code of the broken rule.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.error.code()
    }

    /// Return the error of the broken rule and drop the field.
    #[must_use]
    pub fn into_error(self) -> RecipeError {
        self.error
    }
}

/// A stored parameter that breaks the earlier rule set or cannot be checked (ADR-0118 (d)).
///
/// The type solves one problem: a caller must not show a storage defect as a rule fault. The
/// caller gives `500 E-DB-008` and writes one log line.
///
/// The type is a plain classification and not an error type. The error is the inner
/// [`DraftError`], which a caller takes with [`StructureFault::into_error`].
#[derive(Debug)]
pub struct StructureFault(DraftError);

impl StructureFault {
    /// Return the first error of the damaged part.
    #[must_use]
    pub const fn error(&self) -> &DraftError {
        &self.0
    }

    /// Return the stable code of the first error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.0.code()
    }

    /// Return the first error of the damaged part and drop the classification.
    #[must_use]
    pub fn into_error(self) -> DraftError {
        self.0
    }
}

impl From<RecipeError> for StructureFault {
    fn from(error: RecipeError) -> Self {
        Self(error.into())
    }
}

impl From<DraftError> for StructureFault {
    fn from(error: DraftError) -> Self {
        Self(error)
    }
}

/// Check a stored parameter for a read, first with the earlier rule set and then with the
/// current rule set (ADR-0118 (b), design D7).
///
/// `tree` is the type tree of the parameter. The platform of the tree selects the address rules.
/// The check reuses the stored parsers and the address rules of the commands. It needs no draft.
///
/// The faults come in this order: the value, the limit rows in stored order (`min` before `max`),
/// and the address (`E-RCP-057` before `E-RCP-056`). The value gives one fault for each leaf that
/// breaks the current set, in tree order. Each fault keeps its own leaf and code.
///
/// # Errors
///
/// Returns a [`StructureFault`] when the value, a limit row, or the value against the limits
/// breaks the earlier rule set. A stored part that fails the earlier set is a storage defect.
pub fn check_stored_parameter(
    parameter: &Parameter,
    tree: &TypeTree,
) -> Result<Vec<RuleFault>, StructureFault> {
    let tag = parameter.tag();
    let limits = stored_limits_under(tag, parameter.limits(), tree, RuleSet::Earlier)?;
    let value = stored_value_under(tag, parameter.value(), tree, RuleSet::Earlier)?;
    let earlier = (tree, RuleSet::Earlier);
    if let Some(error) = check_faults(tag, &value, &limits, earlier)
        .into_iter()
        .next()
    {
        return Err(error.into());
    }
    // The current ranges lie inside the earlier ranges. A value and a limit that pass the current
    // set therefore still fit each other, so the check against the limits needs no second run.
    let mut faults = Vec::new();
    faults.extend(value_faults(parameter, &value, tree)?);
    for row in parameter.limits() {
        faults.extend(limit_faults(parameter, row, tree)?);
    }
    faults.extend(address_faults(parameter, tree)?);
    Ok(faults)
}

/// Check every leaf of the stored value with the current set, in tree order.
///
/// `earlier` is the value that passed the earlier set, so each fault is a rule fault of its own
/// leaf and keeps its own code (ADR-0118 (b), (c)).
fn value_faults(
    parameter: &Parameter,
    earlier: &ParameterValue,
    tree: &TypeTree,
) -> Result<Vec<RuleFault>, StructureFault> {
    let tag = parameter.tag();
    let wrong_tree = |_| RecipeError::ValueShapeMismatch {
        tag: tag.clone(),
        leaf: LeafPath::whole(),
        expected: Expected::ParameterType,
    };
    let leaf_faults = earlier
        .leaf_faults_under(tree, RuleSet::Current)
        .map_err(wrong_tree)?;
    leaf_faults
        .into_iter()
        .map(|(leaf, fault)| {
            let error = leaf_error(tag, leaf, fault);
            match &error {
                RecipeError::ValueShapeMismatch { leaf, .. }
                | RecipeError::PrecisionExceeded { leaf, .. } => Ok(RuleFault {
                    field: RuleField::Value(leaf.clone()),
                    error,
                }),
                _ => Err(error.into()),
            }
        })
        .collect()
}

/// Check one stored limit row with the current set. A fault is a rule fault of a bound.
fn limit_faults(
    parameter: &Parameter,
    row: &StoredLimit,
    tree: &TypeTree,
) -> Result<Vec<RuleFault>, StructureFault> {
    let Err(error) = row_error(parameter, row, tree) else {
        return Ok(Vec::new());
    };
    match error {
        RecipeError::PrecisionExceeded {
            field: PrecisionField::LimitMin | PrecisionField::LimitMax,
            ..
        }
        | RecipeError::LimitOutsideTypeRange { .. } => {
            let faults = bound_faults(parameter, row, tree);
            if faults.is_empty() {
                Err(error.into())
            } else {
                Ok(faults)
            }
        }
        other => Err(other.into()),
    }
}

/// Find the bounds of a row that break the current rule set.
///
/// The error of a row names one fault only. A bound can be outside the type range or have too
/// many significant digits, and both bounds can fail (ADR-0118 (c)). The function parses the row
/// once with the `min` only and once with the `max` only, so it reuses the parser of the row.
fn bound_faults(parameter: &Parameter, row: &StoredLimit, tree: &TypeTree) -> Vec<RuleFault> {
    let leaf = &row.leaf_path;
    let min_only = StoredLimit {
        max: None,
        ..row.clone()
    };
    let max_only = StoredLimit {
        min: None,
        ..row.clone()
    };
    let probes = [
        (
            row.min.is_some(),
            RuleField::LimitMin(leaf.clone()),
            min_only,
        ),
        (
            row.max.is_some(),
            RuleField::LimitMax(leaf.clone()),
            max_only,
        ),
    ];
    probes
        .into_iter()
        .filter(|(is_set, ..)| *is_set)
        .filter_map(|(_, field, probe)| {
            row_error(parameter, &probe, tree)
                .err()
                .map(|error| RuleFault { field, error })
        })
        .collect()
}

/// Parse one stored limit row with the current set.
fn row_error(parameter: &Parameter, row: &StoredLimit, tree: &TypeTree) -> Result<(), RecipeError> {
    stored_limits_under(
        parameter.tag(),
        slice::from_ref(row),
        tree,
        RuleSet::Current,
    )
    .map(|_| ())
}

/// Check the data block rule and the end rule of the address (ADR-0116). A fault is a rule fault.
fn address_faults(
    parameter: &Parameter,
    tree: &TypeTree,
) -> Result<Vec<RuleFault>, StructureFault> {
    let (tag, address) = (parameter.tag(), parameter.s7_address());
    let mut faults = Vec::new();
    faults.extend(check_data_block_rule(tree.platform(), tag, address).err());
    match check_end_rule(tag, address, tree) {
        Ok(()) => {}
        Err(DraftError::Recipe(error)) => faults.push(error),
        Err(other) => return Err(other.into()),
    }
    Ok(faults
        .into_iter()
        .map(|error| RuleFault {
            field: RuleField::Address,
            error,
        })
        .collect())
}

#[cfg(test)]
mod tests;
