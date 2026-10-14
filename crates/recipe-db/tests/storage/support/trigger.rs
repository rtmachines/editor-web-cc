//! Assertions about the errors of triggers and constraints.

use std::fmt::Debug;

/// SQLSTATE `check_violation`. Every trigger of `data-model.md` section 4 raises it.
pub const CHECK_VIOLATION: &str = "23514";

/// SQLSTATE `insufficient_privilege`. PostgreSQL raises it before any trigger runs.
pub const INSUFFICIENT_PRIVILEGE: &str = "42501";

/// Assert that a statement failed with SQLSTATE `42501`.
pub fn assert_privilege_denied<T: Debug>(result: Result<T, sqlx::Error>) {
    let error = result.expect_err("the privilege check rejects the statement");
    let database = error
        .as_database_error()
        .expect("the failure is a database error");
    assert_eq!(
        database.code().as_deref(),
        Some(INSUFFICIENT_PRIVILEGE),
        "{error}"
    );
}

/// Assert that a statement failed in a trigger with SQLSTATE `23514` and a message that starts
/// with the error code.
pub fn assert_trigger<T: Debug>(result: Result<T, sqlx::Error>, code: &str) {
    let error = result.expect_err("the trigger rejects the statement");
    let database = error
        .as_database_error()
        .expect("the failure is a database error");
    assert_eq!(database.code().as_deref(), Some(CHECK_VIOLATION), "{error}");
    assert!(
        database.message().starts_with(code),
        "expected {code} at the start of {:?}",
        database.message()
    );
}
