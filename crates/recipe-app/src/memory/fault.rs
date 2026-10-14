//! Failure that a test injects into an in-memory adapter.

use crate::error::{
    LockFailure, PLC_TYPE_MEMBER_NAME_UNIQUE, PLC_TYPE_NAMED_UNIQUE, RECIPE_VERSION_ONE_LIVE_DRAFT,
    StorageError, UNIT_PARENT_EXISTS, UNIT_PATH_UNIQUE,
};

/// A storage failure that an in-memory adapter returns on demand.
///
/// The type solves one problem: `StorageError` is not `Clone` and holds a boxed
/// source, so a test cannot store one in a shared failure slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The failure `StorageError::Deadlock` with the cause `LockFailure::Deadlock` (`E-DB-001`).
    Deadlock,
    /// The failure `StorageError::Deadlock` with the cause `LockFailure::Serialization`
    /// (`E-DB-001`, SQLSTATE `40001`).
    Serialization,
    /// The failure `StorageError::StatementTimeout` (`E-DB-002`).
    StatementTimeout,
    /// The failure `StorageError::PoolTimeout` (`E-DB-004`).
    PoolTimeout,
    /// The failure `StorageError::Unavailable` (`E-DB-005`).
    Unavailable,
    /// The failure `StorageError::Duplicate` with the constraint `unit_path_unique`.
    DuplicateUnitPath,
    /// The failure `StorageError::MissingReference` with the constraint `unit_parent_exists`.
    MissingUnitParent,
    /// The failure `StorageError::Duplicate` with the constraint `plc_type_named_unique`.
    DuplicateTypeName,
    /// The failure `StorageError::Duplicate` with the constraint `recipe_version_one_live_draft`.
    DuplicateLiveDraft,
    /// The failure `StorageError::Duplicate` with the constraint `plc_type_member_name_unique`.
    DuplicateMemberName,
    /// The failure `StorageError::MissingReference` with a constraint that no command knows.
    UnknownMissingReference,
}

/// The place where a queued fault fires (`MemoryStore::fail_next`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultPoint {
    /// The call `begin` of the unit of work.
    Begin,
    /// The write of the audit entries inside `commit`.
    AppendAudit,
    /// The `finish` of the transaction, which is the commit of the database.
    Finish,
}

/// The constraint name of [`Fault::UnknownMissingReference`]. No command maps it.
const UNKNOWN_CONSTRAINT: &str = "unknown_foreign_key";

impl From<Fault> for StorageError {
    fn from(fault: Fault) -> Self {
        match fault {
            Fault::DuplicateUnitPath => Self::Duplicate {
                constraint: UNIT_PATH_UNIQUE.to_owned(),
            },
            Fault::MissingUnitParent => Self::MissingReference {
                constraint: UNIT_PARENT_EXISTS.to_owned(),
            },
            Fault::DuplicateTypeName => Self::Duplicate {
                constraint: PLC_TYPE_NAMED_UNIQUE.to_owned(),
            },
            Fault::DuplicateLiveDraft => Self::Duplicate {
                constraint: RECIPE_VERSION_ONE_LIVE_DRAFT.to_owned(),
            },
            Fault::DuplicateMemberName => Self::Duplicate {
                constraint: PLC_TYPE_MEMBER_NAME_UNIQUE.to_owned(),
            },
            Fault::UnknownMissingReference => Self::MissingReference {
                constraint: UNKNOWN_CONSTRAINT.to_owned(),
            },
            Fault::Deadlock => Self::Deadlock {
                cause: LockFailure::Deadlock,
            },
            Fault::Serialization => Self::Deadlock {
                cause: LockFailure::Serialization,
            },
            Fault::StatementTimeout => Self::StatementTimeout,
            Fault::PoolTimeout => Self::PoolTimeout,
            Fault::Unavailable => Self::Unavailable,
        }
    }
}
