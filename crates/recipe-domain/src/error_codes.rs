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

    /// E-RCP-001, `RecipeError::DuplicateReference`.
    pub const RCP_001: Self = Self::new("E-RCP-001");
    /// E-RCP-002, `RecipeError::VersionNotEditable`.
    pub const RCP_002: Self = Self::new("E-RCP-002");
    /// E-RCP-003, `RecipeError::S7AddressRequired`.
    pub const RCP_003: Self = Self::new("E-RCP-003");
    /// E-RCP-004, `RecipeError::ValueOutOfLimit`.
    pub const RCP_004: Self = Self::new("E-RCP-004");
    /// E-RCP-005, `RecipeError::ValueShapeMismatch`.
    pub const RCP_005: Self = Self::new("E-RCP-005");
    /// E-RCP-006, `RecipeError::LimitOutsideTypeRange`.
    pub const RCP_006: Self = Self::new("E-RCP-006");
    /// E-RCP-007, `RecipeError::VersionNotDraft`.
    pub const RCP_007: Self = Self::new("E-RCP-007");
    /// E-RCP-008, `RecipeError::DraftHasNoParameters`.
    pub const RCP_008: Self = Self::new("E-RCP-008");
    /// E-RCP-009, `RecipeError::VersionNotApproved`.
    pub const RCP_009: Self = Self::new("E-RCP-009");
    /// E-RCP-010, `RecipeError::RecipeHasApprovedVersion`.
    pub const RCP_010: Self = Self::new("E-RCP-010");
    /// E-RCP-011, `RecipeError::DraftAlreadyExists`.
    pub const RCP_011: Self = Self::new("E-RCP-011");
    /// E-RCP-012, `RecipeError::RecipeDeleted`.
    pub const RCP_012: Self = Self::new("E-RCP-012");
    /// E-RCP-013, `RecipeError::TypeNotOnPlatform`.
    pub const RCP_013: Self = Self::new("E-RCP-013");
    /// E-RCP-014, `RecipeError::DuplicateTag`.
    pub const RCP_014: Self = Self::new("E-RCP-014");
    /// E-RCP-015, `RecipeError::ApproverEditedDraft`.
    pub const RCP_015: Self = Self::new("E-RCP-015");
    /// E-RCP-016, `RecipeError::LimitMissing`.
    pub const RCP_016: Self = Self::new("E-RCP-016");
    /// E-RCP-017, `RecipeError::LeafPathNotFound`.
    pub const RCP_017: Self = Self::new("E-RCP-017");
    /// E-RCP-018, `RecipeError::MinGreaterThanMax`.
    pub const RCP_018: Self = Self::new("E-RCP-018");
    /// E-RCP-019, `RecipeError::LimitNotApplicable`.
    pub const RCP_019: Self = Self::new("E-RCP-019");
    /// E-RCP-020, `RecipeError::ValidationFailed`.
    pub const RCP_020: Self = Self::new("E-RCP-020");
    /// E-RCP-021, `RecipeError::SizeBudgetExceeded`.
    pub const RCP_021: Self = Self::new("E-RCP-021");
    /// E-RCP-022, `RecipeError::S7AddressOverlap`.
    pub const RCP_022: Self = Self::new("E-RCP-022");
    /// E-RCP-023, `RecipeError::S7AddressMisaligned`.
    pub const RCP_023: Self = Self::new("E-RCP-023");
    /// E-RCP-024, `RecipeError::TypeRevisionOutdated`.
    pub const RCP_024: Self = Self::new("E-RCP-024");
    /// E-RCP-025, `RecipeError::RecipeNotDeleted`.
    pub const RCP_025: Self = Self::new("E-RCP-025");
    /// E-RCP-026, `RecipeError::RecipeHasRetiredVersion`.
    pub const RCP_026: Self = Self::new("E-RCP-026");
    /// E-RCP-027, `RecipeError::InvalidLangCode`.
    pub const RCP_027: Self = Self::new("E-RCP-027");
    /// E-RCP-028, `RecipeError::RebindNeedsValue`.
    pub const RCP_028: Self = Self::new("E-RCP-028");
    /// E-RCP-029, `RecipeError::RebindDropsLimit`.
    pub const RCP_029: Self = Self::new("E-RCP-029");
    /// E-RCP-050, `RecipeError::TranslationNotFound`.
    pub const RCP_050: Self = Self::new("E-RCP-050");
    /// E-RCP-051, `RecipeError::ParameterNotFound`.
    pub const RCP_051: Self = Self::new("E-RCP-051");
    /// E-RCP-052, `RecipeError::RebindTargetInvalid`.
    pub const RCP_052: Self = Self::new("E-RCP-052");
    /// E-RCP-053, `RecipeError::S7AddressNotAllowed`.
    pub const RCP_053: Self = Self::new("E-RCP-053");
    /// E-RCP-054, `RecipeError::S7AddressWidthMismatch`.
    pub const RCP_054: Self = Self::new("E-RCP-054");

    /// E-AUD-001, `AppError::AuditWriteFailed`.
    pub const AUD_001: Self = Self::new("E-AUD-001");
    /// E-AUD-002, `AuditError::ChainBroken`.
    pub const AUD_002: Self = Self::new("E-AUD-002");

    /// E-DB-001, `StorageError::Deadlock`.
    pub const DB_001: Self = Self::new("E-DB-001");
    /// E-DB-002, `StorageError::StatementTimeout`.
    pub const DB_002: Self = Self::new("E-DB-002");
    /// E-DB-003, `StorageError::Constraint`.
    pub const DB_003: Self = Self::new("E-DB-003");
    /// E-DB-004, `StorageError::PoolTimeout`.
    pub const DB_004: Self = Self::new("E-DB-004");
    /// E-DB-005, `StorageError::Unavailable`.
    pub const DB_005: Self = Self::new("E-DB-005");
    /// E-DB-006, `StorageError::Duplicate` with an unknown constraint name.
    pub const DB_006: Self = Self::new("E-DB-006");
    /// E-DB-007, `StorageError::MissingReference` with an unknown constraint name.
    pub const DB_007: Self = Self::new("E-DB-007");
    /// E-DB-008, `StorageError::Other`.
    pub const DB_008: Self = Self::new("E-DB-008");

    /// E-EXP-001, `ExportError::NoApprovedVersion`.
    pub const EXP_001: Self = Self::new("E-EXP-001");
    /// E-EXP-002, `ExportError::ContentHashMismatch`.
    pub const EXP_002: Self = Self::new("E-EXP-002");
    /// E-EXP-003, `ExportError::VersionNotExportable`.
    pub const EXP_003: Self = Self::new("E-EXP-003");
    /// E-EXP-004, `ExportError::Serialize`.
    pub const EXP_004: Self = Self::new("E-EXP-004");

    /// E-PLT-030, `StorageError::Constraint`, trigger `machine_platform_frozen`.
    pub const PLT_030: Self = Self::new("E-PLT-030");
    /// E-PLT-031, `StorageError::Constraint`, trigger `enforce_type_platform`.
    pub const PLT_031: Self = Self::new("E-PLT-031");
    /// E-PLT-032, `StorageError::Constraint`, trigger `enforce_type_platform`.
    pub const PLT_032: Self = Self::new("E-PLT-032");
    /// E-PLT-033, `StorageError::Constraint`, trigger `plc_type_immutable`.
    pub const PLT_033: Self = Self::new("E-PLT-033");
    /// E-PLT-034, `StorageError::Constraint`, trigger `plc_type_immutable`.
    pub const PLT_034: Self = Self::new("E-PLT-034");
    /// E-PLT-035, `StorageError::Constraint`, trigger `plc_type_immutable`.
    pub const PLT_035: Self = Self::new("E-PLT-035");
    /// E-PLT-036, `StorageError::Constraint`, trigger `plc_type_struct_has_member`.
    pub const PLT_036: Self = Self::new("E-PLT-036");
    /// E-PLT-037, `StorageError::Constraint`, trigger `freeze_identifiers`.
    pub const PLT_037: Self = Self::new("E-PLT-037");
    /// E-PLT-038, `StorageError::Constraint`, trigger `freeze_identifiers`.
    pub const PLT_038: Self = Self::new("E-PLT-038");

    /// E-RCP-030, `StorageError::Constraint`, trigger `enforce_version_mutable`.
    pub const RCP_030: Self = Self::new("E-RCP-030");
    /// E-RCP-031, `StorageError::Constraint`, trigger `enforce_version_mutable`.
    pub const RCP_031: Self = Self::new("E-RCP-031");
    /// E-RCP-032, `StorageError::Constraint`, trigger `enforce_unit_machine`.
    pub const RCP_032: Self = Self::new("E-RCP-032");
    /// E-RCP-033, `StorageError::Constraint`, trigger `recipe_version_transition`.
    pub const RCP_033: Self = Self::new("E-RCP-033");
    /// E-RCP-034, `StorageError::Constraint`, trigger `freeze_identifiers`.
    pub const RCP_034: Self = Self::new("E-RCP-034");
    /// E-RCP-035, `StorageError::Constraint`, trigger `version_size_budget`.
    pub const RCP_035: Self = Self::new("E-RCP-035");
    /// E-RCP-036, `StorageError::Constraint`, trigger `version_size_budget`.
    pub const RCP_036: Self = Self::new("E-RCP-036");

    /// E-WEB-001, `AppError::ConcurrentEdit`.
    pub const WEB_001: Self = Self::new("E-WEB-001");
    /// E-WEB-002, `AppError::Forbidden`.
    pub const WEB_002: Self = Self::new("E-WEB-002");
    /// E-WEB-004, `AppError::Unauthenticated`.
    pub const WEB_004: Self = Self::new("E-WEB-004");
    /// E-WEB-005, `AppError::NotFound`.
    pub const WEB_005: Self = Self::new("E-WEB-005");
    /// E-WEB-009, `AppError::Timeout`.
    pub const WEB_009: Self = Self::new("E-WEB-009");
    /// E-WEB-011, `AppError::TaskFailed`.
    pub const WEB_011: Self = Self::new("E-WEB-011");

    const fn new(text: &'static str) -> Self {
        Self(text)
    }

    /// Return the text form of the code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// Return the registry entry with the text `text`, or `None` when no entry has it.
    #[must_use]
    pub fn find(text: &str) -> Option<Self> {
        REGISTRY.iter().copied().find(|code| code.as_str() == text)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every error code of the domain, in ascending order.
pub const REGISTRY: &[ErrorCode] = &[
    ErrorCode::AUD_001,
    ErrorCode::AUD_002,
    ErrorCode::DB_001,
    ErrorCode::DB_002,
    ErrorCode::DB_003,
    ErrorCode::DB_004,
    ErrorCode::DB_005,
    ErrorCode::DB_006,
    ErrorCode::DB_007,
    ErrorCode::DB_008,
    ErrorCode::EXP_001,
    ErrorCode::EXP_002,
    ErrorCode::EXP_003,
    ErrorCode::EXP_004,
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
    ErrorCode::PLT_030,
    ErrorCode::PLT_031,
    ErrorCode::PLT_032,
    ErrorCode::PLT_033,
    ErrorCode::PLT_034,
    ErrorCode::PLT_035,
    ErrorCode::PLT_036,
    ErrorCode::PLT_037,
    ErrorCode::PLT_038,
    ErrorCode::RCP_001,
    ErrorCode::RCP_002,
    ErrorCode::RCP_003,
    ErrorCode::RCP_004,
    ErrorCode::RCP_005,
    ErrorCode::RCP_006,
    ErrorCode::RCP_007,
    ErrorCode::RCP_008,
    ErrorCode::RCP_009,
    ErrorCode::RCP_010,
    ErrorCode::RCP_011,
    ErrorCode::RCP_012,
    ErrorCode::RCP_013,
    ErrorCode::RCP_014,
    ErrorCode::RCP_015,
    ErrorCode::RCP_016,
    ErrorCode::RCP_017,
    ErrorCode::RCP_018,
    ErrorCode::RCP_019,
    ErrorCode::RCP_020,
    ErrorCode::RCP_021,
    ErrorCode::RCP_022,
    ErrorCode::RCP_023,
    ErrorCode::RCP_024,
    ErrorCode::RCP_025,
    ErrorCode::RCP_026,
    ErrorCode::RCP_027,
    ErrorCode::RCP_028,
    ErrorCode::RCP_029,
    ErrorCode::RCP_030,
    ErrorCode::RCP_031,
    ErrorCode::RCP_032,
    ErrorCode::RCP_033,
    ErrorCode::RCP_034,
    ErrorCode::RCP_035,
    ErrorCode::RCP_036,
    ErrorCode::RCP_050,
    ErrorCode::RCP_051,
    ErrorCode::RCP_052,
    ErrorCode::RCP_053,
    ErrorCode::RCP_054,
    ErrorCode::WEB_001,
    ErrorCode::WEB_002,
    ErrorCode::WEB_004,
    ErrorCode::WEB_005,
    ErrorCode::WEB_009,
    ErrorCode::WEB_011,
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

    /// The codes that tasks 2.1 adds, in text order.
    const ADDED_CODES: [ErrorCode; 36] = [
        ErrorCode::AUD_001,
        ErrorCode::AUD_002,
        ErrorCode::DB_001,
        ErrorCode::DB_002,
        ErrorCode::DB_003,
        ErrorCode::DB_004,
        ErrorCode::DB_005,
        ErrorCode::DB_006,
        ErrorCode::DB_007,
        ErrorCode::DB_008,
        ErrorCode::EXP_001,
        ErrorCode::EXP_002,
        ErrorCode::EXP_003,
        ErrorCode::EXP_004,
        ErrorCode::PLT_030,
        ErrorCode::PLT_031,
        ErrorCode::PLT_032,
        ErrorCode::PLT_033,
        ErrorCode::PLT_034,
        ErrorCode::PLT_035,
        ErrorCode::PLT_036,
        ErrorCode::PLT_037,
        ErrorCode::PLT_038,
        ErrorCode::RCP_030,
        ErrorCode::RCP_031,
        ErrorCode::RCP_032,
        ErrorCode::RCP_033,
        ErrorCode::RCP_034,
        ErrorCode::RCP_035,
        ErrorCode::RCP_036,
        ErrorCode::WEB_001,
        ErrorCode::WEB_002,
        ErrorCode::WEB_004,
        ErrorCode::WEB_005,
        ErrorCode::WEB_009,
        ErrorCode::WEB_011,
    ];

    #[test]
    fn added_codes_are_in_registry() {
        for code in ADDED_CODES {
            assert!(REGISTRY.contains(&code), "{code}");
        }
    }

    #[test]
    fn find_returns_the_registry_entry_of_a_text() {
        assert_eq!(ErrorCode::find("E-RCP-031"), Some(ErrorCode::RCP_031));
    }

    #[test]
    fn find_returns_none_for_an_unknown_text() {
        assert_eq!(ErrorCode::find("E-XYZ-001"), None);
    }

    #[test]
    fn find_returns_every_registry_entry_for_its_text() {
        for code in REGISTRY {
            assert_eq!(ErrorCode::find(code.as_str()), Some(*code));
        }
    }
}
