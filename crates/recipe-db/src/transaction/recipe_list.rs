//! SQL of `RecipeReads::list_recipes` (design D5, `data-model.md` sections 3.1, 3.9, 3.10).
//!
//! The statement runs on the connection of the open transaction and takes no lock.

use recipe_app::error::StorageError;
use recipe_app::ports::{RecipeFilter, RecipeSummary};
use recipe_domain::plant::MachineCode;
use recipe_domain::recipe::{ReferenceNumber, VersionState};
use recipe_domain::text::Text;
use sqlx::PgConnection;
use time::OffsetDateTime;

use super::window::bind_window;
use crate::error::{corrupt, map_error};

/// One page of the recipes. The text `$1` is optional. `strpos` finds it without the wildcards
/// of `LIKE`, and `lower` folds the case of both sides. The code `$2` keeps the live machine with
/// that code, because an archived machine can have the same code (A32). The
/// state `$3` and `$4` (status, deleted) keeps a recipe with `EXISTS` on its versions, because
/// no index covers `status`. The flag `$5` adds the deleted recipes. The lateral join reads the
/// versions of a row once for the approved number and the number of the live draft.
/// `COLLATE "C"` orders the reference number by bytes, whatever the collation of the database is.
const SELECT_LIST: &str = "\
SELECT r.reference_number, r.name, m.code, r.deleted_at, n.approved, n.draft \
FROM recipe r \
JOIN machine m ON m.id = r.machine_id \
LEFT JOIN LATERAL ( \
    SELECT max(v.number) FILTER (WHERE v.status = 'approved') AS approved, \
           max(v.number) FILTER (WHERE v.status = 'draft' AND v.deleted_at IS NULL) AS draft \
    FROM recipe_version v WHERE v.recipe_id = r.id \
) n ON true \
WHERE ($1::text IS NULL OR strpos(lower(r.reference_number), lower($1)) > 0 \
    OR strpos(lower(r.name), lower($1)) > 0) \
AND ($2::text IS NULL OR (m.code = $2 AND m.archived_at IS NULL)) \
AND ($3::text IS NULL OR EXISTS ( \
    SELECT 1 FROM recipe_version v \
    WHERE v.recipe_id = r.id AND v.status = $3 AND (v.deleted_at IS NOT NULL) = $4::boolean)) \
AND ($5::boolean OR r.deleted_at IS NULL) \
ORDER BY r.reference_number COLLATE \"C\" \
OFFSET $6 LIMIT $7";

#[derive(sqlx::FromRow)]
struct SummaryRecord {
    reference_number: String,
    name: String,
    code: String,
    deleted_at: Option<OffsetDateTime>,
    approved: Option<i32>,
    draft: Option<i32>,
}

/// Convert a version number of the database to the number of the port.
fn number(value: Option<i32>) -> Result<Option<u32>, StorageError> {
    value
        .map(u32::try_from)
        .transpose()
        .map_err(|error| corrupt("recipe_version", error))
}

/// Build the summary of a stored row.
fn summary(record: SummaryRecord) -> Result<RecipeSummary, StorageError> {
    Ok(RecipeSummary {
        reference: ReferenceNumber::parse(&record.reference_number)
            .map_err(|error| corrupt("recipe", error))?,
        name: Text::parse(&record.name).map_err(|error| corrupt("recipe", error))?,
        machine_code: MachineCode::parse(&record.code)
            .map_err(|error| corrupt("machine", error))?,
        deleted: record.deleted_at.is_some(),
        approved: number(record.approved)?,
        draft: number(record.draft)?,
    })
}

/// Return the stored shape of a state: its status text and whether the row is deleted.
///
/// `VersionState::from_stored` is the inverse of this function.
fn stored_shape(state: VersionState) -> (&'static str, bool) {
    (
        state.status().as_sql(),
        matches!(state, VersionState::Discarded),
    )
}

/// Read one page of the recipes that match `filter`.
pub(super) async fn list(
    connection: &mut PgConnection,
    filter: &RecipeFilter,
    offset: u64,
    limit: u32,
) -> Result<Vec<RecipeSummary>, StorageError> {
    let (offset, limit) = bind_window(offset, limit)?;
    let state = filter.status.map(stored_shape);
    let records = sqlx::query_as::<_, SummaryRecord>(SELECT_LIST)
        .bind(filter.text.as_ref().map(Text::as_str))
        .bind(filter.machine.map(|code| code.to_string()))
        .bind(state.map(|(status, _)| status))
        .bind(state.is_some_and(|(_, deleted)| deleted))
        .bind(filter.include_deleted)
        .bind(offset)
        .bind(limit)
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    records.into_iter().map(summary).collect()
}
