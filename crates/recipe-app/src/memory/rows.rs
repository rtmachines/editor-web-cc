//! Rows of the in-memory plant model and the storage errors that the adapters share.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{
    MachineCode, MachineId, PlantError, PlcPlatform, PlcType, UnitId, UnitPath,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;

use crate::error::StorageError;

/// A row of the table `machine` (`data-model.md` section 3.1).
#[derive(Clone, Debug)]
pub(crate) struct MachineRow {
    pub(crate) id: MachineId,
    pub(crate) code: MachineCode,
    pub(crate) name: Text,
    pub(crate) platform: PlcPlatform,
    pub(crate) archived_at: Option<Timestamp>,
    pub(crate) created_at: Timestamp,
    pub(crate) updated_at: Timestamp,
}

/// A row of the table `unit` (`data-model.md` section 3.2).
#[derive(Clone, Debug)]
pub(crate) struct UnitRow {
    pub(crate) id: UnitId,
    pub(crate) machine: MachineId,
    pub(crate) path: UnitPath,
    pub(crate) name: Text,
    pub(crate) archived_at: Option<Timestamp>,
    pub(crate) created_at: Timestamp,
    pub(crate) updated_at: Timestamp,
}

/// A row of the table `plc_type` (`data-model.md` section 3.3) with its member rows.
#[derive(Clone, Debug)]
pub(crate) struct TypeRow {
    pub(crate) plc_type: PlcType,
    pub(crate) created_at: Timestamp,
}

/// The cause of the storage error for an `UPDATE` that finds no row.
#[derive(Debug, thiserror::Error)]
#[error("the row of the table {table} does not exist")]
struct MissingRow {
    table: &'static str,
}

/// A write that finds no row. The use case locks the row first, so this is a defect.
pub(crate) fn missing_row(table: &'static str) -> StorageError {
    StorageError::Other {
        sqlstate: None,
        source: Box::new(MissingRow { table }),
    }
}

/// Stored rows that break a rule of the domain mean corrupt storage.
pub(crate) fn corrupt(error: PlantError) -> StorageError {
    StorageError::Other {
        sqlstate: None,
        source: Box::new(error),
    }
}

/// The cause of the storage error for stored rows that break a rule of the domain.
#[derive(Debug, thiserror::Error)]
#[error("{reason}")]
struct CorruptRows {
    reason: String,
}

/// Stored rows of the recipe tables break a rule of the domain, so the storage is corrupt.
pub(crate) fn corrupt_rows(reason: &dyn std::fmt::Display) -> StorageError {
    StorageError::Other {
        sqlstate: None,
        source: Box::new(CorruptRows {
            reason: reason.to_string(),
        }),
    }
}

/// The trigger `enforce_version_mutable` rejects a write to a version that is not a live draft.
pub(crate) fn version_not_live_draft() -> StorageError {
    StorageError::Constraint {
        code: ErrorCode::RCP_031,
        message: "The version is not a live draft.".to_owned(),
    }
}

/// The trigger `recipe_version_transition` rejects a change of a version row.
pub(crate) fn illegal_transition() -> StorageError {
    StorageError::Constraint {
        code: ErrorCode::RCP_033,
        message: "The version status or a frozen column of the version cannot change in this way."
            .to_owned(),
    }
}

/// A unique constraint rejects the write.
pub(crate) fn duplicate(constraint: &str) -> StorageError {
    StorageError::Duplicate {
        constraint: constraint.to_owned(),
    }
}

/// A foreign key rejects the write.
pub(crate) fn missing_reference(constraint: &str) -> StorageError {
    StorageError::MissingReference {
        constraint: constraint.to_owned(),
    }
}

/// A check constraint without a trigger code rejects the write (`E-DB-003`).
pub(crate) fn check_violation() -> StorageError {
    StorageError::Constraint {
        code: ErrorCode::DB_003,
        message: "The change violates a database rule.".to_owned(),
    }
}
