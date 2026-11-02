//! Writer of the list body of the transfer client (`api.md` section 2.7).
//!
//! The structs of this module solve one problem: the list body writes its fields in the fixed
//! order of `api.md` section 2.7. A `serde_json::Value` map would sort the keys, so the module
//! never builds one.

use recipe_domain::export::{ExportError, ExportableRecipe, ExportableRecipes, Sha256, utc_text};
use recipe_domain::plant::MachineCode;
use serde::Serialize;
use serde::ser::Serializer;

/// The list body. The fields come in the order of `api.md` section 2.7.
#[derive(Serialize)]
struct ListBody<'a> {
    machine: MachineFields<'a>,
    recipes: Vec<RecipeFields<'a>>,
}

/// The object `machine`.
#[derive(Serialize)]
struct MachineFields<'a> {
    #[serde(serialize_with = "display_text")]
    code: &'a MachineCode,
    plc_platform: &'static str,
    archived: bool,
}

/// One row of `recipes[]`.
#[derive(Serialize)]
struct RecipeFields<'a> {
    reference_number: &'a str,
    name: &'a str,
    version_number: u32,
    approved_at: String,
    #[serde(serialize_with = "display_text")]
    content_sha256: &'a Sha256,
}

/// Serialize a value as the JSON string of its `Display` text.
fn display_text<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: std::fmt::Display,
    S: Serializer,
{
    serializer.collect_str(value)
}

/// Write the list of exportable recipes as pretty JSON.
///
/// The body has the fields of `api.md` section 2.7 in their fixed order. The indent is two
/// spaces, and the text has no newline at the end, as in `write_document`. The rows keep the
/// order of `list.recipes`. `approved_at` is UTC text with whole seconds, and `content_sha256`
/// is lower-case hexadecimal.
///
/// # Errors
///
/// Returns `E-EXP-004` (`ExportError::Serialize`) when an `approved_at` has a UTC year outside
/// 0000 to 9999, or when `serde_json` fails.
pub fn write_list(list: &ExportableRecipes) -> Result<Vec<u8>, ExportError> {
    let recipes = list
        .recipes
        .iter()
        .map(recipe_fields)
        .collect::<Result<Vec<_>, _>>()?;
    let body = ListBody {
        machine: MachineFields {
            code: &list.code,
            plc_platform: list.platform.as_str(),
            archived: list.archived,
        },
        recipes,
    };
    Ok(serde_json::to_vec_pretty(&body)?)
}

/// Build the fields of one row.
///
/// # Errors
///
/// Returns `E-EXP-004` when `approved_at` has a UTC year outside 0000 to 9999.
fn recipe_fields(row: &ExportableRecipe) -> Result<RecipeFields<'_>, ExportError> {
    Ok(RecipeFields {
        reference_number: row.reference.as_str(),
        name: row.name.as_str(),
        version_number: row.version_number,
        approved_at: utc_text(row.approved_at)?,
        content_sha256: &row.content_sha256,
    })
}

#[cfg(test)]
mod tests;
