//! Adding a parameter to a draft (design D6).

use super::{Draft, Parameter, ParameterChange};
use crate::plant::{LiveUnit, PlcTypeId, S7Address, TypeTrees};
use crate::recipe::{
    DraftError, LimitList, Ordinal, ParameterId, ParameterKey, ParameterTag, ProcessLimit,
};
use crate::text::{LongText, Text};

pub(super) mod step_content;
pub(super) mod step_definition;

/// The request data of a new parameter, after the use case parsed the unit and the lists.
///
/// The type solves one problem: `Draft::add_parameter` gets every part of the parameter in one
/// value. A caller cannot pass the parts in the wrong order. The server generates `id` and
/// `key` (A40). The value stays JSON text until the draft parses it with the type tree.
#[derive(Clone, Debug)]
pub struct NewParameter {
    /// The stable identity of the new parameter.
    pub id: ParameterId,
    /// The key of the new parameter in a URL.
    pub key: ParameterKey,
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
    /// The value as JSON text.
    pub value_json: String,
    /// The process limits of the request.
    pub limits: LimitList,
}

impl Draft {
    /// Add a new parameter to the draft (design D6).
    ///
    /// The method builds the parameter first. Then it runs the checks in three groups. The first
    /// group is the tag, the unit, the type tree, and the S7 rules for the parameter. The second
    /// group is the limits, the shape of the value, and the value against the limits. The third
    /// group is the size budgets and the S7 overlap of the whole draft. The draft changes only
    /// after every check passes. A failed call leaves the draft as it was.
    ///
    /// # Errors
    ///
    /// Returns the error of the first check that fails. The codes are `E-RCP-014`, `E-PLT-020`,
    /// `E-PLT-013`, `E-RCP-003`, `E-RCP-053`, `E-RCP-054`, `E-RCP-023`, `E-RCP-017`,
    /// `E-RCP-019`, `E-RCP-006`, `E-RCP-018`, `E-RCP-005`, `E-RCP-004`, `E-RCP-021`, and
    /// `E-RCP-022`. A failed type resolution returns its `PlantError`.
    pub fn add_parameter(
        &mut self,
        new: NewParameter,
        trees: &TypeTrees,
    ) -> Result<ParameterChange, DraftError> {
        let tree = self.check_new_definition(&new, trees)?;
        let (value, limits) =
            step_content::parse_content(&new.tag, &new.value_json, &new.limits, tree)?;
        let rows = limits.iter().map(ProcessLimit::to_stored).collect();
        let parameter = Parameter::from_new(new, value.to_stored(), rows);
        self.replace_parameters(vec![parameter.clone()], trees)?;
        Ok(ParameterChange::new(None, Some(parameter)))
    }
}

#[cfg(test)]
mod content;
#[cfg(test)]
mod definition;
#[cfg(test)]
pub(in crate::recipe::draft) mod support;
