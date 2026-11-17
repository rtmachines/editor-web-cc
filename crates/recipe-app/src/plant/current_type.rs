//! The check of a type text with the current rule set (ADR-0143 (i2), ADR-0120 (c), ADR-0186 (f)).

use recipe_domain::plant::{
    BaseExpr, PlantError, PlcPlatform, TypeExpr, TypeName, check_siemens_struct_name,
    is_reserved_name,
};

/// Check the type text that a command writes with the current rule (ADR-0143 (i2)).
///
/// The function solves one problem: a request can carry a `TypeExpr` that the earlier rule
/// parsed, for example `Int@1`, because the edit form sends the stored type text again. A command
/// checks the part that it writes with the current rule set (ADR-0120 (c)). The function parses
/// the text of `expr` again with [`TypeExpr::parse`]. On an S7 platform, it also refuses a struct
/// reference with a Siemens reserved name (ADR-0186 (f)). The check is pure and reads no row.
///
/// # Errors
///
/// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) with the reason `ReservedName` when a
/// struct name is a reserved word or a primitive name in any case, or when the platform is an S7
/// platform and the struct name is a Siemens reserved name.
pub(crate) fn require_current_type(
    expr: &TypeExpr,
    platform: PlcPlatform,
) -> Result<(), PlantError> {
    let parsed = TypeExpr::parse(&expr.to_string())?;
    if let BaseExpr::Udt { name, .. } = parsed.base() {
        check_siemens_struct_name(platform, name)?;
    }
    Ok(())
}

/// Tell whether a stored struct name is no proposal of a type input (ADR-0186 (h), ADR-0143 (i2)).
///
/// The function solves one problem: the struct form and the parameter form must leave out the
/// names that every command refuses. A name is refused when it is a reserved word or a primitive
/// name, or when the platform is an S7 platform and the name is a Siemens reserved name. The
/// Siemens part calls [`check_siemens_struct_name`], the check of [`require_current_type`].
pub(crate) fn is_refused_struct_name(platform: PlcPlatform, name: &TypeName) -> bool {
    is_reserved_name(name.as_str()) || check_siemens_struct_name(platform, name).is_err()
}
