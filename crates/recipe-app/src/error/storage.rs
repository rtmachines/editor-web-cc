//! Storage and audit errors (`reliability.md` sections 1.1 and 1.2).
//!
//! The `Display` text is the user message of `reliability.md` section 1.2.
//! It ends before the sentence "Reference: {request_id}.", because only the request knows the reference (design D4).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::{DraftError, ParameterKey, StructureFault, ValueFault};

/// The kind of failure behind `StorageError::Deadlock` (design D16, ADR-0035 c).
///
/// The type solves one problem: the retry of a `SERIALIZABLE` command must tell a serialization
/// failure from a deadlock or a lock timeout, and all three share the code `E-DB-001`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockFailure {
    /// The database detected a deadlock (SQLSTATE `40P01`). It is a bug in the lock order.
    Deadlock,
    /// A lock wait hit `lock_timeout` (SQLSTATE `55P03`).
    LockTimeout,
    /// A `SERIALIZABLE` transaction failed to serialize (SQLSTATE `40001`).
    Serialization,
}

/// Error of the storage ports.
///
/// The type solves one problem: `recipe-app` must not hold a `sqlx` type.
/// The adapter in `recipe-db` maps each database failure into one variant by SQLSTATE.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// A unique constraint rejected the write (`E-DB-006` when the command does not know the name).
    #[error("The change conflicts with an existing entry.")]
    Duplicate {
        /// The name of the violated constraint.
        constraint: String,
    },
    /// A foreign key rejected the write (`E-DB-007` when the command does not know the name).
    #[error("The change refers to an entry that does not exist.")]
    MissingReference {
        /// The name of the violated constraint.
        constraint: String,
    },
    /// A trigger or a check constraint rejected the write.
    #[error("{message}")]
    Constraint {
        /// The code of the trigger, or `E-DB-003` for a check constraint without a trigger code.
        code: ErrorCode,
        /// The user message of the code.
        message: String,
    },
    /// The database detected a deadlock or a lock wait that hit its limit.
    #[error("Another operation holds a lock on the same data. Try again.")]
    Deadlock {
        /// The kind of lock failure. Only the retry helper and the log read it.
        cause: LockFailure,
    },
    /// A statement ran longer than its limit.
    #[error("The operation took too long and was stopped. Try again.")]
    StatementTimeout,
    /// The pool had no free connection in time.
    #[error("No database connection is free. Try again.")]
    PoolTimeout,
    /// The database does not answer.
    #[error("The database does not answer. Try again.")]
    Unavailable,
    /// Any other database error. `Display` hides the details.
    #[error("An unexpected database error occurred. Contact the administrator.")]
    Other {
        /// The SQLSTATE of the failure, when the database gave one.
        sqlstate: Option<String>,
        /// The original error. Only the log reads it.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// The cause of the storage error for a port that breaks its contract.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct BrokenContract(&'static str);

/// The cause of a stored value that does not match its type.
///
/// The type solves one problem: the one log line must name the broken parameter. It holds the key
/// and the fault, and it never holds the value text.
#[derive(Debug, thiserror::Error)]
#[error("the stored value of the parameter {key} does not match its type")]
struct BrokenValue {
    key: ParameterKey,
    #[source]
    fault: FaultSource,
}

/// The `Display` text of a `ValueFault`, as an error source.
///
/// `ValueFault` is not an `Error`, so the chain of the log needs this wrapper.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct FaultSource(ValueFault);

/// The cause of a stored parameter that breaks the earlier rule set (ADR-0118 (d)).
///
/// The type solves one problem: the one log line must name the broken parameter and the code of
/// the fault. It holds the key, the code, and the first error of the domain as its source.
#[derive(Debug, thiserror::Error)]
#[error("the stored parameter {key} breaks the earlier rule set with {code}")]
struct BrokenStoredParameter {
    key: ParameterKey,
    code: ErrorCode,
    #[source]
    error: DraftError,
}

impl StorageError {
    /// Build the error of a broken port contract, for example a result list that is too short.
    ///
    /// The function solves one problem: a use case must stop and commit nothing when an adapter
    /// breaks an invariant. The code is `E-DB-008`, and `Display` hides the message.
    #[must_use]
    pub(crate) fn invariant(message: &'static str) -> Self {
        Self::Other {
            sqlstate: None,
            source: Box::new(BrokenContract(message)),
        }
    }

    /// Build the error of a stored value that does not match its type, a defect of the storage.
    ///
    /// The function solves one problem: the one log line must find the broken row. The source
    /// chain names the parameter key and the fault. The code is `E-DB-008`, and `Display` hides
    /// the details. The message never holds the value text.
    #[must_use]
    pub(crate) fn broken_value(key: ParameterKey, fault: ValueFault) -> Self {
        Self::Other {
            sqlstate: None,
            source: Box::new(BrokenValue {
                key,
                fault: FaultSource(fault),
            }),
        }
    }

    /// Build the error of a stored parameter that breaks the earlier rule set, a defect of the storage.
    ///
    /// The function solves one problem: the one log line must find the broken parameter. The
    /// source chain names the parameter key, the code of the fault, and the first error of the
    /// domain. The code is `E-DB-008`, and `Display` hides the details.
    #[must_use]
    pub(crate) fn broken_stored_parameter(key: ParameterKey, fault: StructureFault) -> Self {
        Self::Other {
            sqlstate: None,
            source: Box::new(BrokenStoredParameter {
                key,
                code: fault.code(),
                error: fault.into_error(),
            }),
        }
    }

    /// Return the stable code of the error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Duplicate { .. } => ErrorCode::DB_006,
            Self::MissingReference { .. } => ErrorCode::DB_007,
            Self::Constraint { code, .. } => *code,
            Self::Deadlock { .. } => ErrorCode::DB_001,
            Self::StatementTimeout => ErrorCode::DB_002,
            Self::PoolTimeout => ErrorCode::DB_004,
            Self::Unavailable => ErrorCode::DB_005,
            Self::Other { .. } => ErrorCode::DB_008,
        }
    }
}

/// Error of the audit chain check.
///
/// The job `audit_chain_check` raises it. No user request returns it.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    /// An audit entry does not match the hash chain (`E-AUD-002`).
    #[error("Audit entry {chain_seq} does not match the hash chain. Contact the administrator.")]
    ChainBroken {
        /// The chain sequence number of the first broken entry.
        chain_seq: i64,
    },
}

impl AuditError {
    /// Return the stable code of the error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::ChainBroken { .. } => ErrorCode::AUD_002,
        }
    }
}

#[cfg(test)]
mod tests;
