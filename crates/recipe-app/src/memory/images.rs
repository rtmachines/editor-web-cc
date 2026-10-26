//! Row images of the in-memory adapters (design D9, ADR-0029).
//!
//! The keys are the column names of `data-model.md` section 3. A timestamp is the
//! `Display` text of the instant. A struct image holds its member rows.

use recipe_domain::plant::RecordShape;
use recipe_domain::timestamp::Timestamp;
use serde_json::{Map, Value};

use super::rows::{MachineRow, TypeRow, UnitRow};
use crate::ports::RowImage;

/// The revision of a row that has no revision column value: the column default.
const DEFAULT_REVISION: u32 = 1;

/// Build a JSON object from pairs. The order of the pairs does not matter.
pub(super) fn object(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect::<Map<String, Value>>(),
    )
}

pub(super) fn time_value(instant: Timestamp) -> Value {
    Value::String(instant.as_offset_date_time().to_string())
}

pub(super) fn optional_time(instant: Option<Timestamp>) -> Value {
    instant.map_or(Value::Null, time_value)
}

/// Return the image of a `machine` row.
pub(super) fn machine_image(row: &MachineRow) -> RowImage {
    RowImage::new(object(vec![
        ("id", Value::String(row.id.to_string())),
        ("code", Value::String(row.code.to_string())),
        ("name", Value::String(row.name.to_string())),
        ("plc_platform", Value::String(row.platform.to_string())),
        ("archived_at", optional_time(row.archived_at)),
        ("created_at", time_value(row.created_at)),
        ("updated_at", time_value(row.updated_at)),
    ]))
}

/// Return the image of a `unit` row.
pub(super) fn unit_image(row: &UnitRow) -> RowImage {
    let parent = row
        .path
        .parent()
        .map_or(Value::Null, |parent| Value::String(parent.to_string()));
    RowImage::new(object(vec![
        ("id", Value::String(row.id.to_string())),
        ("machine_id", Value::String(row.machine.to_string())),
        ("path", Value::String(row.path.to_string())),
        ("parent_path", parent),
        ("depth", Value::from(row.path.depth())),
        ("name", Value::String(row.name.to_string())),
        ("archived_at", optional_time(row.archived_at)),
        ("created_at", time_value(row.created_at)),
        ("updated_at", time_value(row.updated_at)),
    ]))
}

/// The columns that depend on the shape of a type row.
struct ShapeColumns {
    kind: &'static str,
    name: Value,
    size_bytes: Value,
    string_length: Value,
    leaf_class: Value,
    element_type_id: Value,
    dim_lower: Value,
    dim_upper: Value,
    revision: u32,
    supersedes: Value,
}

/// Return the columns of a row of `kind` with no shape column set.
///
/// Each shape arm overrides only the columns that its kind uses.
fn unshaped(kind: &'static str) -> ShapeColumns {
    ShapeColumns {
        kind,
        name: Value::Null,
        size_bytes: Value::Null,
        string_length: Value::Null,
        leaf_class: Value::Null,
        element_type_id: Value::Null,
        dim_lower: Value::Null,
        dim_upper: Value::Null,
        revision: DEFAULT_REVISION,
        supersedes: Value::Null,
    }
}

fn shape_columns(shape: &RecordShape) -> ShapeColumns {
    match shape {
        RecordShape::Primitive {
            primitive,
            leaf_class,
            size_bytes,
            string_length,
        } => ShapeColumns {
            name: Value::String(primitive.name().to_owned()),
            size_bytes: Value::from(*size_bytes),
            string_length: Value::from(*string_length),
            leaf_class: Value::String(leaf_class.as_sql().to_owned()),
            ..unshaped("primitive")
        },
        RecordShape::Array {
            element,
            dimensions,
        } => ShapeColumns {
            element_type_id: Value::String(element.to_string()),
            dim_lower: dimensions.iter().map(|d| Value::from(d.lower())).collect(),
            dim_upper: dimensions.iter().map(|d| Value::from(d.upper())).collect(),
            ..unshaped("array")
        },
        RecordShape::Struct {
            name,
            revision,
            supersedes,
            ..
        } => ShapeColumns {
            name: Value::String(name.to_string()),
            revision: *revision,
            supersedes: supersedes.map_or(Value::Null, |id| Value::String(id.to_string())),
            ..unshaped("struct")
        },
    }
}

/// Return the member rows of a struct as an array, and `None` for any other row.
///
/// A member row has the columns of `plc_type_member` (`data-model.md` section 3.4). The time
/// of a member row is the time of its struct row, because both rows come from one command.
fn member_rows(row: &TypeRow) -> Option<Value> {
    let plc_type = &row.plc_type;
    let RecordShape::Struct { members, .. } = plc_type.shape() else {
        return None;
    };
    let rows = (1_u32..)
        .zip(members)
        .map(|(ordinal, member)| {
            object(vec![
                ("struct_type_id", Value::String(plc_type.id().to_string())),
                ("created_at", time_value(row.created_at)),
                ("ordinal", Value::from(ordinal)),
                ("member_name", Value::String(member.name.to_string())),
                ("member_type_id", Value::String(member.type_id.to_string())),
            ])
        })
        .collect();
    Some(rows)
}

/// Return the image of a `plc_type` row. A struct image holds its member rows.
pub(super) fn type_image(row: &TypeRow) -> RowImage {
    let plc_type = &row.plc_type;
    let columns = shape_columns(plc_type.shape());
    let platforms = plc_type
        .platforms()
        .iter()
        .map(|platform| Value::String(platform.to_string()))
        .collect();
    let mut pairs = vec![
        ("id", Value::String(plc_type.id().to_string())),
        ("kind", Value::String(columns.kind.to_owned())),
        ("name", columns.name),
        ("platforms", platforms),
        ("size_bytes", columns.size_bytes),
        ("string_length", columns.string_length),
        ("leaf_class", columns.leaf_class),
        ("element_type_id", columns.element_type_id),
        ("dim_lower", columns.dim_lower),
        ("dim_upper", columns.dim_upper),
        (
            "machine_id",
            plc_type
                .machine()
                .map_or(Value::Null, |id| Value::String(id.to_string())),
        ),
        ("revision", Value::from(columns.revision)),
        ("supersedes_type_id", columns.supersedes),
        ("created_at", time_value(row.created_at)),
    ];
    if let Some(members) = member_rows(row) {
        pairs.push(("members", members));
    }
    RowImage::new(object(pairs))
}
