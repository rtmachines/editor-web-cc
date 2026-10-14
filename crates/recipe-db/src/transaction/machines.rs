//! SQL of the machine methods of `PlantRepository` (task 9.2, `data-model.md` sections 3.1, 3.2, 6).
//!
//! Each function runs on the connection of the open transaction. A lock statement returns the
//! image of the row before any change. A write returns the image after it (design D5).

use recipe_app::error::StorageError;
use recipe_app::ports::{InsertedMachine, Locked, RowImage};
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlcPlatform, StoredMachine, StoredUnit, Unit, UnitId, UnitPath,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use sqlx::postgres::PgArguments;
use sqlx::{PgConnection, Postgres};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{corrupt, map_error};

/// A live machine by code. The partial unique index `machine_code_live_unique` serves it.
const SELECT_LIVE_BY_CODE: &str = "\
SELECT id, code, name, plc_platform, archived_at FROM machine \
WHERE code = $1 AND archived_at IS NULL";

/// The first archived machine of a code by `id`, as the port contract demands.
const SELECT_ARCHIVED_BY_CODE: &str = "\
SELECT id, code, name, plc_platform, archived_at FROM machine \
WHERE code = $1 AND archived_at IS NOT NULL ORDER BY id LIMIT 1";

const SELECT_BY_ID: &str = "\
SELECT id, code, name, plc_platform, archived_at FROM machine WHERE id = $1";

/// `machine FOR SHARE` (`AddUnit`, `CreateRecipe`). The image comes from the locked row.
const LOCK_SHARED: &str = "\
SELECT id, code, name, plc_platform, archived_at, to_jsonb(m) AS image \
FROM machine m WHERE id = $1 FOR SHARE";

/// `machine FOR NO KEY UPDATE` (`RenameMachine`, `ArchiveMachine`).
const LOCK_NO_KEY_UPDATE: &str = "\
SELECT id, code, name, plc_platform, archived_at, to_jsonb(m) AS image \
FROM machine m WHERE id = $1 FOR NO KEY UPDATE";

const SELECT_UNITS: &str = "\
SELECT id, path, name, archived_at FROM unit WHERE machine_id = $1 ORDER BY id";

const INSERT_MACHINE: &str = "\
INSERT INTO machine (id, code, name, plc_platform, archived_at) \
VALUES ($1, $2, $3, $4, $5) RETURNING to_jsonb(machine)";

const INSERT_UNIT: &str = "\
INSERT INTO unit (id, machine_id, path, name, archived_at) \
VALUES ($1, $2, $3, $4, $5) RETURNING to_jsonb(unit)";

const RENAME_MACHINE: &str = "\
UPDATE machine SET name = $2 WHERE id = $1 RETURNING to_jsonb(machine)";

const ARCHIVE_MACHINE: &str = "\
UPDATE machine SET archived_at = now() WHERE id = $1 RETURNING to_jsonb(machine)";

#[derive(sqlx::FromRow)]
struct MachineRecord {
    id: Uuid,
    code: String,
    name: String,
    plc_platform: String,
    archived_at: Option<OffsetDateTime>,
}

#[derive(sqlx::FromRow)]
struct LockedRecord {
    #[sqlx(flatten)]
    machine: MachineRecord,
    image: Value,
}

#[derive(sqlx::FromRow)]
struct UnitRecord {
    id: Uuid,
    path: String,
    name: String,
    archived_at: Option<OffsetDateTime>,
}

/// Build the domain machine of a machine row and the unit rows of the same machine.
fn build_machine(record: MachineRecord, units: Vec<UnitRecord>) -> Result<Machine, StorageError> {
    let stored_units = units
        .into_iter()
        .map(|unit| {
            Ok(StoredUnit {
                id: UnitId::from(unit.id),
                path: UnitPath::parse(&unit.path).map_err(|error| corrupt("unit", error))?,
                name: Text::parse(&unit.name).map_err(|error| corrupt("unit", error))?,
                archived_at: unit.archived_at.map(Timestamp::from),
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()?;
    let stored = StoredMachine {
        id: MachineId::from(record.id),
        code: MachineCode::parse(&record.code).map_err(|error| corrupt("machine", error))?,
        name: Text::parse(&record.name).map_err(|error| corrupt("machine", error))?,
        platform: PlcPlatform::parse(&record.plc_platform)
            .map_err(|error| corrupt("machine", error))?,
        archived_at: record.archived_at.map(Timestamp::from),
        units: stored_units,
    };
    Machine::try_from(stored).map_err(|error| corrupt("machine", error))
}

/// Read the units of a machine and build the machine.
async fn with_units(
    connection: &mut PgConnection,
    record: MachineRecord,
) -> Result<Machine, StorageError> {
    let units = sqlx::query_as::<_, UnitRecord>(SELECT_UNITS)
        .bind(record.id)
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    build_machine(record, units)
}

/// The query of a machine row with the key already bound.
type MachineQuery = sqlx::query::QueryAs<'static, Postgres, MachineRecord, PgArguments>;

async fn read_one(
    connection: &mut PgConnection,
    query: MachineQuery,
) -> Result<Option<Machine>, StorageError> {
    let record = query
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    match record {
        Some(record) => with_units(connection, record).await.map(Some),
        None => Ok(None),
    }
}

pub(super) async fn by_code(
    connection: &mut PgConnection,
    code: MachineCode,
) -> Result<Option<Machine>, StorageError> {
    let query = sqlx::query_as(SELECT_LIVE_BY_CODE).bind(code.to_string());
    read_one(connection, query).await
}

pub(super) async fn archived_by_code(
    connection: &mut PgConnection,
    code: MachineCode,
) -> Result<Option<Machine>, StorageError> {
    let query = sqlx::query_as(SELECT_ARCHIVED_BY_CODE).bind(code.to_string());
    read_one(connection, query).await
}

pub(super) async fn by_id(
    connection: &mut PgConnection,
    id: MachineId,
) -> Result<Option<Machine>, StorageError> {
    let query = sqlx::query_as(SELECT_BY_ID).bind(*id.as_uuid());
    read_one(connection, query).await
}

/// Take the row lock of `statement` and read the machine with its units.
///
/// A missing row gives `None`. PostgreSQL then takes no lock.
async fn lock(
    connection: &mut PgConnection,
    statement: &'static str,
    id: MachineId,
) -> Result<Option<Locked<Machine>>, StorageError> {
    let record = sqlx::query_as::<_, LockedRecord>(statement)
        .bind(*id.as_uuid())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    let Some(record) = record else {
        return Ok(None);
    };
    let state = with_units(connection, record.machine).await?;
    Ok(Some(Locked {
        state,
        before: RowImage::new(record.image),
    }))
}

pub(super) async fn lock_shared(
    connection: &mut PgConnection,
    id: MachineId,
) -> Result<Option<Locked<Machine>>, StorageError> {
    lock(connection, LOCK_SHARED, id).await
}

pub(super) async fn lock_no_key_update(
    connection: &mut PgConnection,
    id: MachineId,
) -> Result<Option<Locked<Machine>>, StorageError> {
    lock(connection, LOCK_NO_KEY_UPDATE, id).await
}

/// Insert the machine row, then one unit row for each unit in path order.
///
/// A parent path sorts before its children, so the foreign key `unit_parent_exists` holds at
/// every insert.
pub(super) async fn insert(
    connection: &mut PgConnection,
    machine: &Machine,
) -> Result<InsertedMachine, StorageError> {
    let image: Value = sqlx::query_scalar(INSERT_MACHINE)
        .bind(*machine.id().as_uuid())
        .bind(machine.code().to_string())
        .bind(machine.name().as_str())
        .bind(machine.platform().as_str())
        .bind(machine.archived_at().map(|at| *at.as_offset_date_time()))
        .fetch_one(&mut *connection)
        .await
        .map_err(map_error)?;
    let mut units = Vec::new();
    for unit in machine.units() {
        units.push(insert_unit(connection, machine.id(), unit).await?);
    }
    Ok(InsertedMachine {
        machine: RowImage::new(image),
        units,
    })
}

/// Insert one unit row and return its image (`AddUnit`, `CreateMachine`).
///
/// The foreign key `unit_parent_exists` and the unique constraint `unit_path_unique` check the
/// row. The error map names them.
pub(super) async fn insert_unit(
    connection: &mut PgConnection,
    machine: MachineId,
    unit: &Unit,
) -> Result<RowImage, StorageError> {
    let image: Value = sqlx::query_scalar(INSERT_UNIT)
        .bind(*unit.id().as_uuid())
        .bind(*machine.as_uuid())
        .bind(unit.path().to_string())
        .bind(unit.name().as_str())
        .bind(unit.archived_at().map(|at| *at.as_offset_date_time()))
        .fetch_one(connection)
        .await
        .map_err(map_error)?;
    Ok(RowImage::new(image))
}

pub(super) async fn rename(
    connection: &mut PgConnection,
    id: MachineId,
    name: &Text,
) -> Result<RowImage, StorageError> {
    let image: Value = sqlx::query_scalar(RENAME_MACHINE)
        .bind(*id.as_uuid())
        .bind(name.as_str())
        .fetch_one(connection)
        .await
        .map_err(map_error)?;
    Ok(RowImage::new(image))
}

pub(super) async fn archive(
    connection: &mut PgConnection,
    id: MachineId,
) -> Result<RowImage, StorageError> {
    let image: Value = sqlx::query_scalar(ARCHIVE_MACHINE)
        .bind(*id.as_uuid())
        .fetch_one(connection)
        .await
        .map_err(map_error)?;
    Ok(RowImage::new(image))
}
