//! The check of a type text with the current rule set (ADR-0143 (i2), ADR-0120 (c), ADR-0186 (f)).

use recipe_domain::plant::{
    BaseExpr, PlantError, PlatformTypes, PlcPlatform, PlcType, RecordShape, TypeExpr, TypeMember,
    TypeName, check_codesys_members, check_iec_names, check_siemens_struct_name, is_reserved_name,
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

/// Tell whether a stored struct row is no proposal of a type input (ADR-0203 (j), ADR-0186 (h)).
///
/// The function solves one problem: the struct form and the parameter form must leave out the
/// structs that every command refuses. A row is refused when its name is a reserved word or a
/// primitive name. It is also refused on an S7 platform when the name is a Siemens reserved name,
/// and on `CodesysIec` and `TwinCat3` when [`check_iec_names`] refuses the name or a member name.
/// It is also refused on `CodesysIec` when it has one member ([`check_codesys_members`],
/// ADR-0204 (d)). A row that is no struct is no refused struct. The Siemens part calls
/// [`check_siemens_struct_name`], the check of [`require_current_type`].
pub(crate) fn is_refused_struct(platform: PlcPlatform, row: &PlcType) -> bool {
    let RecordShape::Struct { name, members, .. } = row.shape() else {
        return false;
    };
    is_reserved_name(name.as_str())
        || check_siemens_struct_name(platform, name).is_err()
        || check_iec_rules(platform, name, members).is_err()
}

/// Run [`check_iec_names`] and then [`check_codesys_members`] on one struct (ADR-0203, ADR-0204).
///
/// The function solves one problem: each place that reads a stored struct row runs the two checks
/// in the same order, so the name reasons come before the count reason.
fn check_iec_rules(
    platform: PlcPlatform,
    name: &TypeName,
    members: &[TypeMember],
) -> Result<(), PlantError> {
    check_iec_names(platform, name, members.iter().map(|member| &member.name))?;
    check_codesys_members(platform, name, members.iter().map(|member| &member.name))
}

/// Check the stored struct revision that a type text names directly (ADR-0203 (i1)).
///
/// The function solves one problem: a command writes a type text, and the stored revision that
/// the text names can hold a reserved name that the command did not write. The function finds
/// the revision that the base of `expr` names in `types`, also as the element of an array. It
/// calls [`check_iec_names`] and [`check_codesys_members`] on that row. It does not follow the members of the revision into
/// a deeper struct. A base that is no struct, and a struct that `types` does not hold, pass. The
/// later resolution reports a missing struct. The check is pure and reads no row.
///
/// # Errors
///
/// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) with the reason `ReservedName` or
/// `ReservedMemberName` as [`check_iec_names`] gives it. On `CodesysIec`, it also returns
/// the reason `TooFewMembers` for a revision with one member (ADR-0204 (c1)).
pub(crate) fn check_iec_struct(types: &PlatformTypes, expr: &TypeExpr) -> Result<(), PlantError> {
    check_iec_struct_in(types.platform(), types.rows(), expr)
}

/// The check of [`check_iec_struct`] over the rows `rows` of one platform.
///
/// `DefineUdt` calls it with the rows of its working set, which are no `PlatformTypes`.
pub(crate) fn check_iec_struct_in<'a>(
    platform: PlcPlatform,
    rows: impl IntoIterator<Item = &'a PlcType>,
    expr: &TypeExpr,
) -> Result<(), PlantError> {
    let BaseExpr::Udt { name, revision } = expr.base() else {
        return Ok(());
    };
    let stored = rows.into_iter().find_map(|row| match row.shape() {
        RecordShape::Struct {
            name: found,
            revision: number,
            members,
            ..
        } if found == name && number == revision && row.platforms().contains(&platform) => {
            Some(members)
        }
        _ => None,
    });
    let Some(members) = stored else {
        return Ok(());
    };
    check_iec_rules(platform, name, members)
}
