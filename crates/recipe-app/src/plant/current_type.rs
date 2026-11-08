//! The check of a type text with the current rule set (ADR-0143 (i2), ADR-0120 (c)).

use recipe_domain::plant::{PlantError, TypeExpr};

/// Check the type text that a command writes with the current rule (ADR-0143 (i2)).
///
/// The function solves one problem: a request can carry a `TypeExpr` that the earlier rule
/// parsed, for example `Int@1`, because the edit form sends the stored type text again. A command
/// checks the part that it writes with the current rule set (ADR-0120 (c)). The function parses
/// the text of `expr` again with [`TypeExpr::parse`].
///
/// # Errors
///
/// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) with the reason `ReservedName` when a
/// struct name is a reserved word or a primitive name in any case.
pub(crate) fn require_current_type(expr: &TypeExpr) -> Result<(), PlantError> {
    TypeExpr::parse(&expr.to_string()).map(|_| ())
}
