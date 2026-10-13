//! Changing the definition of a parameter in a draft (design D6, `UpdateParameterDefinition`).

use super::add::step_content::parse_content;
use super::add::step_definition::DefinitionParts;
use super::{Draft, ParameterChange};
use crate::plant::{LeafPath, LiveUnit, PlcTypeId, S7Address, TypeTrees};
use crate::recipe::{DraftError, LimitList, Ordinal, ParameterKey, ParameterTag, RecipeError};
use crate::text::{LongText, Text};

/// The text of `E-RCP-005` when a type change comes without a value.
const RETYPE_EXPECTED: &str = "a new value for the new type";

/// The new definition of a parameter, after the use case parsed the unit.
///
/// The type solves one problem: `Draft::set_definition` gets every part of the definition in one
/// value. A caller cannot pass the parts in the wrong order.
#[derive(Clone, Debug)]
pub struct DefinitionUpdate {
    /// The live unit that holds the parameter.
    pub unit: LiveUnit,
    /// The tag of the parameter.
    pub tag: ParameterTag,
    /// The English name.
    pub name_en: Text,
    /// The English description.
    pub description_en: LongText,
    /// The PLC type of the parameter.
    pub plc_type: PlcTypeId,
    /// The S7 address, or `None` when the parameter has none.
    pub s7_address: Option<S7Address>,
    /// The position of the parameter in its draft.
    pub ordinal: Ordinal,
}

/// A new value and new limits for a parameter.
///
/// The type solves one problem: a type change needs both parts together. The old value and the
/// old limits do not fit the new type.
#[derive(Clone, Debug)]
pub struct Retype {
    /// The new value as JSON text.
    pub value_json: String,
    /// The new process limits.
    pub limits: LimitList,
}

impl Draft {
    /// Change the definition of a parameter (design D6).
    ///
    /// The method runs the first group of checks of `add_parameter` on the new definition. The
    /// tag of the parameter itself is free. A change of the PLC type needs `retype`. Without a
    /// type change, `retype` is optional. Without it, the value and the limits stay. The method
    /// checks the size budgets and the S7 rules on the changed draft. The parameter keeps its
    /// identifier, its key, and its translations. The draft changes only after every check
    /// passes. A parameter whose type did not resolve can get a new type.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` for an unknown key. Then it returns the errors of the checks of
    /// `add_parameter`. A type change without `retype` gives `E-RCP-005`.
    pub fn set_definition(
        &mut self,
        key: ParameterKey,
        update: DefinitionUpdate,
        retype: Option<Retype>,
        trees: &TypeTrees,
    ) -> Result<ParameterChange, DraftError> {
        let old = self.parameter(key)?.clone();
        let parts = DefinitionParts {
            unit: &update.unit,
            tag: &update.tag,
            plc_type: update.plc_type,
            s7_address: update.s7_address,
        };
        let tree = self.check_definition_parts(Some(key), parts, trees)?;
        let content = match retype {
            Some(retype) => {
                let (value, limits) =
                    parse_content(&update.tag, &retype.value_json, &retype.limits, tree)?;
                let rows = limits.iter().map(|limit| limit.to_stored()).collect();
                Some((value.to_stored(), rows))
            }
            None if update.plc_type != old.plc_type() => {
                return Err(RecipeError::ValueShapeMismatch {
                    tag: update.tag,
                    leaf: LeafPath::whole(),
                    expected: RETYPE_EXPECTED.to_owned(),
                }
                .into());
            }
            None => None,
        };
        let new = old.redefined(update, content);
        self.replace_parameters(vec![new.clone()], trees)?;
        Ok(ParameterChange::new(Some(old), Some(new)))
    }
}

#[cfg(test)]
mod tests;
