//! Assertions about the constraint that a statement violates.

use std::fmt::Debug;

/// SQLSTATE `unique_violation`. A unique index and a primary key raise it.
pub const UNIQUE_VIOLATION: &str = "23505";

/// SQLSTATE `foreign_key_violation`.
pub const FOREIGN_KEY_VIOLATION: &str = "23503";

/// Assert that a statement failed with `sqlstate` and that the database names `constraint`.
///
/// A CHECK, a primary key, and a foreign key report their constraint name. A unique index,
/// including a partial one, reports the name of the index.
pub fn assert_constraint<T: Debug>(
    result: Result<T, sqlx::Error>,
    sqlstate: &str,
    constraint: &str,
) {
    let error = result.expect_err("the constraint rejects the statement");
    let database = error
        .as_database_error()
        .expect("the failure is a database error");
    assert_eq!(database.code().as_deref(), Some(sqlstate), "{error}");
    assert_eq!(database.constraint(), Some(constraint), "{error}");
}
