//! Reserved struct and member names on `CodesysIec` and `TwinCat3` (ADR-0203).
//!
//! A CODESYS or TwinCAT project refuses a DUT with the name of a native type or a library
//! type, and a member with the name of a keyword. A struct of the editor with such a name can
//! never match a real DUT. The 182 keywords of ADR-0143 (d) stay in `type_name.rs`.

use super::{DerivedTypeReason, MemberName, PlantError, PlcPlatform, RESERVED_KEYWORDS, TypeName};

/// The 15 struct names of ADR-0203 (c). A struct name on `CodesysIec` and `TwinCat3` is none
/// of them, in any case. None of them is in a reserved set of ADR-0143 or ADR-0186.
///
/// The first two are the keywords of [`IEC_EXTRA_KEYWORDS`]. The ops query
/// `deploy/db/check-identifier-rules.sql` holds a second copy, and a test compares both copies.
pub const IEC_RESERVED_STRUCT_NAMES: [&str; 15] = [
    "UNION",
    "END_UNION",
    "BIT",
    "__XINT",
    "__UXINT",
    "__XWORD",
    "PVOID",
    "XINT",
    "UXINT",
    "XWORD",
    "VERSION",
    "RTS_SYSTIMEDATE",
    "T_FILETIME",
    "TIMESTRUCT",
    "ST_AmsAddr",
];

/// The 2 keywords that ADR-0203 (c1) adds to the 182 keywords of ADR-0143 (d). A member name on
/// `CodesysIec` and `TwinCat3` is none of the 184 words, in any case (ADR-0203 (d)).
pub const IEC_EXTRA_KEYWORDS: [&str; 2] = ["UNION", "END_UNION"];

/// Tell whether a text is one of the 15 reserved struct names of ADR-0203 (c), in any case.
///
/// The function does not read the platform. [`check_iec_names`] does.
#[must_use]
pub fn is_iec_reserved_struct_name(text: &str) -> bool {
    IEC_RESERVED_STRUCT_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(text))
}

/// Tell whether a text is one of the 184 reserved member keywords of ADR-0203 (d), in any case.
///
/// The words are the 182 keywords of ADR-0143 (d) and [`IEC_EXTRA_KEYWORDS`]. A primitive name
/// and a type name of ADR-0203 (c2) are no reserved member names. The function does not read
/// the platform. [`check_iec_names`] does.
#[must_use]
pub fn is_iec_reserved_member_name(text: &str) -> bool {
    RESERVED_KEYWORDS
        .iter()
        .chain(IEC_EXTRA_KEYWORDS.iter())
        .any(|word| word.eq_ignore_ascii_case(text))
}

/// Refuse a reserved struct name or a reserved member name on `CodesysIec` and `TwinCat3`
/// (ADR-0203 (a), (d), (g)).
///
/// This check solves the problem that a form parses a name before it knows the platform
/// (ADR-0203 (h)). The use case calls it with the platform of the machine. The S7 platforms
/// accept every name (ADR-0203 (e)). The check reads only names. The count of the members is
/// a separate check. The struct name is checked before the members.
///
/// # Errors
///
/// Returns [`PlantError::DerivedTypeInvalid`] (`E-PLT-013`) for the struct name. The reason is
/// [`DerivedTypeReason::ReservedName`] for a struct name of ADR-0203 (a). The reason is
/// [`DerivedTypeReason::ReservedMemberName`] for the first member with a name of ADR-0203 (d).
pub fn check_iec_names<'a>(
    platform: PlcPlatform,
    name: &TypeName,
    members: impl IntoIterator<Item = &'a MemberName>,
) -> Result<(), PlantError> {
    if platform.is_s7() {
        return Ok(());
    }
    let reason = if is_iec_reserved_struct_name(name.as_str()) {
        DerivedTypeReason::ReservedName
    } else if members
        .into_iter()
        .any(|member| is_iec_reserved_member_name(member.as_str()))
    {
        DerivedTypeReason::ReservedMemberName
    } else {
        return Ok(());
    };
    Err(PlantError::DerivedTypeInvalid {
        type_expr: name.to_string(),
        reason,
    })
}

#[cfg(test)]
mod tests;
