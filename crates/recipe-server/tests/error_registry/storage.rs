//! Samples of `StorageError`.
//!
//! `Constraint` carries a trigger code, and one row of `reliability.md` section 1.2 names it for each
//! trigger code. This module gives one `Constraint` sample for each of these codes.

use recipe_app::error::{LockFailure, StorageError};
use recipe_domain::error_codes::ErrorCode;

use super::sample::Sample;

/// The codes that a `Constraint` value carries: the trigger codes and `E-DB-003`.
const CONSTRAINT_CODES: [ErrorCode; 19] = [
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
    ErrorCode::RCP_037,
    ErrorCode::AUD_003,
    ErrorCode::DB_003,
];

/// Return the name `Type::Variant` of a sample. The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &StorageError) -> &'static str {
    match error {
        StorageError::Duplicate { .. } => "StorageError::Duplicate",
        StorageError::MissingReference { .. } => "StorageError::MissingReference",
        StorageError::Constraint { .. } => "StorageError::Constraint",
        StorageError::Deadlock { .. } => "StorageError::Deadlock",
        StorageError::StatementTimeout => "StorageError::StatementTimeout",
        StorageError::PoolTimeout => "StorageError::PoolTimeout",
        StorageError::Unavailable => "StorageError::Unavailable",
        StorageError::Other { .. } => "StorageError::Other",
    }
}

/// Return one sample for each variant and one `Constraint` sample for each constraint code.
pub fn samples() -> Vec<Sample> {
    let mut errors = vec![
        StorageError::Duplicate {
            constraint: "sample_key".to_owned(),
        },
        StorageError::MissingReference {
            constraint: "sample_fkey".to_owned(),
        },
        StorageError::Deadlock {
            cause: LockFailure::Deadlock,
        },
        StorageError::StatementTimeout,
        StorageError::PoolTimeout,
        StorageError::Unavailable,
        StorageError::Other {
            sqlstate: None,
            source: Box::new(std::io::Error::other("sample")),
        },
    ];
    errors.extend(CONSTRAINT_CODES.map(|code| StorageError::Constraint {
        code,
        message: "sample".to_owned(),
    }));
    errors
        .iter()
        .map(|error| Sample {
            name: name_of(error),
            code: error.code(),
        })
        .collect()
}
