//! Changing the value or the limits of a parameter in a draft (design D6).

use super::add::step_content::{
    check_value, parse_limits, parse_value, stored_limits, stored_value,
};
use super::{Draft, Parameter, ParameterChange};
use crate::plant::TypeTrees;
use crate::recipe::{DraftError, LimitList, ParameterKey, ProcessLimit};

impl Draft {
    /// Set the value of a parameter and keep its limits and translations (design D6).
    ///
    /// The method checks the stored limits against the type tree. Then it parses the new value
    /// and checks it against the limits. Then it checks the size budgets of the changed draft.
    /// The draft changes only after every check passes.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` for an unknown key. Returns `E-PLT-013` or the `PlantError` of a
    /// failed resolution for a type without a tree. Returns `E-RCP-017`, `E-RCP-019`,
    /// `E-RCP-006`, or `E-RCP-018` for a stored limit that the tree rejects. Returns
    /// `E-RCP-005` or `E-RCP-004` for the value. Returns `E-RCP-021` for a budget.
    pub fn set_value(
        &mut self,
        key: ParameterKey,
        json: &str,
        trees: &TypeTrees,
    ) -> Result<ParameterChange, DraftError> {
        let old = self.parameter(key)?.clone();
        let tree = self.resolved_tree(trees, old.plc_type())?;
        let limits = stored_limits(old.tag(), old.limits(), tree)?;
        let value = parse_value(old.tag(), json, &limits, tree)?;
        let new = old.with_value(value.to_stored());
        self.replace_checked(old, new, trees)
    }

    /// Replace the limits of a parameter and keep its value and translations (design D6).
    ///
    /// The method parses the new limits against the type tree. Then it checks the stored value
    /// against the new limits. Then it checks the size budgets of the changed draft. The draft
    /// changes only after every check passes.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` for an unknown key. Returns `E-PLT-013` or the `PlantError` of a
    /// failed resolution for a type without a tree. Returns `E-RCP-017`, `E-RCP-019`,
    /// `E-RCP-006`, or `E-RCP-018` for a new limit that the tree rejects. Returns `E-RCP-005`
    /// for a stored value of the wrong form. Returns `E-RCP-004` for a value outside the new
    /// limits. Returns `E-RCP-021` for a budget.
    pub fn replace_limits(
        &mut self,
        key: ParameterKey,
        limits: &LimitList,
        trees: &TypeTrees,
    ) -> Result<ParameterChange, DraftError> {
        let old = self.parameter(key)?.clone();
        let tree = self.resolved_tree(trees, old.plc_type())?;
        let parsed: Vec<ProcessLimit> = parse_limits(old.tag(), limits, tree)?;
        let value = stored_value(old.tag(), old.value(), tree)?;
        check_value(old.tag(), &value, &parsed, tree)?;
        let rows = parsed.iter().map(ProcessLimit::to_stored).collect();
        let new = old.with_limits(rows);
        self.replace_checked(old, new, trees)
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

#[cfg(test)]
mod tests;
