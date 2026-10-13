//! The one guard between a draft and the type trees of a command (ADR-0028).

use super::Draft;
use crate::plant::{DerivedTypeReason, PlantError, PlcTypeId, TypeTree, TypeTrees};

impl Draft {
    /// Check that the trees belong to the machine and the platform of the draft.
    ///
    /// `shown` names the type in the error. Every command and every check of a whole draft
    /// uses this guard, so trees of another machine never pass.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform.
    pub(in crate::recipe) fn check_trees_belong(
        &self,
        trees: &TypeTrees,
        shown: PlcTypeId,
    ) -> Result<(), PlantError> {
        if trees.machine() != self.machine || trees.platform() != self.platform {
            return Err(not_on_platform(trees, shown));
        }
        Ok(())
    }

    /// Check that the trees belong to the machine and the platform of the draft.
    ///
    /// The error names the type of the first parameter in tag order. A draft without parameters
    /// has no type to check, so it accepts any trees.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform.
    pub(in crate::recipe) fn check_draft_trees(&self, trees: &TypeTrees) -> Result<(), PlantError> {
        match self.parameters().first() {
            Some(first) => self.check_trees_belong(trees, first.plc_type()),
            None => Ok(()),
        }
    }

    /// Return the tree of a type that a whole-draft check walks.
    ///
    /// The method returns `None` for a type whose resolution failed. The snapshot check reports
    /// that failure, so the caller skips the parameter.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform, or when the
    /// trees do not hold the type. A partial `TypeTrees` therefore cannot hide a parameter.
    pub(in crate::recipe::draft) fn held_tree<'t>(
        &self,
        trees: &'t TypeTrees,
        plc_type: PlcTypeId,
    ) -> Result<Option<&'t TypeTree>, PlantError> {
        self.check_trees_belong(trees, plc_type)?;
        match trees.get(plc_type) {
            None => Err(not_on_platform(trees, plc_type)),
            Some(Err(_)) => Ok(None),
            Some(Ok(tree)) => Ok(Some(tree)),
        }
    }
}

/// Build the error of a type that the trees do not offer on the platform of the draft.
pub(crate) fn not_on_platform(trees: &TypeTrees, plc_type: PlcTypeId) -> PlantError {
    PlantError::DerivedTypeInvalid {
        type_expr: trees.label(plc_type),
        reason: DerivedTypeReason::NotOnPlatform,
    }
}
