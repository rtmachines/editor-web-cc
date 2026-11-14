//! Samples of `AppError`.
//!
//! A delegate variant passes the code of its inner error. The registry test checks that code with the
//! module of the inner type, so a delegate has no sample here.

use recipe_app::auth::Operation;
use recipe_app::error::{AppError, Missing, StorageError, TaskStage, TimeoutStage};
use recipe_domain::plant::MachineCode;
use recipe_domain::recipe::ReferenceNumber;

use super::sample::Sample;

/// Return the name `Type::Variant` of an own variant, or `None` for a delegate.
///
/// The `match` has no wildcard, so a new variant stops the build.
fn name_of(error: &AppError) -> Option<&'static str> {
    match error {
        AppError::Plant(_)
        | AppError::Plants(_)
        | AppError::Recipe(_)
        | AppError::Recipes(_)
        | AppError::Export(_)
        | AppError::Identity(_)
        | AppError::Storage(_)
        | AppError::Audit(_) => None,
        AppError::Forbidden { .. } => Some("AppError::Forbidden"),
        AppError::Unauthenticated => Some("AppError::Unauthenticated"),
        AppError::NotFound { .. } => Some("AppError::NotFound"),
        AppError::ConcurrentEdit { .. } => Some("AppError::ConcurrentEdit"),
        AppError::ConnectionConcurrentEdit { .. } => Some("AppError::ConnectionConcurrentEdit"),
        AppError::SecretUnreadable { .. } => Some("AppError::SecretUnreadable"),
        AppError::Timeout { .. } => Some("AppError::Timeout"),
        AppError::TaskFailed { .. } => Some("AppError::TaskFailed"),
        AppError::AuditWriteFailed { .. } => Some("AppError::AuditWriteFailed"),
    }
}

fn machine() -> MachineCode {
    MachineCode::parse("M1").unwrap()
}

/// Return one sample for each own variant of `AppError`.
pub fn samples() -> Vec<Sample> {
    let errors = [
        AppError::Forbidden {
            operation: Operation::ListMachines,
        },
        AppError::Unauthenticated,
        AppError::NotFound {
            entity: Missing::Machine(machine()),
        },
        AppError::ConcurrentEdit {
            reference: ReferenceNumber::parse("RCP-1").unwrap(),
            number: 1,
            expected: 1,
            actual: 2,
        },
        AppError::ConnectionConcurrentEdit {
            code: machine(),
            expected: 1,
            actual: 2,
        },
        AppError::SecretUnreadable { code: machine() },
        AppError::Timeout {
            stage: TimeoutStage::Request,
        },
        AppError::TaskFailed {
            stage: TaskStage::Token,
        },
        AppError::AuditWriteFailed {
            cause: StorageError::Unavailable,
        },
    ];
    errors
        .iter()
        .filter_map(|error| {
            name_of(error).map(|name| Sample {
                name,
                code: error.code(),
            })
        })
        .collect()
}
