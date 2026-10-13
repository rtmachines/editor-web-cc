//! The first group of checks for a parameter definition (design D6, step 1).

use super::NewParameter;
use crate::plant::{LiveUnit, PlantError, PlcTypeId, S7Address, TypeTree, TypeTrees};
use crate::recipe::draft::Draft;
use crate::recipe::draft::guard::not_on_platform;
use crate::recipe::draft::s7::check_definition;
use crate::recipe::{DraftError, ParameterKey, ParameterTag, RecipeError};

/// The parts of a parameter that the first group of checks looks at.
///
/// The type solves one problem: a new parameter and a changed definition run the same checks
/// without a copy of the code.
#[derive(Clone, Copy)]
pub(in crate::recipe::draft) struct DefinitionParts<'a> {
    /// The live unit that holds the parameter.
    pub unit: &'a LiveUnit,
    /// The tag of the parameter.
    pub tag: &'a ParameterTag,
    /// The PLC type of the parameter.
    pub plc_type: PlcTypeId,
    /// The S7 address, or `None` when the parameter has none.
    pub s7_address: Option<S7Address>,
}

impl Draft {
    /// Run the first group of checks for a new parameter (design D6, step 1).
    ///
    /// The method checks the tag and the key, the unit, and the machine and the platform of the
    /// trees. Then it checks the type tree and the S7 rules. It does not change the draft.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-014`, `E-PLT-020`, `E-PLT-013`, the `PlantError` of a failed resolution,
    /// `E-RCP-003`, `E-RCP-053`, `E-RCP-054`, or `E-RCP-023` for the first check that fails.
    pub(super) fn check_new_definition<'t>(
        &self,
        new: &NewParameter,
        trees: &'t TypeTrees,
    ) -> Result<&'t TypeTree, DraftError> {
        let is_id_used = self.parameters.values().any(|p| p.id() == new.id);
        if self.parameters.contains_key(&new.key) || is_id_used {
            return Err(self.duplicate_tag(&new.tag).into());
        }
        let parts = DefinitionParts {
            unit: &new.unit,
            tag: &new.tag,
            plc_type: new.plc_type,
            s7_address: new.s7_address,
        };
        self.check_definition_parts(None, parts, trees)
    }

    /// Run the first group of checks for the definition of one parameter (design D6, step 1).
    ///
    /// `except` names the parameter that the definition replaces. Its own tag is free. The method
    /// does not change the draft.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-014`, `E-PLT-020`, `E-PLT-013`, the `PlantError` of a failed resolution,
    /// `E-RCP-003`, `E-RCP-053`, `E-RCP-054`, or `E-RCP-023` for the first check that fails.
    pub(in crate::recipe::draft) fn check_definition_parts<'t>(
        &self,
        except: Option<ParameterKey>,
        parts: DefinitionParts<'_>,
        trees: &'t TypeTrees,
    ) -> Result<&'t TypeTree, DraftError> {
        let tag_is_used = self
            .parameters
            .iter()
            .any(|(key, p)| Some(*key) != except && p.tag() == parts.tag);
        if tag_is_used {
            return Err(self.duplicate_tag(parts.tag).into());
        }
        if parts.unit.machine() != self.machine {
            return Err(PlantError::UnitNotFound {
                path: parts.unit.path().clone(),
            }
            .into());
        }
        let tree = self.resolved_tree(trees, parts.plc_type)?;
        check_definition(self.platform, parts.tag, parts.s7_address, tree)?;
        Ok(tree)
    }

    /// Return the type tree of `plc_type` for the machine and the platform of the draft.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform, or when they
    /// do not offer the type. Returns the `PlantError` of a failed resolution.
    pub(in crate::recipe::draft) fn resolved_tree<'t>(
        &self,
        trees: &'t TypeTrees,
        plc_type: PlcTypeId,
    ) -> Result<&'t TypeTree, PlantError> {
        self.check_trees_belong(trees, plc_type)?;
        match trees.get(plc_type) {
            None => Err(not_on_platform(trees, plc_type)),
            Some(Err(error)) => Err(error.clone()),
            Some(Ok(tree)) => Ok(tree),
        }
    }

    /// Build the error of a tag that another parameter of the draft uses.
    fn duplicate_tag(&self, tag: &ParameterTag) -> RecipeError {
        RecipeError::DuplicateTag {
            tag: tag.clone(),
            number: self.number,
        }
    }
}
