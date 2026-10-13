//! The error type of the Plant Model context.

use crate::error_codes::ErrorCode;

use crate::recipe::ReferenceNumber;

use std::fmt;

use super::limits::{MAX_DEPTH, MAX_LEAVES, MAX_MEMBERS};
use super::{MachineCode, MemberName, PlcPlatform, StringLengthReason, TypeName, UnitPath};

/// The size limit that a type breaks (`E-PLT-018`).
///
/// The enum is the `limit` field of [`PlantError::TypeTooLarge`].
/// It is not an error type and has no code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeLimit {
    /// The number of scalar leaves of one parameter.
    Leaves,
    /// The number of JSON levels of one value.
    Depth,
    /// The number of members of one struct.
    Members,
}

impl fmt::Display for TypeLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leaves => write!(f, "{MAX_LEAVES} leaves"),
            Self::Depth => write!(f, "{MAX_DEPTH} levels"),
            Self::Members => write!(f, "{MAX_MEMBERS} members"),
        }
    }
}

/// Reason why a type is not valid (`E-PLT-013`, ADR-0025).
///
/// The enum is the `reason` field of [`PlantError::DerivedTypeInvalid`].
/// It is not an error type and has no code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivedTypeReason {
    /// The text does not match the type grammar.
    Grammar,
    /// The struct or the revision does not exist on the machine.
    UnknownStruct,
    /// The type does not exist on the platform of the machine.
    NotOnPlatform,
    /// A bound, a dimension, or a length is outside its range.
    OutOfRange,
    /// The stored type row does not match its kind.
    ///
    /// Only a corrupt stored row or a bug gives this reason. P2.4 maps it
    /// to a storage error (design D5, D6).
    RowShape,
}

impl fmt::Display for DerivedTypeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Grammar => "the text does not match the type grammar",
            Self::UnknownStruct => "the struct or the revision does not exist on the machine",
            Self::NotOnPlatform => "the type does not exist on the platform of the machine",
            Self::OutOfRange => "a bound, a dimension, or a length is outside its range",
            Self::RowShape => "the stored type row does not match its kind",
        })
    }
}

impl From<StringLengthReason> for DerivedTypeReason {
    fn from(reason: StringLengthReason) -> Self {
        match reason {
            StringLengthReason::NotOnPlatform => Self::NotOnPlatform,
            StringLengthReason::OutOfRange => Self::OutOfRange,
        }
    }
}

/// Error of the Plant Model context (`reliability.md` section 1.1).
///
/// The enum has one variant for each code in the range `E-PLT-001` to
/// `E-PLT-049` that the value objects return. Later tasks add more variants.
#[derive(Clone, Debug, thiserror::Error)]
pub enum PlantError {
    /// A unit path has more than five segments (`E-PLT-002`).
    #[error("Unit path {path} has more than 5 levels. Use at most 5 levels.")]
    UnitDepthExceeded {
        /// The rejected input text.
        path: String,
    },
    /// A machine code is already in use (`E-PLT-001`).
    #[error("Machine code {code} is already in use. Use a different code.")]
    DuplicateMachineCode {
        /// The code that exists.
        code: MachineCode,
    },
    /// The parent unit of a unit path does not exist (`E-PLT-003`).
    #[error("The parent unit of {path} does not exist. Create the parent unit first.")]
    ParentNotFound {
        /// The path of the unit that has no parent.
        path: UnitPath,
    },
    /// A unit path already exists on the machine (`E-PLT-004`).
    #[error("Unit {path} already exists on this machine.")]
    DuplicateUnitPath {
        /// The path that exists twice.
        path: UnitPath,
    },
    /// A change would leave the machine without a live main unit (`E-PLT-008`).
    #[error("Unit {path} is the last main unit of machine {code}. A machine needs one main unit.")]
    LastMainUnit {
        /// The code of the machine.
        code: MachineCode,
        /// The path of the last main unit.
        path: UnitPath,
    },
    /// A unit is archived and the command needs a live unit (`E-PLT-009`).
    #[error("Unit {path} is archived. Select a live unit.")]
    UnitArchived {
        /// The path of the archived unit.
        path: UnitPath,
    },
    /// A machine is archived and the command needs a live machine (`E-PLT-010`).
    #[error("Machine {code} is archived.")]
    MachineArchived {
        /// The code of the archived machine.
        code: MachineCode,
    },
    /// A machine has a live draft and cannot be archived (`E-PLT-011`).
    #[error(
        "Machine {code} has a draft in recipe {reference}. Approve or discard the draft first."
    )]
    MachineHasDraft {
        /// The code of the machine.
        code: MachineCode,
        /// The reference number of the recipe with the draft.
        reference: ReferenceNumber,
    },
    /// The live draft of a recipe uses the unit (`E-PLT-012`).
    #[error(
        "Unit {path} is used by the draft of recipe {reference}. Move or delete those parameters first."
    )]
    UnitReferencedByDraft {
        /// The path of the unit in use.
        path: UnitPath,
        /// The reference number of the recipe with the draft.
        reference: ReferenceNumber,
    },
    /// The approved version of a recipe uses the unit (`E-PLT-014`).
    #[error(
        "Unit {path} is used by the approved version of recipe {reference}. An Administrator can retire the version."
    )]
    UnitReferencedByApproved {
        /// The path of the unit in use.
        path: UnitPath,
        /// The reference number of the recipe with the approved version.
        reference: ReferenceNumber,
    },
    /// A struct has itself as a member (`E-PLT-005`).
    #[error("Type {type_name} contains itself. Delete the circular member.")]
    TypeCycle {
        /// The struct that contains itself.
        type_name: TypeName,
    },
    /// A member type does not exist on a platform of the struct (`E-PLT-006`).
    #[error("Member {member} of {type_name} uses a type that does not exist on {platform}.")]
    MemberTypeNotOnPlatform {
        /// The struct with the member.
        type_name: TypeName,
        /// The member with the type.
        member: MemberName,
        /// The platform without the type.
        platform: PlcPlatform,
    },
    /// A struct name exists on the machine (`E-PLT-015`).
    #[error("Type {type_name} already exists on this machine. Create a new revision instead.")]
    DuplicateTypeName {
        /// The name that exists.
        type_name: TypeName,
    },
    /// A struct has no member (`E-PLT-016`).
    #[error("Type {type_name} has no members. Add at least one member.")]
    StructHasNoMembers {
        /// The struct without members.
        type_name: TypeName,
    },
    /// A type is above a size limit (`E-PLT-018`).
    #[error(
        "Type {type_name} is larger than the limit {limit}. Use fewer elements, fewer members, or fewer levels."
    )]
    TypeTooLarge {
        /// The type as text. An array has no name.
        type_name: String,
        /// The limit that the type breaks.
        limit: TypeLimit,
    },
    /// Two members of a struct have one name (`E-PLT-021`).
    #[error(
        "Type {type_name} has two members with the name {member}. Give each member a different name."
    )]
    DuplicateMemberName {
        /// The struct with the two members.
        type_name: TypeName,
        /// The name that exists twice.
        member: MemberName,
    },
    /// A type is not valid (`E-PLT-013`).
    #[error(
        "The type {type_expr} is not valid: {reason}. Use a type such as INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL."
    )]
    DerivedTypeInvalid {
        /// The type as text.
        type_expr: String,
        /// Why the type is not valid.
        reason: DerivedTypeReason,
    },
    /// A text is not a unit path (`E-PLT-017`).
    #[error(
        "{text} is not a unit path. A unit path is U and 1 to 5 numbers from 1 to 64, with a dot between two numbers. Example: U1.2."
    )]
    InvalidUnitPath {
        /// The rejected input text.
        text: String,
    },
    /// A machine does not exist (`E-PLT-019`).
    #[error("Machine {code} does not exist. Select an existing machine.")]
    MachineNotFound {
        /// The code that names no machine.
        code: MachineCode,
    },
    /// A unit does not exist on the machine (`E-PLT-020`).
    #[error("Unit {path} does not exist on this machine. Select an existing unit.")]
    UnitNotFound {
        /// The path that names no unit.
        path: UnitPath,
    },
}

impl PlantError {
    /// Return the stable error code of this error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::UnitDepthExceeded { .. } => ErrorCode::PLT_002,
            Self::DuplicateMachineCode { .. } => ErrorCode::PLT_001,
            Self::ParentNotFound { .. } => ErrorCode::PLT_003,
            Self::DuplicateUnitPath { .. } => ErrorCode::PLT_004,
            Self::LastMainUnit { .. } => ErrorCode::PLT_008,
            Self::UnitArchived { .. } => ErrorCode::PLT_009,
            Self::MachineArchived { .. } => ErrorCode::PLT_010,
            Self::MachineHasDraft { .. } => ErrorCode::PLT_011,

            Self::UnitReferencedByDraft { .. } => ErrorCode::PLT_012,
            Self::UnitReferencedByApproved { .. } => ErrorCode::PLT_014,
            Self::TypeCycle { .. } => ErrorCode::PLT_005,
            Self::MemberTypeNotOnPlatform { .. } => ErrorCode::PLT_006,
            Self::DerivedTypeInvalid { .. } => ErrorCode::PLT_013,
            Self::DuplicateTypeName { .. } => ErrorCode::PLT_015,
            Self::StructHasNoMembers { .. } => ErrorCode::PLT_016,
            Self::TypeTooLarge { .. } => ErrorCode::PLT_018,
            Self::DuplicateMemberName { .. } => ErrorCode::PLT_021,
            Self::InvalidUnitPath { .. } => ErrorCode::PLT_017,
            Self::MachineNotFound { .. } => ErrorCode::PLT_019,
            Self::UnitNotFound { .. } => ErrorCode::PLT_020,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_codes::REGISTRY;

    #[test]
    fn unit_depth_exceeded_has_code_plt_002_and_the_section_1_2_text() {
        let error = PlantError::UnitDepthExceeded {
            path: "U1.2.3.4.5.6".to_owned(),
        };
        assert_eq!(error.code().as_str(), "E-PLT-002");
        assert_eq!(
            error.to_string(),
            "Unit path U1.2.3.4.5.6 has more than 5 levels. Use at most 5 levels."
        );
    }

    #[test]
    fn invalid_unit_path_has_code_plt_017_and_the_section_1_2_text() {
        let error = PlantError::InvalidUnitPath {
            text: "U65".to_owned(),
        };
        assert_eq!(error.code().as_str(), "E-PLT-017");
        assert_eq!(
            error.to_string(),
            "U65 is not a unit path. A unit path is U and 1 to 5 numbers from 1 to 64, with a dot between two numbers. Example: U1.2."
        );
    }

    fn path(text: &str) -> UnitPath {
        UnitPath::parse(text).unwrap()
    }

    #[test]
    fn machine_tree_errors_have_their_code_registry_row_and_text() {
        let code = MachineCode::parse("M7").unwrap();
        let cases = [
            (
                PlantError::ParentNotFound { path: path("U1.2") },
                ErrorCode::PLT_003,
                "E-PLT-003",
                "The parent unit of U1.2 does not exist. Create the parent unit first.",
            ),
            (
                PlantError::DuplicateUnitPath { path: path("U1.2") },
                ErrorCode::PLT_004,
                "E-PLT-004",
                "Unit U1.2 already exists on this machine.",
            ),
            (
                PlantError::LastMainUnit {
                    code,
                    path: path("U1"),
                },
                ErrorCode::PLT_008,
                "E-PLT-008",
                "Unit U1 is the last main unit of machine M7. A machine needs one main unit.",
            ),
            (
                PlantError::UnitArchived { path: path("U1.2") },
                ErrorCode::PLT_009,
                "E-PLT-009",
                "Unit U1.2 is archived. Select a live unit.",
            ),
            (
                PlantError::MachineArchived { code },
                ErrorCode::PLT_010,
                "E-PLT-010",
                "Machine M7 is archived.",
            ),
        ];
        for (error, expected, text, message) in cases {
            assert_eq!(error.code(), expected);
            assert_eq!(error.code().as_str(), text);
            assert!(REGISTRY.contains(&expected), "{text}");
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn machine_edit_errors_have_their_code_registry_row_and_text() {
        let code = MachineCode::parse("M7").unwrap();
        let reference = ReferenceNumber::parse("r-100").unwrap();
        let cases = [
            (
                PlantError::DuplicateMachineCode { code },
                ErrorCode::PLT_001,
                "E-PLT-001",
                "Machine code M7 is already in use. Use a different code.",
            ),
            (
                PlantError::MachineHasDraft { code, reference },
                ErrorCode::PLT_011,
                "E-PLT-011",
                "Machine M7 has a draft in recipe R-100. Approve or discard the draft first.",
            ),
            (
                PlantError::MachineNotFound { code },
                ErrorCode::PLT_019,
                "E-PLT-019",
                "Machine M7 does not exist. Select an existing machine.",
            ),
            (
                PlantError::UnitNotFound { path: path("U1.2") },
                ErrorCode::PLT_020,
                "E-PLT-020",
                "Unit U1.2 does not exist on this machine. Select an existing unit.",
            ),
        ];
        for (error, expected, text, message) in cases {
            assert_eq!(error.code(), expected);
            assert_eq!(error.code().as_str(), text);
            assert!(REGISTRY.contains(&expected), "{text}");
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn unit_usage_errors_have_their_code_registry_row_and_text() {
        let reference = ReferenceNumber::parse("r-100").unwrap();
        let cases = [
            (
                PlantError::UnitReferencedByDraft {
                    path: path("U1.2"),
                    reference: reference.clone(),
                },
                ErrorCode::PLT_012,
                "E-PLT-012",
                "Unit U1.2 is used by the draft of recipe R-100. Move or delete those parameters first.",
            ),
            (
                PlantError::UnitReferencedByApproved {
                    path: path("U1.2"),
                    reference,
                },
                ErrorCode::PLT_014,
                "E-PLT-014",
                "Unit U1.2 is used by the approved version of recipe R-100. An Administrator can retire the version.",
            ),
        ];
        for (error, expected, text, message) in cases {
            assert_eq!(error.code(), expected);
            assert_eq!(error.code().as_str(), text);
            assert!(REGISTRY.contains(&expected), "{text}");
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn derived_type_invalid_has_code_plt_013_and_the_section_1_2_text() {
        let error = PlantError::DerivedTypeInvalid {
            type_expr: "STRING(255)".to_owned(),
            reason: DerivedTypeReason::OutOfRange,
        };
        assert_eq!(error.code(), ErrorCode::PLT_013);
        assert_eq!(error.code().as_str(), "E-PLT-013");
        assert!(REGISTRY.contains(&ErrorCode::PLT_013));
        assert_eq!(
            error.to_string(),
            "The type STRING(255) is not valid: a bound, a dimension, or a length is outside its range. Use a type such as INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL."
        );
    }

    #[test]
    fn type_limits_show_the_numbers_of_section_7_11() {
        assert_eq!(TypeLimit::Leaves.to_string(), "16384 leaves");
        assert_eq!(TypeLimit::Depth.to_string(), "32 levels");
        assert_eq!(TypeLimit::Members.to_string(), "1024 members");
    }

    #[test]
    fn type_errors_have_their_code_registry_row_and_text() {
        let name = TypeName::parse("ST_Zone").unwrap();
        let member = MemberName::parse("speed").unwrap();
        let cases = [
            (
                PlantError::TypeCycle {
                    type_name: name.clone(),
                },
                ErrorCode::PLT_005,
                "E-PLT-005",
                "Type ST_Zone contains itself. Delete the circular member.",
            ),
            (
                PlantError::MemberTypeNotOnPlatform {
                    type_name: name.clone(),
                    member: member.clone(),
                    platform: PlcPlatform::S7_300,
                },
                ErrorCode::PLT_006,
                "E-PLT-006",
                "Member speed of ST_Zone uses a type that does not exist on S7_300.",
            ),
            (
                PlantError::DuplicateTypeName {
                    type_name: name.clone(),
                },
                ErrorCode::PLT_015,
                "E-PLT-015",
                "Type ST_Zone already exists on this machine. Create a new revision instead.",
            ),
            (
                PlantError::StructHasNoMembers {
                    type_name: name.clone(),
                },
                ErrorCode::PLT_016,
                "E-PLT-016",
                "Type ST_Zone has no members. Add at least one member.",
            ),
            (
                PlantError::TypeTooLarge {
                    type_name: "ST_Zone".to_owned(),
                    limit: TypeLimit::Members,
                },
                ErrorCode::PLT_018,
                "E-PLT-018",
                "Type ST_Zone is larger than the limit 1024 members. Use fewer elements, fewer members, or fewer levels.",
            ),
            (
                PlantError::DuplicateMemberName {
                    type_name: name,
                    member,
                },
                ErrorCode::PLT_021,
                "E-PLT-021",
                "Type ST_Zone has two members with the name speed. Give each member a different name.",
            ),
        ];
        for (error, expected, text, message) in cases {
            assert_eq!(error.code(), expected);
            assert_eq!(error.code().as_str(), text);
            assert!(REGISTRY.contains(&expected), "{text}");
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn derived_type_reasons_have_their_phrase() {
        let cases = [
            (
                DerivedTypeReason::Grammar,
                "the text does not match the type grammar",
            ),
            (
                DerivedTypeReason::UnknownStruct,
                "the struct or the revision does not exist on the machine",
            ),
            (
                DerivedTypeReason::NotOnPlatform,
                "the type does not exist on the platform of the machine",
            ),
            (
                DerivedTypeReason::OutOfRange,
                "a bound, a dimension, or a length is outside its range",
            ),
            (
                DerivedTypeReason::RowShape,
                "the stored type row does not match its kind",
            ),
        ];
        for (reason, text) in cases {
            assert_eq!(reason.to_string(), text);
        }
    }

    #[test]
    fn registry_contains_both_plant_codes() {
        assert!(REGISTRY.contains(&ErrorCode::PLT_002));
        assert!(REGISTRY.contains(&ErrorCode::PLT_017));
    }

    /// Return the E-PLT rows of `reliability.md` section 1.2 as (code, variant, template).
    fn plant_doc_rows() -> Vec<(String, String, String)> {
        const DOC: &str = include_str!("../../../../docs/architecture/reliability.md");
        DOC.lines()
            .filter(|line| line.starts_with("| `E-PLT-"))
            .filter_map(|line| {
                let cells: Vec<&str> = line.split('|').map(str::trim).collect();
                let code = cells.get(1)?.trim_matches('`');
                let owner = cells.get(2)?.trim_matches('`');
                let variant = owner
                    .strip_prefix("PlantError::")?
                    .split([' ', '{', '('])
                    .next()?;
                let template = cells.get(4)?;
                Some((code.to_owned(), variant.to_owned(), (*template).to_owned()))
            })
            .collect()
    }

    /// Replace each `{field}` of a template with the text of its value.
    fn fill(template: &str, fields: &[(&str, String)]) -> String {
        fields
            .iter()
            .fold(template.to_owned(), |text, (name, value)| {
                text.replace(&format!("{{{name}}}"), value)
            })
    }

    fn machine_code() -> MachineCode {
        MachineCode::parse("M7").unwrap()
    }

    fn reference() -> ReferenceNumber {
        ReferenceNumber::parse("r-100").unwrap()
    }

    fn zone() -> TypeName {
        TypeName::parse("ST_Zone").unwrap()
    }

    fn speed() -> MemberName {
        MemberName::parse("speed").unwrap()
    }

    /// One sample of each `PlantError` variant, with the display text of each field.
    fn plant_samples() -> Vec<(PlantError, Vec<(&'static str, String)>)> {
        vec![
            (
                PlantError::UnitDepthExceeded {
                    path: "U1.2.3.4.5.6".to_owned(),
                },
                vec![("path", "U1.2.3.4.5.6".to_owned())],
            ),
            (
                PlantError::DuplicateMachineCode {
                    code: machine_code(),
                },
                vec![("code", machine_code().to_string())],
            ),
            (
                PlantError::ParentNotFound { path: path("U1.2") },
                vec![("path", path("U1.2").to_string())],
            ),
            (
                PlantError::DuplicateUnitPath { path: path("U1.2") },
                vec![("path", path("U1.2").to_string())],
            ),
            (
                PlantError::TypeCycle { type_name: zone() },
                vec![("type_name", zone().to_string())],
            ),
            (
                PlantError::MemberTypeNotOnPlatform {
                    type_name: zone(),
                    member: speed(),
                    platform: PlcPlatform::S7_300,
                },
                vec![
                    ("type_name", zone().to_string()),
                    ("member", speed().to_string()),
                    ("platform", PlcPlatform::S7_300.to_string()),
                ],
            ),
            (
                PlantError::LastMainUnit {
                    code: machine_code(),
                    path: path("U1"),
                },
                vec![
                    ("code", machine_code().to_string()),
                    ("path", path("U1").to_string()),
                ],
            ),
            (
                PlantError::UnitArchived { path: path("U1.2") },
                vec![("path", path("U1.2").to_string())],
            ),
            (
                PlantError::MachineArchived {
                    code: machine_code(),
                },
                vec![("code", machine_code().to_string())],
            ),
            (
                PlantError::MachineHasDraft {
                    code: machine_code(),
                    reference: reference(),
                },
                vec![
                    ("code", machine_code().to_string()),
                    ("reference", reference().to_string()),
                ],
            ),
            (
                PlantError::UnitReferencedByDraft {
                    path: path("U1.2"),
                    reference: reference(),
                },
                vec![
                    ("path", path("U1.2").to_string()),
                    ("reference", reference().to_string()),
                ],
            ),
            (
                PlantError::UnitReferencedByApproved {
                    path: path("U1.2"),
                    reference: reference(),
                },
                vec![
                    ("path", path("U1.2").to_string()),
                    ("reference", reference().to_string()),
                ],
            ),
            (
                PlantError::DuplicateTypeName { type_name: zone() },
                vec![("type_name", zone().to_string())],
            ),
            (
                PlantError::StructHasNoMembers { type_name: zone() },
                vec![("type_name", zone().to_string())],
            ),
            (
                PlantError::TypeTooLarge {
                    type_name: "ST_Zone".to_owned(),
                    limit: TypeLimit::Members,
                },
                vec![
                    ("type_name", "ST_Zone".to_owned()),
                    ("limit", TypeLimit::Members.to_string()),
                ],
            ),
            (
                PlantError::DuplicateMemberName {
                    type_name: zone(),
                    member: speed(),
                },
                vec![
                    ("type_name", zone().to_string()),
                    ("member", speed().to_string()),
                ],
            ),
            (
                PlantError::DerivedTypeInvalid {
                    type_expr: "STRING(255)".to_owned(),
                    reason: DerivedTypeReason::OutOfRange,
                },
                vec![
                    ("type_expr", "STRING(255)".to_owned()),
                    ("reason", DerivedTypeReason::OutOfRange.to_string()),
                ],
            ),
            (
                PlantError::InvalidUnitPath {
                    text: "U65".to_owned(),
                },
                vec![("text", "U65".to_owned())],
            ),
            (
                PlantError::MachineNotFound {
                    code: machine_code(),
                },
                vec![("code", machine_code().to_string())],
            ),
            (
                PlantError::UnitNotFound { path: path("U1.2") },
                vec![("path", path("U1.2").to_string())],
            ),
        ]
    }

    #[test]
    fn registry_matches_reliability() {
        let rows = plant_doc_rows();
        let expected: Vec<String> = (1..=6)
            .chain(8..=21)
            .map(|number| format!("E-PLT-{number:03}"))
            .collect();

        // Assertion 1: the doc has the 20 domain rows, in order.
        let doc_codes: Vec<String> = rows.iter().map(|row| row.0.clone()).collect();
        assert_eq!(doc_codes, expected);

        // Assertion 2: REGISTRY holds the same E-PLT codes.
        let registry: Vec<String> = REGISTRY
            .iter()
            .map(|code| code.as_str().to_owned())
            .filter(|code| code.starts_with("E-PLT-"))
            .collect();
        assert_eq!(registry, expected);

        // Assertions 3 and 4: each variant has one sample that matches its doc row.
        let samples = plant_samples();
        let mut sample_codes: Vec<String> = Vec::new();
        for (error, fields) in &samples {
            let code = error.code().as_str().to_owned();
            let row = rows
                .iter()
                .find(|row| row.0 == code)
                .expect("the code of a sample has a doc row");
            let debug = format!("{error:?}");
            let debug_variant = debug.split([' ', '{', '(']).next().unwrap_or("");
            assert_eq!(debug_variant, row.1.as_str(), "{code}");
            let text = fill(&row.2, fields);
            assert!(!text.contains('{'), "{code} has an unfilled field");
            assert_eq!(error.to_string(), text, "{code}");
            sample_codes.push(code);
        }
        assert_eq!(samples.len(), rows.len());
        for code in &expected {
            let count = sample_codes.iter().filter(|c| *c == code).count();
            assert_eq!(count, 1, "{code} has {count} samples");
        }
    }
}
