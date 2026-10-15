//! SQL of `PlantRepository::list_machines` (design D4, `data-model.md` section 3.1).
//!
//! The statement runs on the connection of the open transaction and takes no lock.

use recipe_app::error::StorageError;
use recipe_app::ports::{MachineFilter, MachineSummary};
use recipe_domain::plant::{MachineCode, MachineId, PlcPlatform};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::window::bind_window;
use crate::error::{corrupt, map_error};

/// One page of the machines. The text `$1` is optional. `strpos` finds it without the wildcards
/// of `LIKE`, and `lower` folds the case of both sides. The number of the code orders the rows.
/// For one code, the live machine comes first, and then the latest archiving. `id` ends the order.
const SELECT_LIST: &str = "\
SELECT id, code, name, plc_platform, archived_at FROM machine \
WHERE ($1::text IS NULL OR strpos(lower(code), lower($1)) > 0 \
OR strpos(lower(name), lower($1)) > 0) \
AND ($2::boolean OR archived_at IS NULL) \
ORDER BY substr(code, 2)::integer, archived_at DESC NULLS FIRST, id \
OFFSET $3 LIMIT $4";

#[derive(sqlx::FromRow)]
struct SummaryRecord {
    id: Uuid,
    code: String,
    name: String,
    plc_platform: String,
    archived_at: Option<OffsetDateTime>,
}

/// Build the summary of a stored row.
fn summary(record: SummaryRecord) -> Result<MachineSummary, StorageError> {
    Ok(MachineSummary {
        id: MachineId::from(record.id),
        code: MachineCode::parse(&record.code).map_err(|error| corrupt("machine", error))?,
        name: Text::parse(&record.name).map_err(|error| corrupt("machine", error))?,
        platform: PlcPlatform::parse(&record.plc_platform)
            .map_err(|error| corrupt("machine", error))?,
        archived_at: record.archived_at.map(Timestamp::from),
    })
}

/// Read one page of the machines that match `filter`.
pub(super) async fn list(
    connection: &mut PgConnection,
    filter: &MachineFilter,
    offset: u64,
    limit: u32,
) -> Result<Vec<MachineSummary>, StorageError> {
    let (offset, limit) = bind_window(offset, limit)?;
    let records = sqlx::query_as::<_, SummaryRecord>(SELECT_LIST)
        .bind(filter.text.as_ref().map(Text::as_str))
        .bind(filter.include_archived)
        .bind(offset)
        .bind(limit)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    records.into_iter().map(summary).collect()
}
