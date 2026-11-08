//! The one commit path of the parameter commands of a draft (design D6, D13).

use super::{Draft, Parameter};
use crate::plant::TypeTrees;
use crate::recipe::{DraftError, ParameterKey};

impl Draft {
    /// Insert or replace parameters, after the size budgets and the S7 rules pass.
    ///
    /// The method solves one problem: every command that changes parameters runs the same
    /// whole-draft checks in the same order. The method checks the changed copy of the draft.
    /// The draft changes only after every check passes.
    ///
    /// The S7 overlap check gets the keys of `replacements`. It reports only an overlap of a pair
    /// that holds one of them. A stored overlap of two other parameters does not refuse the
    /// command (ADR-0149 (a), (b)).
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-021` for the first count above its limit. Then it returns the errors of the
    /// S7 layout check, with `E-RCP-022` for an overlap that holds a replaced parameter.
    pub(in crate::recipe) fn replace_parameters(
        &mut self,
        replacements: Vec<Parameter>,
        trees: &TypeTrees,
    ) -> Result<(), DraftError> {
        let written: Vec<ParameterKey> = replacements.iter().map(Parameter::key).collect();
        let mut next = self.clone();
        for parameter in replacements {
            next.parameters.insert(parameter.key(), parameter);
        }
        next.check_budgets_total(trees)?;
        next.check_s7_layout(trees, &written)?;
        *self = next;
        Ok(())
    }
}
