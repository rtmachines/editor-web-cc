//! `PlcConnections` on the transaction (task 4.7, ADR-0154, `data-model.md` sections 3.18 and 6).
//!
//! Each statement runs on the connection of the open transaction. A row image comes from
//! `to_jsonb(plc_connection) - 'password_sealed'`, so no image holds the ciphertext. Only
//! `lock_connection_shared` reads the sealed bytes, and only of an IEC row. The row of an S7
//! record never reads or writes `password_sealed`.

use std::fmt;

use async_trait::async_trait;
use recipe_app::error::{PLC_CONNECTION_MACHINE_PLATFORM, StorageError};
use recipe_app::ports::{
    Locked, PasswordWrite, PlcConnections, RowImage, SealedSecret, SharedConnection,
    StoredConnection,
};
use recipe_domain::plant::{
    CommMode, MachineId, PasswordState, PlcAddress, PlcConnection, PlcConnectionId, PlcHost,
    PlcLogin, PlcPlatform, Rack, Slot, TcpPort,
};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use super::PgTransaction;
use crate::error::{corrupt, map_error, missing_row};

/// The table of this module, for the errors of a corrupt row.
const TABLE: &str = "plc_connection";

/// The record of a machine without a lock (`GetPlcConnectionForm`, `SetPlcConnection` step 2).
const SELECT_BY_MACHINE: &str = "\
SELECT id, plc_platform, host, rack, slot, port, address, comm_mode, login, row_version, \
       CASE WHEN plc_platform IN ('CodesysIec', 'TwinCat3') \
            THEN password_sealed IS NOT NULL ELSE false END AS has_password \
FROM plc_connection WHERE machine_id = $1";

/// `plc_connection FOR UPDATE` (`SetPlcConnection`). The image comes from the locked row.
const LOCK_FOR_UPDATE: &str = "\
SELECT id, plc_platform, host, rack, slot, port, address, comm_mode, login, row_version, \
       CASE WHEN plc_platform IN ('CodesysIec', 'TwinCat3') \
            THEN password_sealed IS NOT NULL ELSE false END AS has_password, \
       to_jsonb(c) - 'password_sealed' AS image \
FROM plc_connection c WHERE machine_id = $1 FOR UPDATE";

/// `plc_connection FOR SHARE` (`ExportPlcConnection`). The only statement that reads the bytes.
const LOCK_SHARED: &str = "\
SELECT id, plc_platform, host, rack, slot, port, address, comm_mode, login, row_version, \
       CASE WHEN plc_platform IN ('CodesysIec', 'TwinCat3') THEN password_sealed END AS sealed \
FROM plc_connection WHERE machine_id = $1 FOR SHARE";

/// The new row takes the platform and the data block number of its machine, so the composite key
/// `plc_connection_machine_platform` and the shape check hold. A missing machine gives no row,
/// and the adapter maps that to the key. An IEC machine has no number, so the copy is `NULL`.
const INSERT: &str = "\
INSERT INTO plc_connection \
    (machine_id, plc_platform, host, rack, slot, db_number, port, address, comm_mode, login, password_sealed) \
SELECT m.id, m.plc_platform, $2, $3, $4, m.db_number, $5, $6, $7, $8, $9 \
FROM machine m WHERE m.id = $1 \
RETURNING to_jsonb(plc_connection) - 'password_sealed'";

/// The update that leaves `password_sealed` unchanged. The trigger adds 1 to `row_version`.
/// The copy `db_number` comes from the machine row that the command already holds (design D7).
const UPDATE_KEEP: &str = "\
UPDATE plc_connection SET host = $2, rack = $3, slot = $4, \
    db_number = (SELECT m.db_number FROM machine m WHERE m.id = $1), port = $5, \
    address = $6, comm_mode = $7, login = $8 \
WHERE machine_id = $1 RETURNING to_jsonb(plc_connection) - 'password_sealed'";

/// The update that stores a new sealed password.
const UPDATE_SET: &str = "\
UPDATE plc_connection SET host = $2, rack = $3, slot = $4, \
    db_number = (SELECT m.db_number FROM machine m WHERE m.id = $1), port = $5, \
    address = $6, comm_mode = $7, login = $8, password_sealed = $9 \
WHERE machine_id = $1 RETURNING to_jsonb(plc_connection) - 'password_sealed'";

/// The update that sets `password_sealed` to `NULL`.
const UPDATE_CLEAR: &str = "\
UPDATE plc_connection SET host = $2, rack = $3, slot = $4, \
    db_number = (SELECT m.db_number FROM machine m WHERE m.id = $1), port = $5, \
    address = $6, comm_mode = $7, login = $8, password_sealed = NULL \
WHERE machine_id = $1 RETURNING to_jsonb(plc_connection) - 'password_sealed'";

/// The value columns of a record, as the statements bind them.
///
/// The type solves one problem: both shapes bind the same seven parameters, and a column of the
/// other shape is `NULL`. The `CHECK` `plc_connection_shape` refuses a mix.
struct Columns<'a> {
    host: &'a str,
    rack: Option<i16>,
    slot: Option<i16>,
    port: Option<i32>,
    address: Option<&'a str>,
    comm_mode: Option<&'static str>,
    login: Option<&'a str>,
}

impl<'a> From<&'a PlcConnection> for Columns<'a> {
    fn from(connection: &'a PlcConnection) -> Self {
        match connection {
            PlcConnection::S7 { host, rack, slot } => Self {
                host: host.as_str(),
                rack: Some(i16::from(rack.value())),
                slot: Some(i16::from(slot.value())),
                port: None,
                address: None,
                comm_mode: None,
                login: None,
            },
            PlcConnection::Iec {
                host,
                port,
                address,
                mode,
                login,
            } => Self {
                host: host.as_str(),
                rack: None,
                slot: None,
                port: Some(i32::from(port.value())),
                address: Some(address.as_str()),
                comm_mode: Some(mode.as_str()),
                login: login.as_ref().map(PlcLogin::as_str),
            },
        }
    }
}

/// The query of a statement that returns the image of one row.
type QueryScalar<'q, T> =
    sqlx::query::QueryScalar<'q, sqlx::Postgres, T, sqlx::postgres::PgArguments>;

/// Bind the key and the seven value columns of `columns`, in the order of the statements.
fn bind_columns<'q>(
    query: QueryScalar<'q, Value>,
    machine: MachineId,
    columns: &Columns<'q>,
) -> QueryScalar<'q, Value> {
    query
        .bind(*machine.as_uuid())
        .bind(columns.host)
        .bind(columns.rack)
        .bind(columns.slot)
        .bind(columns.port)
        .bind(columns.address)
        .bind(columns.comm_mode)
        .bind(columns.login)
}

/// The value columns of a stored row. No field holds `password_sealed`.
#[derive(sqlx::FromRow)]
struct ConnectionRecord {
    id: Uuid,
    plc_platform: String,
    host: String,
    rack: Option<i16>,
    slot: Option<i16>,
    port: Option<i32>,
    address: Option<String>,
    comm_mode: Option<String>,
    login: Option<String>,
    row_version: i64,
}

#[derive(sqlx::FromRow)]
struct StoredRecord {
    #[sqlx(flatten)]
    record: ConnectionRecord,
    has_password: bool,
}

#[derive(sqlx::FromRow)]
struct LockedRecord {
    #[sqlx(flatten)]
    stored: StoredRecord,
    image: Value,
}

/// A row of `LOCK_SHARED`. The type has no `Debug`, so no output shows the sealed bytes.
#[derive(sqlx::FromRow)]
struct SharedRecord {
    #[sqlx(flatten)]
    record: ConnectionRecord,
    sealed: Option<Vec<u8>>,
}

/// Return the value of a required column, or the error of a corrupt row.
fn required<T>(column: &'static str, value: Option<T>) -> Result<T, StorageError> {
    value.ok_or_else(|| corrupt(TABLE, format!("the column {column} is null")))
}

/// Parse a number column through its value object. The value object takes the text of the number.
fn parse_number<T, E: fmt::Display>(
    column: &'static str,
    value: Option<impl fmt::Display>,
    parse: impl FnOnce(&str) -> Result<T, E>,
) -> Result<T, StorageError> {
    let value = required(column, value)?;
    parse(&value.to_string()).map_err(|error| corrupt(TABLE, error))
}

/// Parse a text column through its value object.
fn parse_text<T, E: fmt::Display>(
    column: &'static str,
    value: Option<&str>,
    parse: impl FnOnce(&str) -> Result<T, E>,
) -> Result<T, StorageError> {
    parse(required(column, value)?).map_err(|error| corrupt(TABLE, error))
}

/// Build the connection of a row. The platform decides the shape.
fn build_connection(record: &ConnectionRecord) -> Result<PlcConnection, StorageError> {
    let platform =
        PlcPlatform::parse(&record.plc_platform).map_err(|error| corrupt(TABLE, error))?;
    let host = PlcHost::parse(&record.host).map_err(|error| corrupt(TABLE, error))?;
    if platform.is_s7() {
        Ok(PlcConnection::S7 {
            host,
            rack: parse_number("rack", record.rack, Rack::parse)?,
            slot: parse_number("slot", record.slot, Slot::parse)?,
        })
    } else {
        let login = record
            .login
            .as_deref()
            .map(PlcLogin::parse)
            .transpose()
            .map_err(|error| corrupt(TABLE, error))?;
        Ok(PlcConnection::Iec {
            host,
            port: parse_number("port", record.port, TcpPort::parse)?,
            address: parse_text("address", record.address.as_deref(), PlcAddress::parse)?,
            mode: parse_text("comm_mode", record.comm_mode.as_deref(), CommMode::parse)?,
            login,
        })
    }
}

/// Build the stored record of a row.
fn build_stored(record: StoredRecord) -> Result<StoredConnection, StorageError> {
    Ok(StoredConnection {
        id: PlcConnectionId::from(record.record.id),
        connection: build_connection(&record.record)?,
        row_version: record.record.row_version,
        password: if record.has_password {
            PasswordState::Stored
        } else {
            PasswordState::Absent
        },
    })
}

async fn by_machine(
    connection: &mut PgConnection,
    machine: MachineId,
) -> Result<Option<StoredConnection>, StorageError> {
    let record = sqlx::query_as::<_, StoredRecord>(SELECT_BY_MACHINE)
        .bind(*machine.as_uuid())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    record.map(build_stored).transpose()
}

async fn lock_for_update(
    connection: &mut PgConnection,
    machine: MachineId,
) -> Result<Option<Locked<StoredConnection>>, StorageError> {
    let record = sqlx::query_as::<_, LockedRecord>(LOCK_FOR_UPDATE)
        .bind(*machine.as_uuid())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    record
        .map(|record| {
            Ok(Locked {
                state: build_stored(record.stored)?,
                before: RowImage::new(record.image),
            })
        })
        .transpose()
}

async fn lock_shared(
    connection: &mut PgConnection,
    machine: MachineId,
) -> Result<Option<SharedConnection>, StorageError> {
    let record = sqlx::query_as::<_, SharedRecord>(LOCK_SHARED)
        .bind(*machine.as_uuid())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    record
        .map(|record| {
            let has_password = record.sealed.is_some();
            let stored = StoredRecord {
                record: record.record,
                has_password,
            };
            Ok(SharedConnection {
                stored: build_stored(stored)?,
                sealed: record.sealed.map(SealedSecret::from_bytes),
            })
        })
        .transpose()
}

async fn insert(
    connection: &mut PgConnection,
    machine: MachineId,
    plc: &PlcConnection,
    password: Option<&SealedSecret>,
) -> Result<RowImage, StorageError> {
    let columns = Columns::from(plc);
    let query = bind_columns(sqlx::query_scalar(INSERT), machine, &columns)
        .bind(password.map(SealedSecret::as_bytes));
    let image = query.fetch_optional(connection).await.map_err(map_error)?;
    // The statement selects from `machine`. No machine gives no row, as a missing key would.
    image
        .map(RowImage::new)
        .ok_or_else(|| StorageError::MissingReference {
            constraint: PLC_CONNECTION_MACHINE_PLATFORM.to_owned(),
        })
}

async fn update(
    connection: &mut PgConnection,
    machine: MachineId,
    plc: &PlcConnection,
    password: &PasswordWrite,
) -> Result<RowImage, StorageError> {
    let columns = Columns::from(plc);
    let statement = match password {
        PasswordWrite::Keep => UPDATE_KEEP,
        PasswordWrite::Set(_) => UPDATE_SET,
        PasswordWrite::Clear => UPDATE_CLEAR,
    };
    let mut query = bind_columns(sqlx::query_scalar(statement), machine, &columns);
    if let PasswordWrite::Set(secret) = password {
        query = query.bind(secret.as_bytes());
    }
    let image = query.fetch_optional(connection).await.map_err(map_error)?;
    image.map(RowImage::new).ok_or_else(|| missing_row(TABLE))
}

#[async_trait]
impl PlcConnections for PgTransaction {
    async fn connection_by_machine(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<StoredConnection>, StorageError> {
        by_machine(&mut self.tx, machine).await
    }

    async fn lock_connection_for_update(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<Locked<StoredConnection>>, StorageError> {
        lock_for_update(&mut self.tx, machine).await
    }

    async fn lock_connection_shared(
        &mut self,
        machine: MachineId,
    ) -> Result<Option<SharedConnection>, StorageError> {
        lock_shared(&mut self.tx, machine).await
    }

    async fn insert_connection(
        &mut self,
        machine: MachineId,
        connection: &PlcConnection,
        password: Option<&SealedSecret>,
    ) -> Result<RowImage, StorageError> {
        insert(&mut self.tx, machine, connection, password).await
    }

    async fn update_connection(
        &mut self,
        machine: MachineId,
        connection: &PlcConnection,
        password: &PasswordWrite,
    ) -> Result<RowImage, StorageError> {
        update(&mut self.tx, machine, connection, password).await
    }
}
