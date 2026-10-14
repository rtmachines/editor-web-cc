//! The one more run of a `SERIALIZABLE` command (design D8, ADR-0035 c).

use std::future::Future;

use crate::error::{AppError, LockFailure, StorageError};

/// Tell whether the error is a serialization failure (`40001`).
///
/// The failure can come directly from a statement or from `finish`, as `AppError::Storage`. It
/// can also come from the audit insert, as `AppError::AuditWriteFailed`.
fn is_serialization_failure(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Storage(StorageError::Deadlock {
            cause: LockFailure::Serialization
        }) | AppError::AuditWriteFailed {
            cause: StorageError::Deadlock {
                cause: LockFailure::Serialization
            }
        }
    )
}

/// Run `body` and run it once more after a serialization failure.
///
/// The function solves one problem: `SetUserRole`, `DeactivateUser`, and
/// `bootstrap_administrator` run `SERIALIZABLE`, and PostgreSQL may abort one of two
/// concurrent transactions with `40001`. The database promises that a retry can succeed. A
/// deadlock or a lock timeout never runs again, because the lock order of ADR-0015 prevents the
/// first and the second needs a person to look. A second serialization failure, in either
/// form, becomes `AppError::Storage` with the code `E-DB-001`.
///
/// `body` builds a new future for each run. The future must open its own transaction.
///
/// # Errors
///
/// Returns the error of the last run.
pub async fn serializable_once_more<T, F, Fut>(mut body: F) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    match body().await {
        Err(error) if is_serialization_failure(&error) => {}
        other => return other,
    }
    match body().await {
        Err(error) if is_serialization_failure(&error) => {
            Err(AppError::Storage(StorageError::Deadlock {
                cause: LockFailure::Serialization,
            }))
        }
        other => other,
    }
}

#[cfg(test)]
mod tests;
