//! SQL of `TypeCatalog::type_usage` (design D4, `data-model.md` sections 3.3, 3.4, 3.9, 3.10,
//! and 3.12).
//!
//! The statement runs on the connection of the open transaction and takes no lock.

use recipe_app::error::StorageError;
use recipe_app::ports::TypeUsageRow;
use recipe_domain::plant::{MachineId, PlcTypeId};
use recipe_domain::recipe::{ParameterTag, ReferenceNumber, VersionStatus};
use sqlx::PgConnection;
use uuid::Uuid;

use super::window::bind_window;
use crate::error::{corrupt, map_error};

/// One page of the parameters that use the type `$2` of the machine `$1`.
///
/// `edges` is the one set of links from a type to the struct or the array that holds it. The
/// first part lists the member links, and the second part lists the element links. `UNION ALL`
/// keeps a link twice when a struct has two members of one type, and it needs no sort or hash
/// step. `holders` starts at the type and adds each type that holds a type of the set. Its
/// `UNION` drops a row that the set already has, so a cycle of links ends and a double link
/// adds no row. The recursive term reads `holders` once.
/// `IN` gives one row for each parameter, also when its type holds the type twice. A discarded
/// draft has `v.deleted_at`, and a deleted recipe has `r.deleted_at`. `COLLATE "C"` orders the
/// text by bytes, whatever the collation of the database is.
const SELECT_USAGE: &str = "\
WITH RECURSIVE edges AS ( \
    SELECT member_type_id AS inner_id, struct_type_id AS outer_id FROM plc_type_member \
    UNION ALL \
    SELECT element_type_id, id FROM plc_type WHERE element_type_id IS NOT NULL \
), holders AS ( \
    SELECT $2::uuid AS type_id \
    UNION \
    SELECT edges.outer_id FROM edges JOIN holders ON edges.inner_id = holders.type_id \
) \
SELECT r.reference_number, v.number, v.status, p.tag, p.plc_type_id \
FROM parameter p \
JOIN recipe_version v ON v.id = p.recipe_version_id \
JOIN recipe r ON r.id = v.recipe_id \
WHERE r.machine_id = $1 AND r.deleted_at IS NULL AND v.deleted_at IS NULL \
AND p.plc_type_id IN (SELECT type_id FROM holders) \
ORDER BY r.reference_number COLLATE \"C\", v.number DESC, p.tag COLLATE \"C\" \
OFFSET $3 LIMIT $4";

#[derive(sqlx::FromRow)]
struct UsageRecord {
    reference_number: String,
    number: i32,
    status: String,
    tag: String,
    plc_type_id: Uuid,
}

/// Build the usage row of a stored row.
fn usage_row(record: UsageRecord) -> Result<TypeUsageRow, StorageError> {
    Ok(TypeUsageRow {
        reference: ReferenceNumber::parse(&record.reference_number)
            .map_err(|error| corrupt("recipe", error))?,
        number: u32::try_from(record.number).map_err(|error| corrupt("recipe_version", error))?,
        status: VersionStatus::parse(&record.status)
            .map_err(|error| corrupt("recipe_version", error))?,
        tag: ParameterTag::parse(&record.tag).map_err(|error| corrupt("parameter", error))?,
        parameter_type: PlcTypeId::from(record.plc_type_id),
    })
}

/// Read one page of the parameters of `machine` that use the type `type_id`.
pub(super) async fn usage(
    connection: &mut PgConnection,
    machine: MachineId,
    type_id: PlcTypeId,
    offset: u64,
    limit: u32,
) -> Result<Vec<TypeUsageRow>, StorageError> {
    let (offset, limit) = bind_window(offset, limit)?;
    let records = sqlx::query_as::<_, UsageRecord>(SELECT_USAGE)
        .bind(*machine.as_uuid())
        .bind(*type_id.as_uuid())
        .bind(offset)
        .bind(limit)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    records.into_iter().map(usage_row).collect()
}
