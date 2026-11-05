use std::error::Error as _;
use std::io;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::{RecipeError, ReferenceNumber};

use super::*;

fn deadlock(cause: LockFailure) -> StorageError {
    StorageError::Deadlock { cause }
}

fn other() -> StorageError {
    StorageError::Other {
        sqlstate: Some("XX000".to_owned()),
        source: Box::new(io::Error::other("secret internal text")),
    }
}

#[test]
fn each_variant_has_its_code() {
    let cases = [
        (
            StorageError::Duplicate {
                constraint: "x".to_owned(),
            },
            ErrorCode::DB_006,
        ),
        (
            StorageError::MissingReference {
                constraint: "x".to_owned(),
            },
            ErrorCode::DB_007,
        ),
        (deadlock(LockFailure::Deadlock), ErrorCode::DB_001),
        (deadlock(LockFailure::LockTimeout), ErrorCode::DB_001),
        (deadlock(LockFailure::Serialization), ErrorCode::DB_001),
        (StorageError::StatementTimeout, ErrorCode::DB_002),
        (StorageError::PoolTimeout, ErrorCode::DB_004),
        (StorageError::Unavailable, ErrorCode::DB_005),
        (other(), ErrorCode::DB_008),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code, "{error:?}");
    }
}

#[test]
fn constraint_keeps_its_own_code_and_message() {
    let error = StorageError::Constraint {
        code: ErrorCode::RCP_031,
        message: "The version is not a live draft.".to_owned(),
    };
    assert_eq!(error.code(), ErrorCode::RCP_031);
    assert_eq!(error.to_string(), "The version is not a live draft.");
}

#[test]
fn display_gives_the_user_message_without_the_reference() {
    let texts = [
        (
            StorageError::Duplicate {
                constraint: "c".to_owned(),
            },
            "The change conflicts with an existing entry.",
        ),
        (
            StorageError::MissingReference {
                constraint: "c".to_owned(),
            },
            "The change refers to an entry that does not exist.",
        ),
        (
            deadlock(LockFailure::Deadlock),
            "Another operation holds a lock on the same data. Try again.",
        ),
        (
            deadlock(LockFailure::LockTimeout),
            "Another operation holds a lock on the same data. Try again.",
        ),
        (
            deadlock(LockFailure::Serialization),
            "Another operation holds a lock on the same data. Try again.",
        ),
        (
            StorageError::StatementTimeout,
            "The operation took too long and was stopped. Try again.",
        ),
        (
            StorageError::PoolTimeout,
            "No database connection is free. Try again.",
        ),
        (
            StorageError::Unavailable,
            "The database does not answer. Try again.",
        ),
        (
            other(),
            "An unexpected database error occurred. Contact the administrator.",
        ),
    ];
    for (error, text) in texts {
        assert_eq!(error.to_string(), text);
    }
}

#[test]
fn other_hides_source_and_sqlstate_in_display() {
    let shown = other().to_string();
    assert!(!shown.contains("secret internal text"));
    assert!(!shown.contains("XX000"));
}

#[test]
fn other_keeps_the_source_reachable() {
    let error = other();
    let inner = error.source().unwrap();
    assert_eq!(inner.to_string(), "secret internal text");
}

#[test]
fn invariant_is_an_other_error_with_the_message_as_source() {
    let error = StorageError::invariant("an adapter returned too few rows");
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert_eq!(
        error.to_string(),
        "An unexpected database error occurred. Contact the administrator."
    );
    assert!(matches!(&error, StorageError::Other { sqlstate: None, .. }));
    let inner = error.source().unwrap();
    assert_eq!(inner.to_string(), "an adapter returned too few rows");
}

#[test]
fn broken_stored_parameter_names_the_key_and_the_code_in_the_chain() {
    let key = ParameterKey::from(uuid::Uuid::nil());
    let fault = StructureFault::from(RecipeError::RecipeDeleted {
        reference: ReferenceNumber::parse("R-7").unwrap(),
    });

    let error = StorageError::broken_stored_parameter(key, fault);

    assert_eq!(error.code(), ErrorCode::DB_008);
    assert_eq!(
        error.to_string(),
        "An unexpected database error occurred. Contact the administrator."
    );
    let chain: Vec<String> = std::iter::successors(error.source(), |&cause| cause.source())
        .map(ToString::to_string)
        .collect();
    let first = chain.first().unwrap();
    assert!(first.contains(&key.to_string()), "{chain:?}");
    assert!(first.contains(ErrorCode::RCP_012.as_str()), "{chain:?}");
    assert!(chain.iter().any(|text| text.contains("R-7")), "{chain:?}");
}

#[test]
fn chain_broken_has_code_and_message() {
    let error = AuditError::ChainBroken { chain_seq: 42 };
    assert_eq!(error.code(), ErrorCode::AUD_002);
    assert_eq!(
        error.to_string(),
        "Audit entry 42 does not match the hash chain. Contact the administrator."
    );
}
