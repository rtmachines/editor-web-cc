use super::*;
use crate::error_codes::REGISTRY;
use crate::plant::UnitPath;
use crate::recipe::ReferenceNumber;

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-100").unwrap()
}

fn serde_error() -> serde_json::Error {
    serde_json::from_str::<serde_json::Value>("{").unwrap_err()
}

#[test]
fn no_approved_version_has_code_exp_001() {
    let error = ExportError::NoApprovedVersion {
        reference: reference(),
    };
    assert_eq!(error.code(), ErrorCode::EXP_001);
}

#[test]
fn content_hash_mismatch_has_code_exp_002() {
    let error = ExportError::ContentHashMismatch {
        reference: reference(),
        number: 3,
    };
    assert_eq!(error.code(), ErrorCode::EXP_002);
}

#[test]
fn version_not_exportable_has_code_exp_003() {
    let error = ExportError::VersionNotExportable {
        reference: reference(),
        number: 3,
    };
    assert_eq!(error.code(), ErrorCode::EXP_003);
}

#[test]
fn serialize_has_code_exp_004() {
    let error = ExportError::from(serde_error());
    assert_eq!(error.code(), ErrorCode::EXP_004);
}

#[test]
fn every_code_of_the_export_context_is_in_registry() {
    let codes = [
        ExportError::NoApprovedVersion {
            reference: reference(),
        }
        .code(),
        ExportError::ContentHashMismatch {
            reference: reference(),
            number: 3,
        }
        .code(),
        ExportError::VersionNotExportable {
            reference: reference(),
            number: 3,
        }
        .code(),
        ExportError::from(serde_error()).code(),
    ];
    for code in codes {
        assert!(REGISTRY.contains(&code), "{code} is not in REGISTRY");
    }
}

#[test]
fn no_approved_version_message_names_the_recipe() {
    let error = ExportError::NoApprovedVersion {
        reference: reference(),
    };
    assert_eq!(error.to_string(), "Recipe R-100 has no approved version.");
}

#[test]
fn version_not_exportable_message_names_the_version() {
    let error = ExportError::VersionNotExportable {
        reference: reference(),
        number: 3,
    };
    assert_eq!(
        error.to_string(),
        "Version 3 of recipe R-100 is a draft or was discarded. Only approved and retired versions can be exported."
    );
}

#[test]
fn content_hash_mismatch_message_refuses_the_export() {
    let error = ExportError::ContentHashMismatch {
        reference: reference(),
        number: 3,
    };
    assert_eq!(
        error.to_string(),
        "Version 3 of recipe R-100 does not match its stored hash. The export is refused. Contact the administrator."
    );
}

#[test]
fn serialize_message_hides_the_inner_serde_text() {
    let error = ExportError::from(serde_error());
    assert_eq!(
        error.to_string(),
        "The server cannot create the export document. Contact the administrator."
    );
}

#[test]
fn serde_json_error_converts_into_serialize_variant() {
    let error = ExportError::from(serde_error());
    assert!(matches!(error, ExportError::Serialize(_)));
}

fn empty_scope() -> ExportError {
    ExportError::EmptyScope {
        reference: reference(),
        number: 3,
        path: UnitPath::parse("U1.2").unwrap(),
    }
}

#[test]
fn empty_scope_has_code_exp_005() {
    assert_eq!(empty_scope().code(), ErrorCode::EXP_005);
}

#[test]
fn empty_scope_is_in_registry() {
    assert!(REGISTRY.contains(&ErrorCode::EXP_005));
}

#[test]
fn empty_scope_message_names_the_version_and_the_unit() {
    assert_eq!(
        empty_scope().to_string(),
        "Version 3 of recipe R-100 has no parameter in unit U1.2 or below it."
    );
}
