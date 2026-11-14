//! Samples of `RecipeError`, one for each variant.

use recipe_domain::identity::Login;
use recipe_domain::plant::{LeafPath, PlcPlatform, S7Address, S7Width, TypeName};
use recipe_domain::recipe::{
    Budget, Expected, LangCode, ParameterKey, ParameterTag, PrecisionField, RebindTargetReason,
    RecipeError, ReferenceNumber, VersionStatus,
};

use super::sample::Sample;

/// Return the name `Type::Variant` of a sample. The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &RecipeError) -> &'static str {
    match error {
        RecipeError::DuplicateReference { .. } => "RecipeError::DuplicateReference",
        RecipeError::InvalidLangCode { .. } => "RecipeError::InvalidLangCode",
        RecipeError::RecipeDeleted { .. } => "RecipeError::RecipeDeleted",
        RecipeError::RecipeHasApprovedVersion { .. } => "RecipeError::RecipeHasApprovedVersion",
        RecipeError::RecipeHasRetiredVersion { .. } => "RecipeError::RecipeHasRetiredVersion",
        RecipeError::ApproverEditedDraft { .. } => "RecipeError::ApproverEditedDraft",
        RecipeError::VersionNotDraft { .. } => "RecipeError::VersionNotDraft",
        RecipeError::VersionNotApproved { .. } => "RecipeError::VersionNotApproved",
        RecipeError::DraftAlreadyExists { .. } => "RecipeError::DraftAlreadyExists",
        RecipeError::SizeBudgetExceeded { .. } => "RecipeError::SizeBudgetExceeded",
        RecipeError::RecipeNotDeleted { .. } => "RecipeError::RecipeNotDeleted",
        RecipeError::VersionNotEditable { .. } => "RecipeError::VersionNotEditable",
        RecipeError::DuplicateTag { .. } => "RecipeError::DuplicateTag",
        RecipeError::S7AddressRequired { .. } => "RecipeError::S7AddressRequired",
        RecipeError::S7AddressOverlap { .. } => "RecipeError::S7AddressOverlap",
        RecipeError::S7AddressMisaligned { .. } => "RecipeError::S7AddressMisaligned",
        RecipeError::S7AddressNotAllowed { .. } => "RecipeError::S7AddressNotAllowed",
        RecipeError::S7AddressWidthMismatch { .. } => "RecipeError::S7AddressWidthMismatch",
        RecipeError::PrecisionExceeded { .. } => "RecipeError::PrecisionExceeded",
        RecipeError::S7AddressPastBlockEnd { .. } => "RecipeError::S7AddressPastBlockEnd",
        RecipeError::S7DataBlockOutOfRange { .. } => "RecipeError::S7DataBlockOutOfRange",
        RecipeError::ValueOutOfLimit { .. } => "RecipeError::ValueOutOfLimit",
        RecipeError::ValueShapeMismatch { .. } => "RecipeError::ValueShapeMismatch",
        RecipeError::LimitOutsideTypeRange { .. } => "RecipeError::LimitOutsideTypeRange",
        RecipeError::LeafPathNotFound { .. } => "RecipeError::LeafPathNotFound",
        RecipeError::MinGreaterThanMax { .. } => "RecipeError::MinGreaterThanMax",
        RecipeError::LimitNotApplicable { .. } => "RecipeError::LimitNotApplicable",
        RecipeError::TypeNotOnPlatform { .. } => "RecipeError::TypeNotOnPlatform",
        RecipeError::DraftHasNoParameters { .. } => "RecipeError::DraftHasNoParameters",
        RecipeError::LimitMissing { .. } => "RecipeError::LimitMissing",
        RecipeError::TypeRevisionOutdated { .. } => "RecipeError::TypeRevisionOutdated",
        RecipeError::ValidationFailed { .. } => "RecipeError::ValidationFailed",
        RecipeError::TranslationNotFound { .. } => "RecipeError::TranslationNotFound",
        RecipeError::ParameterNotFound { .. } => "RecipeError::ParameterNotFound",
        RecipeError::RebindNeedsValue { .. } => "RecipeError::RebindNeedsValue",
        RecipeError::RebindDropsLimit { .. } => "RecipeError::RebindDropsLimit",
        RecipeError::RebindTargetInvalid { .. } => "RecipeError::RebindTargetInvalid",
    }
}

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-1").unwrap()
}

fn tag() -> ParameterTag {
    ParameterTag::parse("SPEED").unwrap()
}

fn other_tag() -> ParameterTag {
    ParameterTag::parse("TEMP").unwrap()
}

fn leaf() -> LeafPath {
    LeafPath::parse(".value").unwrap()
}

fn address() -> S7Address {
    S7Address::parse("DB1.DBD0").unwrap()
}

fn type_name() -> TypeName {
    TypeName::parse("ST_Zone").unwrap()
}

fn lang() -> LangCode {
    LangCode::parse("pl").unwrap()
}

/// Return the samples of the recipe and version lifecycle variants.
fn lifecycle_errors() -> Vec<RecipeError> {
    vec![
        RecipeError::DuplicateReference {
            reference: reference(),
        },
        RecipeError::InvalidLangCode {
            lang: "x".to_owned(),
        },
        RecipeError::RecipeDeleted {
            reference: reference(),
        },
        RecipeError::RecipeHasApprovedVersion {
            reference: reference(),
        },
        RecipeError::RecipeHasRetiredVersion {
            reference: reference(),
        },
        RecipeError::ApproverEditedDraft {
            reference: reference(),
            number: 1,
            login: Login::parse("anowak").unwrap(),
        },
        RecipeError::VersionNotDraft {
            reference: reference(),
            number: 1,
        },
        RecipeError::VersionNotApproved {
            reference: reference(),
            number: 1,
        },
        RecipeError::DraftAlreadyExists {
            reference: reference(),
            number: 1,
        },
        RecipeError::SizeBudgetExceeded {
            budget: Budget::Parameters,
            limit: 1,
            actual: 2,
        },
        RecipeError::RecipeNotDeleted {
            reference: reference(),
        },
        RecipeError::VersionNotEditable {
            reference: reference(),
            number: 1,
            status: VersionStatus::Approved,
        },
    ]
}

/// Return the samples of the tag and S7 address variants.
fn address_errors() -> Vec<RecipeError> {
    vec![
        RecipeError::DuplicateTag {
            tag: tag(),
            number: 1,
        },
        RecipeError::S7AddressRequired { tag: tag() },
        RecipeError::S7AddressOverlap {
            tag: tag(),
            other_tag: other_tag(),
            address: address(),
        },
        RecipeError::S7AddressMisaligned {
            tag: tag(),
            address: address(),
        },
        RecipeError::S7AddressNotAllowed {
            tag: tag(),
            platform: PlcPlatform::TwinCat3,
        },
        RecipeError::S7AddressWidthMismatch {
            tag: tag(),
            address: address(),
            width: S7Width::D,
        },
        RecipeError::S7AddressPastBlockEnd {
            tag: tag(),
            address: address(),
            size: 4,
        },
        RecipeError::S7DataBlockOutOfRange {
            tag: tag(),
            address: address(),
            max: 1,
        },
    ]
}

/// Return the samples of the value and limit variants.
fn value_errors() -> Vec<RecipeError> {
    vec![
        RecipeError::PrecisionExceeded {
            tag: tag(),
            leaf: leaf(),
            field: PrecisionField::Value,
            type_name: "REAL".to_owned(),
            max_digits: 7,
        },
        RecipeError::ValueOutOfLimit {
            tag: tag(),
            leaf: leaf(),
            value: "9".to_owned(),
            min: "0".to_owned(),
            max: "5".to_owned(),
        },
        RecipeError::ValueShapeMismatch {
            tag: tag(),
            leaf: leaf(),
            expected: Expected::Scalar,
        },
        RecipeError::LimitOutsideTypeRange {
            tag: tag(),
            leaf: leaf(),
        },
        RecipeError::LeafPathNotFound {
            tag: tag(),
            leaf: leaf(),
        },
        RecipeError::MinGreaterThanMax {
            tag: tag(),
            leaf: leaf(),
            min: "5".to_owned(),
            max: "0".to_owned(),
        },
        RecipeError::LimitNotApplicable {
            tag: tag(),
            leaf: leaf(),
        },
        RecipeError::LimitMissing {
            tag: tag(),
            leaf: leaf(),
        },
    ]
}

/// Return the samples of the type, rebind, and lookup variants.
fn draft_errors() -> Vec<RecipeError> {
    vec![
        RecipeError::TypeNotOnPlatform {
            tag: tag(),
            type_name: "LREAL".to_owned(),
            platform: PlcPlatform::S7_1500,
        },
        RecipeError::DraftHasNoParameters {
            reference: reference(),
            number: 1,
        },
        RecipeError::TypeRevisionOutdated {
            tag: tag(),
            type_name: type_name(),
            current: 1,
            newest: 2,
        },
        RecipeError::ValidationFailed { items: Vec::new() },
        RecipeError::TranslationNotFound {
            reference: reference(),
            number: 1,
            tag: tag(),
            lang: lang(),
        },
        RecipeError::ParameterNotFound {
            reference: reference(),
            number: 1,
            key: ParameterKey::parse("00000000-0000-0000-0000-000000000001").unwrap(),
        },
        RecipeError::RebindNeedsValue {
            tag: tag(),
            leaf: leaf(),
        },
        RecipeError::RebindDropsLimit {
            tag: tag(),
            leaf: leaf(),
        },
        RecipeError::RebindTargetInvalid {
            type_name: "ST_Zone".to_owned(),
            reason: RebindTargetReason::NotNewer,
        },
    ]
}

/// Return one sample for each variant of `RecipeError`.
pub fn samples() -> Vec<Sample> {
    let groups = [
        lifecycle_errors(),
        address_errors(),
        value_errors(),
        draft_errors(),
    ];
    groups
        .iter()
        .flatten()
        .map(|error| Sample {
            name: name_of(error),
            code: error.code(),
        })
        .collect()
}
