//! In-memory `PlcConnections` (task 5.1, ADR-0154).

use async_trait::async_trait;
use recipe_domain::plant::{
    CommMode, DataBlockNumber, MachineId, PasswordState, PlcAddress, PlcConnection,
    PlcConnectionId, PlcHost, PlcLogin, PlcPlatform, Rack, Slot, TcpPort,
};
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use uuid::Uuid;

use super::images::{object, time_value};
use super::rows::{check_violation, duplicate, missing_reference, missing_row};
use super::{LockMode, MemoryStore, MemoryTransaction};
use crate::error::{PLC_CONNECTION_MACHINE_PLATFORM, PLC_CONNECTION_MACHINE_UNIQUE, StorageError};
use crate::ports::{
    Locked, PasswordWrite, PlcConnections, RowImage, SealedSecret, SharedConnection,
    StoredConnection,
};

/// The table name of the journal for a connection lock.
const TABLE: &str = "plc_connection";

/// The high bits of every row id of this table, so a row id never equals the id of a machine that a test seeds.
const ROW_ID_BASE: u128 = 1 << 100;

/// Return the id of the row with this serial number.
fn row_id(serial: u128) -> Uuid {
    Uuid::from_u128(ROW_ID_BASE | serial)
}

/// The columns of a row that depend on the platform shape.
///
/// The type solves one problem: [`PlcConnection`] has no `Clone`, and the state of the
/// in-memory store must copy at `begin`. The row keeps the values and builds a connection on demand.
#[derive(Clone, Debug)]
pub(crate) enum ShapeRow {
    /// The columns of an S7 row.
    S7 {
        host: PlcHost,
        rack: Rack,
        slot: Slot,
        db_number: DataBlockNumber,
    },
    /// The columns of an IEC row.
    Iec {
        host: PlcHost,
        port: TcpPort,
        address: PlcAddress,
        mode: CommMode,
        login: Option<PlcLogin>,
    },
}

impl ShapeRow {
    fn of(connection: &PlcConnection) -> Self {
        match connection {
            PlcConnection::S7 {
                host,
                rack,
                slot,
                db_number,
            } => Self::S7 {
                host: host.clone(),
                rack: *rack,
                slot: *slot,
                db_number: *db_number,
            },
            PlcConnection::Iec {
                host,
                port,
                address,
                mode,
                login,
            } => Self::Iec {
                host: host.clone(),
                port: *port,
                address: address.clone(),
                mode: *mode,
                login: login.clone(),
            },
        }
    }

    fn to_connection(&self) -> PlcConnection {
        match self {
            Self::S7 {
                host,
                rack,
                slot,
                db_number,
            } => PlcConnection::S7 {
                host: host.clone(),
                rack: *rack,
                slot: *slot,
                db_number: *db_number,
            },
            Self::Iec {
                host,
                port,
                address,
                mode,
                login,
            } => PlcConnection::Iec {
                host: host.clone(),
                port: *port,
                address: address.clone(),
                mode: *mode,
                login: login.clone(),
            },
        }
    }

    /// Return true when the shape fits the platform (`plc_connection_shape`).
    const fn fits(&self, platform: PlcPlatform) -> bool {
        matches!(self, Self::S7 { .. }) == platform.is_s7()
    }

    /// Return true when the login and the password follow the mode (`plc_connection_credentials`).
    fn credentials_fit(&self, has_password: bool) -> bool {
        match self {
            Self::S7 { .. } => !has_password,
            Self::Iec { mode, login, .. } => match mode {
                CommMode::OpcUa | CommMode::CodesysV3 => login.is_some() && has_password,
                CommMode::Ads => login.is_some() == has_password,
                CommMode::ModbusTcp => login.is_none() && !has_password,
            },
        }
    }

    /// Return the columns of the shape for a row image.
    fn columns(&self) -> Vec<(&'static str, Value)> {
        match self {
            Self::S7 {
                host,
                rack,
                slot,
                db_number,
            } => vec![
                ("host", Value::from(host.as_str())),
                ("rack", Value::from(rack.value())),
                ("slot", Value::from(slot.value())),
                ("db_number", Value::from(db_number.value())),
                ("port", Value::Null),
                ("address", Value::Null),
                ("comm_mode", Value::Null),
                ("login", Value::Null),
            ],
            Self::Iec {
                host,
                port,
                address,
                mode,
                login,
            } => vec![
                ("host", Value::from(host.as_str())),
                ("rack", Value::Null),
                ("slot", Value::Null),
                ("db_number", Value::Null),
                ("port", Value::from(port.value())),
                ("address", Value::from(address.as_str())),
                ("comm_mode", Value::from(mode.as_str())),
                (
                    "login",
                    login
                        .as_ref()
                        .map_or(Value::Null, |login| Value::from(login.as_str())),
                ),
            ],
        }
    }
}

/// A row of the table `plc_connection` (`data-model.md` section 3.18).
#[derive(Clone, Debug)]
pub(crate) struct ConnectionRow {
    id: Uuid,
    platform: PlcPlatform,
    shape: ShapeRow,
    sealed: Option<SealedSecret>,
    row_version: i64,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl ConnectionRow {
    fn stored(&self) -> StoredConnection {
        StoredConnection {
            id: PlcConnectionId::from(self.id),
            connection: self.shape.to_connection(),
            row_version: self.row_version,
            password: if self.sealed.is_some() {
                PasswordState::Stored
            } else {
                PasswordState::Absent
            },
        }
    }

    /// Return the image of the row. It never holds `password_sealed`.
    fn image(&self, machine: MachineId) -> RowImage {
        let mut pairs = vec![
            ("id", Value::String(self.id.to_string())),
            ("machine_id", Value::String(machine.to_string())),
            ("plc_platform", Value::String(self.platform.to_string())),
            ("row_version", Value::from(self.row_version)),
            ("created_at", time_value(self.created_at)),
            ("updated_at", time_value(self.updated_at)),
        ];
        pairs.extend(self.shape.columns());
        RowImage::new(object(pairs))
    }
}

/// Check the shape and the credentials of a row like the table checks do.
fn check_row(
    shape: &ShapeRow,
    platform: PlcPlatform,
    has_password: bool,
) -> Result<(), StorageError> {
    if shape.fits(platform) && shape.credentials_fit(has_password) {
        Ok(())
    } else {
        Err(check_violation())
    }
}

impl MemoryTransaction {
    /// Return the platform of a machine, or a missing reference for the composite foreign key.
    fn platform_of(&self, machine: MachineId) -> Result<PlcPlatform, StorageError> {
        self.state()
            .machines
            .get(&machine)
            .map(|row| row.platform)
            .ok_or_else(|| missing_reference(PLC_CONNECTION_MACHINE_PLATFORM))
    }
}

impl MemoryStore {
    /// Store a committed record for `machine` at `row_version` 1 without a transaction.
    ///
    /// A fixture of the tests uses it for a record that exists before a command runs.
    /// The method keeps no record when the machine is missing, so a test names a seeded machine.
    pub fn seed_connection(
        &self,
        machine: MachineId,
        connection: &PlcConnection,
        password: Option<SealedSecret>,
    ) {
        self.update_state(|state, now| {
            let Some(platform) = state.machines.get(&machine).map(|row| row.platform) else {
                return;
            };
            let Ok(serial) = state.next_connection_serial() else {
                return;
            };
            let row = ConnectionRow {
                id: row_id(serial),
                platform,
                shape: ShapeRow::of(connection),
                sealed: password,
                row_version: 1,
                created_at: now,
                updated_at: now,
            };
            state.connections.insert(machine, row);
        });
    }
}

#[async_trait]
impl PlcConnections for MemoryTransaction {
    async fn connection_by_machine(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<StoredConnection>, StorageError> {
        Ok(self
            .state()
            .connections
            .get(&machine)
            .map(ConnectionRow::stored))
    }

    async fn lock_connection_for_update(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<Locked<StoredConnection>>, StorageError> {
        let Some(row) = self.state().connections.get(&machine) else {
            return Ok(None);
        };
        let locked = Locked {
            state: row.stored(),
            before: row.image(machine),
        };
        self.lock(TABLE, LockMode::ForUpdate, &machine.to_string());
        Ok(Some(locked))
    }

    async fn lock_connection_shared(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<SharedConnection>, StorageError> {
        let Some(row) = self.state().connections.get(&machine) else {
            return Ok(None);
        };
        let shared = SharedConnection {
            stored: row.stored(),
            sealed: row.sealed.clone(),
        };
        self.lock(TABLE, LockMode::ForShare, &machine.to_string());
        Ok(Some(shared))
    }

    async fn insert_connection(
        &mut self,
        machine: MachineId,
        connection: &PlcConnection,
        password: Option<&SealedSecret>,
    ) -> Result<RowImage, StorageError> {
        let platform = self.platform_of(machine)?;
        if self.state().connections.contains_key(&machine) {
            return Err(duplicate(PLC_CONNECTION_MACHINE_UNIQUE));
        }
        let shape = ShapeRow::of(connection);
        check_row(&shape, platform, password.is_some())?;
        let serial = self.state_mut().next_connection_serial()?;
        let now = self.now();
        let row = ConnectionRow {
            id: row_id(serial),
            platform,
            shape,
            sealed: password.cloned(),
            row_version: 1,
            created_at: now,
            updated_at: now,
        };
        let image = row.image(machine);
        self.state_mut().connections.insert(machine, row);
        Ok(image)
    }

    async fn update_connection(
        &mut self,
        machine: MachineId,
        connection: &PlcConnection,
        password: &PasswordWrite,
    ) -> Result<RowImage, StorageError> {
        let now = self.now();
        let Some(existing) = self.state().connections.get(&machine) else {
            return Err(missing_row(TABLE));
        };
        let shape = ShapeRow::of(connection);
        let sealed = match password {
            PasswordWrite::Keep => existing.sealed.clone(),
            PasswordWrite::Set(secret) => Some(secret.clone()),
            PasswordWrite::Clear => None,
        };
        check_row(&shape, existing.platform, sealed.is_some())?;
        let row_version = existing
            .row_version
            .checked_add(1)
            .ok_or_else(|| StorageError::invariant("the row version overflows"))?;
        let row = ConnectionRow {
            shape,
            sealed,
            row_version,
            updated_at: now,
            ..existing.clone()
        };
        let image = row.image(machine);
        self.state_mut().connections.insert(machine, row);
        Ok(image)
    }
}

#[cfg(test)]
mod tests;
