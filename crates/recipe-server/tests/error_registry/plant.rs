//! Samples of `PlantError`, one for each variant.

use recipe_domain::plant::{
    CommMode, DerivedTypeReason, MachineCode, MemberName, PlantError, PlcAddress, PlcPlatform,
    TypeLimit, TypeName, UnitPath,
};
use recipe_domain::recipe::ReferenceNumber;

use super::sample::Sample;

/// Return the name `Type::Variant` of a sample. The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &PlantError) -> &'static str {
    match error {
        PlantError::UnitDepthExceeded { .. } => "PlantError::UnitDepthExceeded",
        PlantError::DuplicateMachineCode { .. } => "PlantError::DuplicateMachineCode",
        PlantError::ParentNotFound { .. } => "PlantError::ParentNotFound",
        PlantError::DuplicateUnitPath { .. } => "PlantError::DuplicateUnitPath",
        PlantError::LastMainUnit { .. } => "PlantError::LastMainUnit",
        PlantError::UnitArchived { .. } => "PlantError::UnitArchived",
        PlantError::MachineArchived { .. } => "PlantError::MachineArchived",
        PlantError::MachineHasDraft { .. } => "PlantError::MachineHasDraft",
        PlantError::UnitReferencedByDraft { .. } => "PlantError::UnitReferencedByDraft",
        PlantError::UnitReferencedByApproved { .. } => "PlantError::UnitReferencedByApproved",
        PlantError::TypeCycle { .. } => "PlantError::TypeCycle",
        PlantError::MemberTypeNotOnPlatform { .. } => "PlantError::MemberTypeNotOnPlatform",
        PlantError::DuplicateTypeName { .. } => "PlantError::DuplicateTypeName",
        PlantError::StructHasNoMembers { .. } => "PlantError::StructHasNoMembers",
        PlantError::TypeTooLarge { .. } => "PlantError::TypeTooLarge",
        PlantError::DuplicateMemberName { .. } => "PlantError::DuplicateMemberName",
        PlantError::DerivedTypeInvalid { .. } => "PlantError::DerivedTypeInvalid",
        PlantError::InvalidUnitPath { .. } => "PlantError::InvalidUnitPath",
        PlantError::MachineNotFound { .. } => "PlantError::MachineNotFound",
        PlantError::UnitNotFound { .. } => "PlantError::UnitNotFound",
        PlantError::ConnectionFieldNotOnPlatform { .. } => {
            "PlantError::ConnectionFieldNotOnPlatform"
        }
        PlantError::ConnectionFieldMissing { .. } => "PlantError::ConnectionFieldMissing",
        PlantError::DataBlockOutOfRange { .. } => "PlantError::DataBlockOutOfRange",
        PlantError::CommModeNotOnPlatform { .. } => "PlantError::CommModeNotOnPlatform",
        PlantError::AddressNotAmsNetId { .. } => "PlantError::AddressNotAmsNetId",
        PlantError::ConnectionNotConfigured { .. } => "PlantError::ConnectionNotConfigured",
        PlantError::ConnectionFieldNotInMode { .. } => "PlantError::ConnectionFieldNotInMode",
        PlantError::ConnectionRecipeMismatch { .. } => "PlantError::ConnectionRecipeMismatch",
        PlantError::DataBlockMissing { .. } => "PlantError::DataBlockMissing",
        PlantError::DataBlockNotOnPlatform { .. } => "PlantError::DataBlockNotOnPlatform",
        PlantError::RevisionUnchanged { .. } => "PlantError::RevisionUnchanged",
    }
}

fn code() -> MachineCode {
    MachineCode::parse("M1").unwrap()
}

fn path() -> UnitPath {
    UnitPath::parse("U1.2").unwrap()
}

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-1").unwrap()
}

fn type_name() -> TypeName {
    TypeName::parse("ST_Zone").unwrap()
}

fn member() -> MemberName {
    MemberName::parse("speed").unwrap()
}

/// Return the samples of the machine and unit variants.
fn unit_errors() -> Vec<PlantError> {
    vec![
        PlantError::UnitDepthExceeded {
            path: "U1.2.3.4.5.6".to_owned(),
        },
        PlantError::DuplicateMachineCode { code: code() },
        PlantError::ParentNotFound { path: path() },
        PlantError::DuplicateUnitPath { path: path() },
        PlantError::LastMainUnit {
            code: code(),
            path: path(),
        },
        PlantError::UnitArchived { path: path() },
        PlantError::MachineArchived { code: code() },
        PlantError::MachineHasDraft {
            code: code(),
            reference: reference(),
        },
        PlantError::UnitReferencedByDraft {
            path: path(),
            reference: reference(),
        },
        PlantError::UnitReferencedByApproved {
            path: path(),
            reference: reference(),
        },
        PlantError::InvalidUnitPath {
            text: "X1".to_owned(),
        },
        PlantError::MachineNotFound { code: code() },
        PlantError::UnitNotFound { path: path() },
    ]
}

/// Return the samples of the type and revision variants.
fn type_errors() -> Vec<PlantError> {
    vec![
        PlantError::TypeCycle {
            type_name: type_name(),
        },
        PlantError::MemberTypeNotOnPlatform {
            type_name: type_name(),
            member: member(),
            platform: PlcPlatform::S7_1500,
        },
        PlantError::DuplicateTypeName {
            type_name: type_name(),
        },
        PlantError::StructHasNoMembers {
            type_name: type_name(),
        },
        PlantError::TypeTooLarge {
            type_name: "ARRAY[1..9999] OF REAL".to_owned(),
            limit: TypeLimit::Leaves,
        },
        PlantError::DuplicateMemberName {
            type_name: type_name(),
            member: member(),
        },
        PlantError::DerivedTypeInvalid {
            type_expr: "ARRAY[".to_owned(),
            reason: DerivedTypeReason::Grammar,
        },
        PlantError::RevisionUnchanged {
            type_name: type_name(),
            revision: 2,
        },
    ]
}

/// Return the samples of the PLC connection and data block variants.
fn connection_errors() -> Vec<PlantError> {
    vec![
        PlantError::ConnectionFieldNotOnPlatform {
            code: code(),
            field: "rack",
            platform: PlcPlatform::TwinCat3,
        },
        PlantError::ConnectionFieldMissing {
            code: code(),
            field: "host",
        },
        PlantError::DataBlockOutOfRange {
            code: code(),
            number: 60_000,
            max: 32_767,
        },
        PlantError::CommModeNotOnPlatform {
            code: code(),
            mode: CommMode::Ads,
            platform: PlcPlatform::S7_1500,
        },
        PlantError::AddressNotAmsNetId {
            code: code(),
            address: PlcAddress::parse("PLC1").unwrap(),
        },
        PlantError::ConnectionNotConfigured { code: code() },
        PlantError::ConnectionFieldNotInMode {
            code: code(),
            field: "port",
            mode: CommMode::Ads,
        },
        PlantError::ConnectionRecipeMismatch {
            code: code(),
            reference: reference(),
        },
        PlantError::DataBlockMissing {
            code: code(),
            platform: PlcPlatform::S7_1500,
        },
        PlantError::DataBlockNotOnPlatform {
            code: code(),
            platform: PlcPlatform::TwinCat3,
        },
    ]
}

/// Return one sample for each variant of `PlantError`.
pub fn samples() -> Vec<Sample> {
    let groups = [unit_errors(), type_errors(), connection_errors()];
    groups
        .iter()
        .flatten()
        .map(|error| Sample {
            name: name_of(error),
            code: error.code(),
        })
        .collect()
}
