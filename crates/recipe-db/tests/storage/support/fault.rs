//! The injected fault of the atomic tests and the two assertions about where it hit.
//!
//! A test trigger with the body `FAULT` fails one statement with SQLSTATE `P0001`. A fault in an
//! audit insert becomes `E-AUD-001`. A fault in any other statement stays a storage error.

use recipe_app::error::{AppError, StorageError};

/// A trigger body that fails the statement with SQLSTATE `P0001`.
pub const FAULT: &str = "RAISE EXCEPTION 'injected fault';";

/// Assert that the command failed in a statement of the command and not in the audit insert.
pub fn assert_statement_fault(error: &AppError) {
    assert!(
        matches!(error, AppError::Storage(StorageError::Other { sqlstate, .. })
            if sqlstate.as_deref() == Some("P0001")),
        "{error:?}"
    );
}

/// Assert that the command failed in the audit insert (`E-AUD-001`).
pub fn assert_audit_fault(error: &AppError) {
    assert!(
        matches!(error, AppError::AuditWriteFailed { .. }),
        "{error:?}"
    );
}
