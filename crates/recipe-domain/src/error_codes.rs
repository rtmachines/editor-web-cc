//! Stable error codes of the domain (`reliability.md` section 1.2).
//!
//! A code has the text form `E-<CTX>-<nnn>`. Only this module creates
//! values of [`ErrorCode`], so every code appears in [`REGISTRY`].

use std::fmt;

/// A stable error code such as `E-PLT-001`.
///
/// The type solves one problem: a caller cannot invent a code that has no
/// row in `reliability.md` section 1.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorCode(&'static str);

impl ErrorCode {
    /// E-PLT-001, `PlantError::DuplicateMachineCode`.
    pub const PLT_001: Self = Self::new("E-PLT-001");
    /// E-PLT-002, `PlantError::UnitDepthExceeded`.
    pub const PLT_002: Self = Self::new("E-PLT-002");
    /// E-PLT-003, `PlantError::ParentNotFound`.
    pub const PLT_003: Self = Self::new("E-PLT-003");
    /// E-PLT-004, `PlantError::DuplicateUnitPath`.
    pub const PLT_004: Self = Self::new("E-PLT-004");
    /// E-PLT-005, `PlantError::TypeCycle`.
    pub const PLT_005: Self = Self::new("E-PLT-005");
    /// E-PLT-006, `PlantError::MemberTypeNotOnPlatform`.
    pub const PLT_006: Self = Self::new("E-PLT-006");
    /// E-PLT-008, `PlantError::LastMainUnit`.
    pub const PLT_008: Self = Self::new("E-PLT-008");
    /// E-PLT-009, `PlantError::UnitArchived`.
    pub const PLT_009: Self = Self::new("E-PLT-009");
    /// E-PLT-010, `PlantError::MachineArchived`.
    pub const PLT_010: Self = Self::new("E-PLT-010");
    /// E-PLT-011, `PlantError::MachineHasDraft`.
    pub const PLT_011: Self = Self::new("E-PLT-011");
    /// E-PLT-012, `PlantError::UnitReferencedByDraft`.
    pub const PLT_012: Self = Self::new("E-PLT-012");
    /// E-PLT-013, `PlantError::DerivedTypeInvalid`.
    pub const PLT_013: Self = Self::new("E-PLT-013");
    /// E-PLT-014, `PlantError::UnitReferencedByApproved`.
    pub const PLT_014: Self = Self::new("E-PLT-014");
    /// E-PLT-015, `PlantError::DuplicateTypeName`.
    pub const PLT_015: Self = Self::new("E-PLT-015");
    /// E-PLT-016, `PlantError::StructHasNoMembers`.
    pub const PLT_016: Self = Self::new("E-PLT-016");
    /// E-PLT-017, `PlantError::InvalidUnitPath`.
    pub const PLT_017: Self = Self::new("E-PLT-017");
    /// E-PLT-018, `PlantError::TypeTooLarge`.
    pub const PLT_018: Self = Self::new("E-PLT-018");
    /// E-PLT-019, `PlantError::MachineNotFound`.
    pub const PLT_019: Self = Self::new("E-PLT-019");
    /// E-PLT-020, `PlantError::UnitNotFound`.
    pub const PLT_020: Self = Self::new("E-PLT-020");

    /// E-PLT-021, `PlantError::DuplicateMemberName`.
    pub const PLT_021: Self = Self::new("E-PLT-021");

    const fn new(text: &'static str) -> Self {
        Self(text)
    }

    /// Return the text form of the code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every error code of the domain, in ascending order.
pub const REGISTRY: &[ErrorCode] = &[
    ErrorCode::PLT_001,
    ErrorCode::PLT_002,
    ErrorCode::PLT_003,
    ErrorCode::PLT_004,
    ErrorCode::PLT_005,
    ErrorCode::PLT_006,
    ErrorCode::PLT_008,
    ErrorCode::PLT_009,
    ErrorCode::PLT_010,
    ErrorCode::PLT_011,
    ErrorCode::PLT_012,
    ErrorCode::PLT_013,
    ErrorCode::PLT_014,
    ErrorCode::PLT_015,
    ErrorCode::PLT_016,
    ErrorCode::PLT_017,
    ErrorCode::PLT_018,
    ErrorCode::PLT_019,
    ErrorCode::PLT_020,
    ErrorCode::PLT_021,
];

/// Return true when `code` has the format and a number range of section 1.2.
#[cfg(test)]
fn in_registry_range(code: &str) -> bool {
    let Some(rest) = code.strip_prefix("E-") else {
        return false;
    };
    let Some((context, digits)) = rest.split_once('-') else {
        return false;
    };
    if digits.len() != 3 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let Ok(number) = digits.parse::<u16>() else {
        return false;
    };
    match context {
        "PLT" => (1..=49).contains(&number),
        "RCP" => (1..=69).contains(&number),
        "IDN" | "WEB" => (1..=19).contains(&number),
        "AUD" | "EXP" | "DB" => (1..=9).contains(&number),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_form_and_display_show_the_code() {
        let code = ErrorCode::new("E-PLT-001");
        assert_eq!(code.as_str(), "E-PLT-001");
        assert_eq!(code.to_string(), "E-PLT-001");
    }

    #[test]
    fn codes_order_by_text() {
        assert!(ErrorCode::new("E-PLT-001") < ErrorCode::new("E-PLT-002"));
        assert!(ErrorCode::new("E-PLT-009") < ErrorCode::new("E-PLT-010"));
    }

    #[test]
    fn range_check_accepts_codes_inside_the_ranges() {
        for code in [
            "E-PLT-001",
            "E-PLT-049",
            "E-RCP-050",
            "E-RCP-069",
            "E-WEB-019",
        ] {
            assert!(in_registry_range(code), "{code}");
        }
    }

    #[test]
    fn range_check_rejects_codes_outside_the_ranges() {
        for code in [
            "E-PLT-050",
            "E-RCP-070",
            "E-IDN-020",
            "E-PLT-1",
            "E-XYZ-001",
            "E-PLT-0010",
        ] {
            assert!(!in_registry_range(code), "{code}");
        }
    }

    #[test]
    fn every_registry_row_is_inside_its_range() {
        for code in REGISTRY {
            assert!(in_registry_range(code.as_str()), "{code}");
        }
    }

    #[test]
    fn registry_is_sorted_and_unique() {
        for pair in REGISTRY.windows(2) {
            assert!(pair[0] < pair[1], "{} then {}", pair[0], pair[1]);
        }
    }
}
