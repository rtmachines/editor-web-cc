//! Changing the value or the limits of a parameter in a draft (design D6).

use super::add::step_content::{
    check_faults, checked_value, parse_limits, stored_limit_conflict, stored_limits_under,
    stored_value_under,
};
use super::{Draft, Parameter, ParameterChange};
use crate::plant::{DerivedTypeReason, PlantError, PlcTypeId, RuleSet, TypeTree, TypeTrees};
use crate::recipe::read_check::StoredDefect;
use crate::recipe::{DraftError, LimitList, ParameterKey, ProcessLimit, RecipeErrors, ValueInput};

impl Draft {
    /// Set the value of a parameter and keep its limits and translations (design D6).
    ///
    /// The method checks the stored limits against the type tree. Then it parses the new value
    /// and checks every leaf against the limits (ADR-0041). Then it checks the size budgets of
    /// the changed draft. The draft changes only after every check passes.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` for an unknown key. Returns `E-PLT-013` for a type that the platform
    /// does not hold. Returns `E-PLT-005`, `E-PLT-018`, or `E-PLT-013` with the reason
    /// `RowShape` inside a stored defect for a stored type that does not resolve. Returns
    /// `E-RCP-017`, `E-RCP-019`, `E-RCP-006`, or `E-RCP-018` inside a stored defect for a stored
    /// limit that the tree rejects. Returns `E-RCP-019` inside a stored defect for a leaf that
    /// two stored limits cover.
    ///
    /// Returns the faults of the new value in the preorder of the leaves. Returns `E-RCP-005`
    /// for a shape fault, `E-RCP-004` for a value outside its limits, and `E-RCP-055` for too
    /// many significant digits. A JSON value reports only its first shape fault.
    ///
    /// Returns `E-RCP-021` for a budget. Returns `E-RCP-022` for an address overlap that holds
    /// the changed parameter. Returns the other S7 layout faults of `check_s7_layout`.
    pub fn set_value(
        &mut self,
        key: ParameterKey,
        value: &ValueInput,
        trees: &TypeTrees,
    ) -> Result<ParameterChange, DraftError> {
        let old = self.parameter(key)?.clone();
        let tree = self.stored_tree(key, trees, old.plc_type())?;
        // The command reads the stored limits and does not write them (ADR-0120 (d)).
        let stored = |error| DraftError::Stored(Box::new(StoredDefect::of_parameter(key, error)));
        let limits =
            stored_limits_under(old.tag(), old.limits(), tree, RuleSet::Earlier).map_err(stored)?;
        stored_limit_conflict(old.tag(), &limits, tree).map_err(stored)?;
        let value = checked_value(old.tag(), value, &limits, tree)?;
        let new = old.with_value(value.to_stored());
        self.replace_checked(old, new, trees)
    }

    /// Replace the limits of a parameter and keep its value and translations (design D6).
    ///
    /// The method parses every new limit row against the type tree. Then it checks every leaf
    /// of the stored value against the rows that parsed (ADR-0041). Then it checks the size
    /// budgets of the changed draft. The draft changes only after every check passes.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` for an unknown key. Returns `E-PLT-013` for a type that the platform
    /// does not hold. Returns `E-PLT-005`, `E-PLT-018`, or `E-PLT-013` with the reason
    /// `RowShape` inside a stored defect for a stored type that does not resolve. Returns
    /// `E-RCP-005` inside a stored defect for a stored value that the tree rejects.
    ///
    /// Returns a list of faults for the value and the rows. The list holds every `E-RCP-004`
    /// of the value in the preorder of the leaves. Then it holds every `E-RCP-017`,
    /// `E-RCP-019`, `E-RCP-006`, `E-RCP-018`, or `E-RCP-055` of the rows, in the order of the
    /// rows.
    ///
    /// Returns `E-RCP-021` for a budget. Returns `E-RCP-022` for an address overlap that holds
    /// the changed parameter. Returns the other S7 layout faults of `check_s7_layout`.
    pub fn replace_limits(
        &mut self,
        key: ParameterKey,
        limits: &LimitList,
        trees: &TypeTrees,
    ) -> Result<ParameterChange, DraftError> {
        let old = self.parameter(key)?.clone();
        let tree = self.stored_tree(key, trees, old.plc_type())?;
        let (parsed, row_errors) = parse_limits(old.tag(), limits, tree);
        // The command reads the stored value and does not write it (ADR-0120 (d)).
        let value = stored_value_under(old.tag(), old.value(), tree, RuleSet::Earlier).map_err(
            |error| DraftError::Stored(Box::new(StoredDefect::of_parameter(key, error))),
        )?;
        let mut errors = check_faults(old.tag(), &value, &parsed, (tree, RuleSet::Earlier));
        errors.extend(row_errors);
        if let Some(errors) = RecipeErrors::from_vec(errors) {
            return Err(errors.into());
        }
        let rows = parsed.iter().map(ProcessLimit::to_stored).collect();
        let new = old.with_limits(rows);
        self.replace_checked(old, new, trees)
    }

    /// Return the type tree of a stored parameter and wrap a defect of the stored type.
    ///
    /// A type that cannot resolve because of its stored rows is a defect of the storage
    /// (ADR-0195 (w)). A type that the platform does not hold stays a plain error.
    fn stored_tree<'t>(
        &self,
        key: ParameterKey,
        trees: &'t TypeTrees,
        plc_type: PlcTypeId,
    ) -> Result<&'t TypeTree, DraftError> {
        self.resolved_tree(trees, plc_type).map_err(|error| {
            if is_stored_resolution_defect(&error) {
                DraftError::Stored(Box::new(StoredDefect::of_parameter(key, error)))
            } else {
                error.into()
            }
        })
    }

    /// Replace one parameter after the whole-draft checks pass.
    fn replace_checked(
        &mut self,
        old: Parameter,
        new: Parameter,
        trees: &TypeTrees,
    ) -> Result<ParameterChange, DraftError> {
        self.replace_parameters(vec![new.clone()], trees)?;
        Ok(ParameterChange::new(Some(old), Some(new)))
    }
}

/// Tell whether a failed resolution comes from the stored rows of the type (design D2).
///
/// A type that the platform does not hold is no such defect. Only `TypeCycle`, `TypeTooLarge`, and
/// `DerivedTypeInvalid` with the reason `RowShape` are.
#[must_use]
pub const fn is_stored_resolution_defect(error: &PlantError) -> bool {
    matches!(
        error,
        PlantError::TypeCycle { .. }
            | PlantError::TypeTooLarge { .. }
            | PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::RowShape,
                ..
            }
    )
}

#[cfg(test)]
mod tests;
