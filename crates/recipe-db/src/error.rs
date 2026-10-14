//! Map from `sqlx::Error` to `StorageError`.
//!
//! The crate has no public error type (`reliability.md` section 1.1). This module is the only
//! place that reads the failure of the driver. The map follows the order of `reliability.md`
//! section 1.1:
//!
//! 1. A database error whose message starts with a trigger code of the ranges `E-PLT-030..049` and
//!    `E-RCP-030..049`, or with `E-AUD-003`, becomes `Constraint` with that code, whatever its
//!    SQLSTATE. This check comes first.
//! 2. Then the SQLSTATE decides: `23505`, `23503`, `23514`, `40P01`, `40001`, `57014`, and `55P03`.
//!    `40P01`, `55P03`, and `40001` give `Deadlock` with the cause `Deadlock`, `LockTimeout`,
//!    and `Serialization`. The SQLSTATEs of a lost connection (class `08`, `57P01`, `57P02`, `57P03`, and `53300`) give
//!    `Unavailable`.
//! 3. Then the kind of the driver error decides: a pool timeout and the connection failures.
//! 4. Every other error becomes `Other` and keeps the original error as its source.

use std::fmt;

use recipe_app::error::{LockFailure, StorageError};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::RecipeVersionId;

use crate::trigger_messages::user_message;

/// SQLSTATE `lock_not_available`. PostgreSQL raises it when `lock_timeout` ends a wait.
const LOCK_NOT_AVAILABLE: &str = "55P03";

/// SQLSTATE `check_violation`. A check constraint and every database trigger raise it.
const CHECK_VIOLATION: &str = "23514";

/// SQLSTATE `unique_violation`.
const UNIQUE_VIOLATION: &str = "23505";

/// SQLSTATE `foreign_key_violation`.
const FOREIGN_KEY_VIOLATION: &str = "23503";

/// SQLSTATE `deadlock_detected`. A deadlock is a bug in the lock order (ADR-0015).
const DEADLOCK_DETECTED: &str = "40P01";

/// SQLSTATE `serialization_failure`. `serializable_once_more` retries it once.
const SERIALIZATION_FAILURE: &str = "40001";

/// SQLSTATE `query_canceled`. PostgreSQL raises it when `statement_timeout` ends a statement.
const QUERY_CANCELED: &str = "57014";

/// The prefix of the SQLSTATE class `08`, connection exception. Every code of the class counts.
const CONNECTION_EXCEPTION_CLASS: &str = "08";

/// The SQLSTATEs outside class `08` that mean the server is gone or refuses a connection:
/// `admin_shutdown`, `crash_shutdown`, `cannot_connect_now`, and `too_many_connections`.
const CONNECTION_LOSS_STATES: [&str; 4] = ["57P01", "57P02", "57P03", "53300"];

/// The numbers of the trigger range of each context, `030` to `049` (`reliability.md` section 1.2).
const TRIGGER_NUMBERS: std::ops::RangeInclusive<u16> = 30..=49;

/// The cause of a `StorageError::Other` that a defect of the stored data or of a use case gives.
///
/// The enum solves one problem: the three defects need a cause that names the failure, and one
/// private type replaces three hand-written error types. No variant holds a secret.
#[derive(Debug, thiserror::Error)]
enum RowError {
    /// A stored row breaks a rule of the domain.
    #[error("the row of the table {table} is corrupt: {reason}")]
    Corrupt {
        /// The table of the row.
        table: &'static str,
        /// The rule that the row breaks.
        reason: String,
    },
    /// A write finds no row, although the use case locked it first.
    #[error("the row of the table {table} does not exist")]
    Missing {
        /// The table of the row.
        table: &'static str,
    },
    /// A status write follows no lock of the same transaction, or its row version is stale.
    #[error("the status write of the version {} has no lock with this row version", .version.as_uuid())]
    StatusWriteWithoutLock {
        /// The version that the write names.
        version: RecipeVersionId,
    },
}

impl From<RowError> for StorageError {
    fn from(error: RowError) -> Self {
        Self::Other {
            sqlstate: None,
            source: Box::new(error),
        }
    }
}

/// Build the storage error of a stored row that the domain constructors reject (`E-DB-008`).
pub(crate) fn corrupt(table: &'static str, reason: impl fmt::Display) -> StorageError {
    RowError::Corrupt {
        table,
        reason: reason.to_string(),
    }
    .into()
}

/// Build the storage error of a write that finds no row. The use case locks the row first, so
/// this is a defect (`E-DB-008`).
pub(crate) fn missing_row(table: &'static str) -> StorageError {
    RowError::Missing { table }.into()
}

/// Build the error of a status write that follows no lock of the same transaction, or that
/// changes no row (`E-DB-008`, ADR-0030). The write changed nothing.
pub(crate) fn without_lock(version: RecipeVersionId) -> StorageError {
    RowError::StatusWriteWithoutLock { version }.into()
}

/// Tell whether a SQLSTATE means that the connection or the server is lost.
fn is_connection_loss(state: &str) -> bool {
    state.starts_with(CONNECTION_EXCEPTION_CLASS) || CONNECTION_LOSS_STATES.contains(&state)
}

/// Return the SQLSTATE of a database error, when the database gave one.
fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code.into_owned())
}

/// Tell whether the database ended a lock wait with `lock_timeout`.
pub(crate) fn is_lock_timeout(error: &sqlx::Error) -> bool {
    sqlstate(error).as_deref() == Some(LOCK_NOT_AVAILABLE)
}

/// Return the trigger code that starts a database message, when the code lies in a trigger range.
///
/// The trigger ranges are `E-PLT-030..049` and `E-RCP-030..049`. Every code of the registry
/// in these ranges has a user message. `E-AUD-003` of the trigger `security_event_guard` is the
/// one trigger code outside these ranges (ADR-0035).
fn trigger_code(message: &str) -> Option<ErrorCode> {
    let first = message.split_whitespace().next()?;
    let code = ErrorCode::find(first)?;
    if code == ErrorCode::AUD_003 {
        return Some(code);
    }
    let text = code.as_str();
    let is_trigger_prefix = matches!(text.get(..6), Some("E-PLT-" | "E-RCP-"));
    let number = text.get(6..)?.parse::<u16>().ok()?;
    (is_trigger_prefix && TRIGGER_NUMBERS.contains(&number)).then_some(code)
}

/// Build `StorageError::Constraint` with the user message of a registered code.
fn constraint(code: ErrorCode) -> Option<StorageError> {
    let message = user_message(code)?;
    Some(StorageError::Constraint {
        code,
        message: message.to_owned(),
    })
}

/// Build the error of a write that the adapter refuses before it reaches the database.
///
/// The error equals the one that a check constraint without a trigger code gives (`E-DB-003`).
/// The function solves one problem: a port that accepts only some rows must refuse the others
/// with the same error on every adapter.
pub(crate) fn check_violation() -> StorageError {
    constraint(ErrorCode::DB_003).unwrap_or_else(|| StorageError::Constraint {
        code: ErrorCode::DB_003,
        message: String::new(),
    })
}

/// Map a message that starts with a trigger code. The SQLSTATE does not matter.
fn trigger_constraint(error: &sqlx::Error) -> Option<StorageError> {
    let code = trigger_code(error.as_database_error()?.message())?;
    constraint(code)
}

/// Build the storage error of a lock failure. All three causes share the code `E-DB-001`.
const fn lock_failure(cause: LockFailure) -> StorageError {
    StorageError::Deadlock { cause }
}

/// Map a database error by its SQLSTATE.
///
/// A duplicate or a missing reference needs the name of the constraint. Without a name, the
/// function returns `None`, so the error becomes `Other`.
fn by_sqlstate(error: &sqlx::Error) -> Option<StorageError> {
    let database_error = error.as_database_error()?;
    let constraint_name = || database_error.constraint().map(str::to_owned);
    match sqlstate(error)?.as_str() {
        UNIQUE_VIOLATION => {
            constraint_name().map(|constraint| StorageError::Duplicate { constraint })
        }
        FOREIGN_KEY_VIOLATION => {
            constraint_name().map(|constraint| StorageError::MissingReference { constraint })
        }
        CHECK_VIOLATION => constraint(ErrorCode::DB_003),
        DEADLOCK_DETECTED => Some(lock_failure(LockFailure::Deadlock)),
        LOCK_NOT_AVAILABLE => Some(lock_failure(LockFailure::LockTimeout)),
        SERIALIZATION_FAILURE => Some(lock_failure(LockFailure::Serialization)),
        QUERY_CANCELED => Some(StorageError::StatementTimeout),
        state if is_connection_loss(state) => Some(StorageError::Unavailable),
        _ => None,
    }
}

/// Convert a driver error into the `StorageError` of the ports.
///
/// The function follows the order of `reliability.md` section 1.1. The original error stays the
/// `source` of `StorageError::Other`. The function logs nothing.
pub(crate) fn map_error(error: sqlx::Error) -> StorageError {
    if let Some(mapped) = trigger_constraint(&error).or_else(|| by_sqlstate(&error)) {
        return mapped;
    }
    match error {
        sqlx::Error::Io(_)
        | sqlx::Error::Tls(_)
        | sqlx::Error::PoolClosed
        | sqlx::Error::WorkerCrashed => StorageError::Unavailable,
        sqlx::Error::PoolTimedOut => StorageError::PoolTimeout,
        other => StorageError::Other {
            sqlstate: sqlstate(&other),
            source: Box::new(other),
        },
    }
}

#[cfg(test)]
mod tests;
