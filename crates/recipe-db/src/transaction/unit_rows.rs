//! SQL of the unit methods of `PlantRepository` and `UnitLookup` (tasks 9.3 and 9.4,
//! `data-model.md` sections 3.2 and 6).
//!
//! Each function runs on the connection of the open transaction. A lock statement returns the
//! image of the row before any change. A write returns the image after it (design D5).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use recipe_app::error::StorageError;
use recipe_app::ports::{Locked, RowImage};
use recipe_domain::plant::{Machine, MachineId, Unit, UnitId, UnitPath, UnitUsage};
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::text::Text;
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use super::machines;
use crate::error::{corrupt, map_error};

/// `unit FOR UPDATE` at one path (`RenameUnit`). The image comes from the locked row.
const LOCK_FOR_UPDATE: &str = "\
SELECT id, machine_id, path, to_jsonb(u) AS image FROM unit u \
WHERE machine_id = $1 AND path = $2 FOR UPDATE";

/// `unit FOR UPDATE` on a unit and its descendants, ordered by `id` (`ArchiveUnit`).
///
/// The text `$2 || '.%'` matches the descendants. A path has digits and dots only, so it holds
/// no `LIKE` wildcard, and `U1.1` never matches `U1.10`.
const LOCK_TREE: &str = "\
SELECT id, machine_id, path, to_jsonb(u) AS image FROM unit u \
WHERE machine_id = $1 AND (path = $2 OR path LIKE $2 || '.%') ORDER BY id FOR UPDATE";

/// `unit FOR SHARE` at one path (`AddUnit` parent, `AddParameter` unit).
const LOCK_SHARED_AT_PATH: &str = "\
SELECT id, machine_id, path, to_jsonb(u) AS image FROM unit u \
WHERE machine_id = $1 AND path = $2 FOR SHARE";

/// `unit FOR SHARE` on a list of units, ordered by `id` (`CreateVersion`).
///
/// The sort comes before the lock, so the rows are locked in the order of `id`.
const LOCK_SHARED_BY_IDS: &str = "\
SELECT id, machine_id, path, to_jsonb(u) AS image FROM unit u \
WHERE id = ANY($1) ORDER BY id FOR SHARE";

const RENAME_UNIT: &str = "UPDATE unit SET name = $2 WHERE id = $1 RETURNING to_jsonb(unit)";

/// One statement archives all units. The join with the input list gives one image for each
/// input position, in the order of the input, also when an identifier repeats.
const ARCHIVE_UNITS: &str = "\
WITH ids AS (SELECT id, ord FROM unnest($1::uuid[]) WITH ORDINALITY AS t(id, ord)), \
done AS ( \
    UPDATE unit SET archived_at = now() WHERE id IN (SELECT id FROM ids) \
    RETURNING id, to_jsonb(unit) AS image \
) \
SELECT done.image FROM ids JOIN done ON done.id = ids.id ORDER BY ids.ord";

/// The recipes whose live draft or approved version has a parameter on one of the units.
///
/// The query starts from the recipes of the machine and keeps the live versions only, so the
/// retired and discarded versions never reach the parameter join.
const SELECT_USAGE: &str = "\
SELECT DISTINCT p.unit_id, r.reference_number, v.status \
FROM recipe r \
JOIN recipe_version v ON v.recipe_id = r.id \
AND (v.status = 'approved' OR (v.status = 'draft' AND v.deleted_at IS NULL)) \
JOIN parameter p ON p.recipe_version_id = v.id AND p.unit_id = ANY($2) \
WHERE r.machine_id = $1";

/// A unit row as a lock statement reads it, with its image.
#[derive(sqlx::FromRow)]
struct UnitHandle {
    machine_id: Uuid,
    path: String,
    image: Value,
}

#[derive(sqlx::FromRow)]
struct UsageRow {
    unit_id: Uuid,
    reference_number: String,
    status: String,
}

/// Build the domain units of locked rows.
///
/// The domain builds a `Unit` only inside its `Machine`. The function reads each machine of the
/// rows once, and it takes the unit from the machine.
async fn build_units(
    connection: &mut PgConnection,
    rows: Vec<UnitHandle>,
) -> Result<Vec<Locked<Unit>>, StorageError> {
    let mut machines: BTreeMap<Uuid, Machine> = BTreeMap::new();
    let mut locked = Vec::with_capacity(rows.len());
    for row in rows {
        if let Entry::Vacant(slot) = machines.entry(row.machine_id) {
            let machine = machines::by_id(connection, MachineId::from(row.machine_id))
                .await?
                .ok_or_else(|| corrupt("unit", "the machine of the unit does not exist"))?;
            slot.insert(machine);
        }
        let path = UnitPath::parse(&row.path).map_err(|error| corrupt("unit", error))?;
        let state = machines
            .get(&row.machine_id)
            .and_then(|machine| machine.unit(&path))
            .cloned()
            .ok_or_else(|| corrupt("unit", "the unit is not in its machine"))?;
        locked.push(Locked {
            state,
            before: RowImage::new(row.image),
        });
    }
    Ok(locked)
}

/// Run a lock statement that gives at most one unit.
async fn lock_one(
    connection: &mut PgConnection,
    statement: &'static str,
    machine: MachineId,
    path: &UnitPath,
) -> Result<Option<Locked<Unit>>, StorageError> {
    let row = sqlx::query_as::<_, UnitHandle>(statement)
        .bind(*machine.as_uuid())
        .bind(path.to_string())
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(build_units(connection, vec![row]).await?.pop())
}

pub(super) async fn lock_for_update(
    connection: &mut PgConnection,
    machine: MachineId,
    path: &UnitPath,
) -> Result<Option<Locked<Unit>>, StorageError> {
    lock_one(connection, LOCK_FOR_UPDATE, machine, path).await
}

pub(super) async fn lock_tree(
    connection: &mut PgConnection,
    machine: MachineId,
    path: &UnitPath,
) -> Result<Vec<Locked<Unit>>, StorageError> {
    let rows = sqlx::query_as::<_, UnitHandle>(LOCK_TREE)
        .bind(*machine.as_uuid())
        .bind(path.to_string())
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    build_units(connection, rows).await
}

pub(super) async fn lock_shared_at_path(
    connection: &mut PgConnection,
    machine: MachineId,
    path: &UnitPath,
) -> Result<Option<Unit>, StorageError> {
    let locked = lock_one(connection, LOCK_SHARED_AT_PATH, machine, path).await?;
    Ok(locked.map(|entry| entry.state))
}

pub(super) async fn lock_shared_by_ids(
    connection: &mut PgConnection,
    units: &[UnitId],
) -> Result<Vec<Unit>, StorageError> {
    let ids: Vec<Uuid> = units.iter().map(|unit| *unit.as_uuid()).collect();
    let rows = sqlx::query_as::<_, UnitHandle>(LOCK_SHARED_BY_IDS)
        .bind(ids)
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let locked = build_units(connection, rows).await?;
    Ok(locked.into_iter().map(|entry| entry.state).collect())
}

pub(super) async fn rename(
    connection: &mut PgConnection,
    unit: UnitId,
    name: &Text,
) -> Result<RowImage, StorageError> {
    let image: Value = sqlx::query_scalar(RENAME_UNIT)
        .bind(*unit.as_uuid())
        .bind(name.as_str())
        .fetch_one(connection)
        .await
        .map_err(map_error)?;
    Ok(RowImage::new(image))
}

/// Archive all `units` in one statement. The result keeps the order and the count of the input.
pub(super) async fn archive(
    connection: &mut PgConnection,
    units: &[UnitId],
) -> Result<Vec<RowImage>, StorageError> {
    let ids: Vec<Uuid> = units.iter().map(|unit| *unit.as_uuid()).collect();
    let images: Vec<Value> = sqlx::query_scalar(ARCHIVE_UNITS)
        .bind(ids)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    if images.len() != units.len() {
        return Err(corrupt("unit", "an archived unit has no row"));
    }
    Ok(images.into_iter().map(RowImage::new).collect())
}

/// Read the live drafts and the approved versions that use `units`. The statement takes no lock.
pub(super) async fn usage(
    connection: &mut PgConnection,
    machine: MachineId,
    units: &[UnitId],
) -> Result<UnitUsage, StorageError> {
    let ids: Vec<Uuid> = units.iter().map(|unit| *unit.as_uuid()).collect();
    let rows = sqlx::query_as::<_, UsageRow>(SELECT_USAGE)
        .bind(*machine.as_uuid())
        .bind(ids)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    let mut usage = UnitUsage::default();
    for row in rows {
        let unit = UnitId::from(row.unit_id);
        let reference = ReferenceNumber::parse(&row.reference_number)
            .map_err(|error| corrupt("recipe", error))?;
        match row.status.as_str() {
            "draft" => usage.add_draft(unit, reference),
            "approved" => usage.add_approved(unit, reference),
            other => return Err(corrupt("recipe_version", format!("the status {other}"))),
        }
    }
    Ok(usage)
}
