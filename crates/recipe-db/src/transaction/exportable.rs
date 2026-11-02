//! SQL of `RecipeReads::exportable_recipes` (design D4, ADR-0109 (j), `data-model.md` section 6).
//!
//! The statement runs on the connection of the open transaction and takes no row lock.

use recipe_app::error::StorageError;
use recipe_app::ports::{ExportableRecipe, ExportableRecipes};
use recipe_domain::export::Sha256;
use recipe_domain::plant::{MachineCode, PlcPlatform};
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::error::{corrupt, map_error};

/// The machine of the code `$1` and its recipes with an approved version.
///
/// The CTE `resolved` applies ADR-0109 (j) as `SELECT_EXPORT_BY_CODE` in `machines.rs` does. `archived_at DESC NULLS FIRST` puts the live machine
/// first and then the latest archiving. `id DESC` breaks a tie of two equal instants, as the
/// memory adapter does. The inner join in the parentheses keeps a recipe only with its approved
/// version, so a machine without such a recipe still gives one row with null recipe columns.
/// `COLLATE "C"` orders the reference number by bytes, whatever the collation of the database is.
const SELECT_EXPORTABLE: &str = "\
WITH resolved AS ( \
    SELECT id, code, plc_platform, archived_at FROM machine \
    WHERE code = $1 \
    ORDER BY archived_at DESC NULLS FIRST, id DESC LIMIT 1 \
) \
SELECT m.code, m.plc_platform, m.archived_at IS NOT NULL AS archived, \
    r.reference_number, r.name, v.number, v.approved_at, v.content_sha256 \
FROM resolved m \
LEFT JOIN (recipe r JOIN recipe_version v ON v.recipe_id = r.id AND v.status = 'approved') \
    ON r.machine_id = m.id AND r.deleted_at IS NULL \
ORDER BY r.reference_number COLLATE \"C\"";

#[derive(sqlx::FromRow)]
struct ExportableRecord {
    code: String,
    plc_platform: String,
    archived: bool,
    reference_number: Option<String>,
    name: Option<String>,
    number: Option<i32>,
    approved_at: Option<OffsetDateTime>,
    content_sha256: Option<String>,
}

/// Build the recipe row of a stored row, or `None` for the row of a machine without recipes.
///
/// An approved version without its approval time or its hash is a corrupt row.
fn recipe(record: &ExportableRecord) -> Result<Option<ExportableRecipe>, StorageError> {
    let Some(reference) = record.reference_number.as_deref() else {
        return Ok(None);
    };
    let (Some(name), Some(number), Some(approved_at), Some(hash)) = (
        record.name.as_deref(),
        record.number,
        record.approved_at,
        record.content_sha256.as_deref(),
    ) else {
        return Err(corrupt(
            "recipe_version",
            "an approved version has no approval or hash",
        ));
    };
    Ok(Some(ExportableRecipe {
        reference: ReferenceNumber::parse(reference).map_err(|error| corrupt("recipe", error))?,
        name: Text::parse(name).map_err(|error| corrupt("recipe", error))?,
        version_number: u32::try_from(number).map_err(|error| corrupt("recipe_version", error))?,
        approved_at: Timestamp::from(approved_at),
        content_sha256: Sha256::parse(hash).map_err(|error| corrupt("recipe_version", error))?,
    }))
}

/// Read the machine that `code` names and its exportable recipes. No machine gives `None`.
pub(super) async fn by_machine_code(
    connection: &mut PgConnection,
    code: MachineCode,
) -> Result<Option<ExportableRecipes>, StorageError> {
    let records = sqlx::query_as::<_, ExportableRecord>(SELECT_EXPORTABLE)
        .bind(code.to_string())
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    let Some(first) = records.first() else {
        return Ok(None);
    };
    Ok(Some(ExportableRecipes {
        code: MachineCode::parse(&first.code).map_err(|error| corrupt("machine", error))?,
        platform: PlcPlatform::parse(&first.plc_platform)
            .map_err(|error| corrupt("machine", error))?,
        archived: first.archived,
        recipes: records
            .iter()
            .filter_map(|record| recipe(record).transpose())
            .collect::<Result<Vec<_>, _>>()?,
    }))
}
