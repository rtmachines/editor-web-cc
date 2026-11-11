//! Port of the PLC connection record of a machine (`data-model.md` sections 3.18 and 6, ADR-0154).

use async_trait::async_trait;
use recipe_domain::plant::{MachineId, PasswordState, PlcConnection, PlcConnectionId};

use super::{Locked, RowImage, SealedSecret};
use crate::error::StorageError;

/// A stored connection record without its password.
///
/// The type solves one problem: a page query and `SetPlcConnection` need the settings, the
/// lock token, and the password state, but never the sealed bytes (`data-model.md` section 3.18).
/// The type has no `PartialEq`, because [`PlcConnection`] has none.
#[derive(Debug)]
pub struct StoredConnection {
    /// The id of the record row. An audit entry names it in `entity_id` (`data-model.md` section 10).
    pub id: PlcConnectionId,
    /// The platform shape of the record.
    pub connection: PlcConnection,
    /// The optimistic lock token of the record. The first save gives 1.
    pub row_version: i64,
    /// Whether the record holds a sealed password.
    pub password: PasswordState,
}

/// A stored connection record that the export read under `FOR SHARE`.
///
/// The type solves one problem: only `ExportPlcConnection` reads the sealed bytes, and the
/// type keeps them apart from the record that every other reader gets.
#[derive(Debug)]
pub struct SharedConnection {
    /// The record without its password.
    pub stored: StoredConnection,
    /// The sealed password, or `None` when the record holds no password.
    pub sealed: Option<SealedSecret>,
}

/// The action of an update on the column `password_sealed`.
///
/// The type solves one problem: an update keeps, replaces, or clears the sealed password, and
/// the adapter must never write a password that the use case did not choose.
#[derive(Debug)]
pub enum PasswordWrite {
    /// Leave the column unchanged. An S7 record always uses this action.
    Keep,
    /// Store this sealed password.
    Set(SealedSecret),
    /// Set the column to `NULL`.
    Clear,
}

/// Repository of the PLC connection record of a machine.
///
/// The port solves one problem: `SetPlcConnection`, `GetPlcConnectionForm`, and
/// `ExportPlcConnection` need the locks, the writes, and the row images of `data-model.md`
/// section 6 without a database in the use case. A machine has at most one record. A lock
/// method takes the SQL lock in its name. A read method takes no lock. A write method returns
/// the row image after the change. A row image never holds `password_sealed`, so an audit
/// entry cannot carry the ciphertext. Every update adds 1 to `row_version`.
#[async_trait]
pub trait PlcConnections: Send {
    /// Read the record of `machine` without a lock (`SetPlcConnection` step 2, `GetPlcConnectionForm`).
    ///
    /// The statement reads `password_sealed IS NOT NULL` and never the sealed bytes.
    /// A machine without a record gives `None`.
    async fn connection_by_machine(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<StoredConnection>, StorageError>;

    /// Take `plc_connection FOR UPDATE` on the record of `machine` (`SetPlcConnection`).
    ///
    /// The result holds the record and its row image before the change.
    /// A machine without a record gives `None` and takes no lock.
    async fn lock_connection_for_update(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<Locked<StoredConnection>>, StorageError>;

    /// Take `plc_connection FOR SHARE` on the record of `machine` and read the sealed password (`ExportPlcConnection`).
    ///
    /// A machine without a record gives `None` and takes no lock.
    async fn lock_connection_shared(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<SharedConnection>, StorageError>;

    /// Run `INSERT plc_connection` for `machine` and return the new row (`SetPlcConnection`).
    ///
    /// The row starts at `row_version` 1 and gets its `id` from the database. The image holds the `id`.
    /// `password` is the sealed password, or `None`.
    /// A second record for the machine gives `Duplicate` with `plc_connection_machine_unique`.
    /// A missing machine gives `MissingReference` with `plc_connection_machine_platform`.
    /// A shape or credentials that the platform or the mode refuses give `E-DB-003`.
    async fn insert_connection(
        &mut self,
        machine: MachineId,
        connection: &PlcConnection,
        password: Option<&SealedSecret>,
    ) -> Result<RowImage, StorageError>;

    /// Run `UPDATE plc_connection SET` the value columns and return the row after the change (`SetPlcConnection`).
    ///
    /// The statement never sets `row_version`. The trigger adds 1. `password` names the action
    /// on `password_sealed`. A shape or credentials that the platform or the mode refuse give `E-DB-003`.
    async fn update_connection(
        &mut self,
        machine: MachineId,
        connection: &PlcConnection,
        password: &PasswordWrite,
    ) -> Result<RowImage, StorageError>;
}
