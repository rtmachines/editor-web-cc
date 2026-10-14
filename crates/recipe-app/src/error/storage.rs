//! Storage and audit errors (`reliability.md` sections 1.1 and 1.2).
//!
//! The `Display` text is the user message of `reliability.md` section 1.2.
//! It ends before the sentence "Reference: {request_id}.", because only the request knows the reference (design D4).

use recipe_domain::error_codes::ErrorCode;

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
mod tests {
    use std::error::Error as _;
    use std::io;

    use recipe_domain::error_codes::ErrorCode;

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
    fn chain_broken_has_code_and_message() {
        let error = AuditError::ChainBroken { chain_seq: 42 };
        assert_eq!(error.code(), ErrorCode::AUD_002);
        assert_eq!(
            error.to_string(),
            "Audit entry 42 does not match the hash chain. Contact the administrator."
        );
    }
}
