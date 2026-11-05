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
mod tests;
