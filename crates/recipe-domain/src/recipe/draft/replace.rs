//! The one commit path of the parameter commands of a draft (design D6, D13).

use super::{Draft, Parameter};
use crate::plant::TypeTrees;
use crate::recipe::DraftError;

impl Draft {
    /// Insert or replace parameters, after the size budgets and the S7 rules pass.
    ///
    /// The method solves one problem: every command that changes parameters runs the same
    /// whole-draft checks in the same order. The method checks the changed copy of the draft.
    /// The draft changes only after every check passes.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-021` for the first count above its limit. Then it returns the errors of the
    /// S7 layout check.
    pub(in crate::recipe) fn replace_parameters(
        &mut self,
        replacements: Vec<Parameter>,
        trees: &TypeTrees,
    ) -> Result<(), DraftError> {
        let mut next = self.clone();
        for parameter in replacements {
            next.parameters.insert(parameter.key(), parameter);
        }
        next.check_budgets_total(trees)?;
        next.check_s7_layout(trees)?;
        *self = next;
        Ok(())
    }
}
