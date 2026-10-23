//! Writer of the export document v1 (`api.md` sections 6.1 to 6.3, design D5 (a)).
//!
//! The structs of this module solve one problem: the document writes its fields in the fixed
//! order of `api.md` section 6.1. A `serde_json::Value` map would sort the keys, so the module
//! never builds one.

use std::collections::BTreeMap;
use std::fmt::Display;

use recipe_domain::export::{
    DocumentFacts, ExportError, ExportStatus, HashedBody, LimitEntry, OrderedValue, ParameterEntry,
    SCHEMA_VERSION, Sha256, TypeEntry, TypeReference, utc_text,
};
use recipe_domain::plant::{MachineCode, UnitPath};
use recipe_domain::recipe::ParameterKey;
use recipe_domain::text::Text;
use serde::ser::Error as _;
use serde::{Serialize, Serializer};

/// The export document. The fields come in the order of `api.md` section 6.1.
#[derive(Serialize)]
struct Document<'a> {
    schema_version: &'static str,
    exported_at: String,
    exported_by: &'a str,
    loadable: bool,
    #[serde(serialize_with = "display_text")]
    content_sha256: &'a Sha256,
    machine: MachineFields<'a>,
    recipe: RecipeFields<'a>,
    version: VersionFields<'a>,
    types: &'a [TypeEntry],
    parameters: Vec<ParameterFields<'a>>,
}

/// The object `machine`.
#[derive(Serialize)]
struct MachineFields<'a> {
    #[serde(serialize_with = "display_text")]
    code: MachineCode,
    name: &'a str,
    plc_platform: &'static str,
}

/// The object `recipe`.
#[derive(Serialize)]
struct RecipeFields<'a> {
    reference_number: &'a str,
    name: &'a str,
}

/// The object `version`.
#[derive(Serialize)]
struct VersionFields<'a> {
    number: u32,
    status: ExportStatus,
    approved_at: String,
    approved_by: &'a str,
}

/// One entry of `parameters[]`: the fields of the hashed entry with `name_en` after `key`.
#[derive(Serialize)]
struct ParameterFields<'a> {
    #[serde(serialize_with = "display_text")]
    unit_path: &'a UnitPath,
    tag: &'a str,
    #[serde(serialize_with = "display_text")]
    key: ParameterKey,
    name_en: &'a str,
    type_id: TypeReference,
    s7_address: Option<String>,
    limits: &'a [LimitEntry],
    value: &'a OrderedValue,
}

/// Serialize a value as the JSON string of its `Display` text.
fn display_text<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    serializer.collect_str(value)
}

/// Write the export document v1 of a hashed body and its facts as pretty JSON.
///
/// The document has the fields of `api.md` section 6.1 in their fixed order. The indent is two
/// spaces, and the text has no newline at the end. The hashed body gives the fields "In hash".
/// The facts give the other fields. `name_en` of each parameter comes from `facts.names`.
///
/// # Errors
///
/// Returns `E-EXP-004` (`ExportError::Serialize`) when `facts.names` has no name for the key of a
/// parameter, when a time has a UTC year outside 0000 to 9999, or when `serde_json` fails.
pub fn write_document(body: &HashedBody, facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
    let parameters = body
        .parameters()
        .iter()
        .map(|entry| parameter_fields(entry, &facts.names))
        .collect::<Result<Vec<_>, _>>()?;
    let document = Document {
        schema_version: SCHEMA_VERSION,
        exported_at: utc_text(facts.exported_at)?,
        exported_by: facts.exported_by.as_str(),
        loadable: facts.loadable(),
        content_sha256: &facts.content_sha256,
        machine: MachineFields {
            code: body.machine_code(),
            name: facts.machine_name.as_str(),
            plc_platform: body.platform().as_str(),
        },
        recipe: RecipeFields {
            reference_number: body.reference_number().as_str(),
            name: facts.recipe_name.as_str(),
        },
        version: VersionFields {
            number: body.version_number(),
            status: facts.status,
            approved_at: utc_text(facts.approved_at)?,
            approved_by: facts.approved_by.as_str(),
        },
        types: body.types(),
        parameters,
    };
    Ok(serde_json::to_vec_pretty(&document)?)
}

/// Build the fields of one parameter with its `name_en`.
///
/// # Errors
///
/// Returns `E-EXP-004` when `names` has no name for the key of the entry.
fn parameter_fields<'a>(
    entry: &'a ParameterEntry,
    names: &'a BTreeMap<ParameterKey, Text>,
) -> Result<ParameterFields<'a>, ExportError> {
    let key = entry.key();
    let name_en = names.get(&key).ok_or_else(|| {
        ExportError::Serialize(serde_json::Error::custom(format_args!(
            "the parameter {key} has no name_en"
        )))
    })?;
    Ok(ParameterFields {
        unit_path: entry.unit_path(),
        tag: entry.tag().as_str(),
        key,
        name_en: name_en.as_str(),
        type_id: entry.type_id(),
        s7_address: entry.s7_address().map(|address| address.to_string()),
        limits: entry.limits(),
        value: entry.value(),
    })
}
