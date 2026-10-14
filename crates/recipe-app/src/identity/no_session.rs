//! The first statement of an operation that needs no session (design D6).

use crate::auth::{Operation, Requirement};
use crate::error::AppError;

/// Check that `operation` has the requirement `NoSession`.
///
/// The function solves one problem: `authorize` needs a user, but `Login` and `GetLoginForm` run
/// without one. Each of them calls this check first, so a catalog row with another requirement
/// does not run silently. The check fails closed with `E-WEB-002`.
///
/// # Errors
///
/// Returns `AppError::Forbidden` when the requirement of `operation` is not `NoSession`.
pub(super) fn require_no_session(operation: Operation) -> Result<(), AppError> {
    match operation.requirement() {
        Requirement::NoSession => Ok(()),
        Requirement::Permission(_) | Requirement::AnyOf(..) | Requirement::Authenticated => {
            Err(AppError::Forbidden { operation })
        }
    }
}
