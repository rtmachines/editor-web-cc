//! SQL of the `TypeCatalog` methods (task 9.4, `data-model.md` sections 1.4, 3.3, 3.4, 6).
//!
//! Each function runs on the connection of the open transaction. No function takes a row lock,
//! because `recipe_app` has no `UPDATE` grant on `plc_type`. `lock_derived_types` takes the
//! class-2 advisory locks instead.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use recipe_app::error::StorageError;
use recipe_app::ports::{DerivedTypeLock, Ensured, RowImage};
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafClass, MachineId, MemberName, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeMember, TypeName,
};
use serde_json::Value;
use sqlx::PgConnection;
use sqlx::Postgres;
use sqlx::postgres::PgArguments;
use uuid::Uuid;

use crate::error::{check_violation, corrupt, map_error};

/// The class of the advisory locks of derived types (`data-model.md` section 1.4).
const DERIVED_TYPE_LOCK_CLASS: i32 = 2;

/// The name of the primary key of `plc_type`. `ON CONFLICT DO NOTHING` also skips its conflicts.
const TYPE_PKEY: &str = "plc_type_pkey";

const SELECT_TYPES: &str = "\
SELECT id, kind, name, platforms, size_bytes, string_length, leaf_class, element_type_id, \
dim_lower, dim_upper, machine_id, revision, supersedes_type_id \
FROM plc_type WHERE machine_id IS NULL OR machine_id = $1 ORDER BY id";

/// The member rows of all structs of a machine. A global row is never a struct.
const SELECT_MEMBERS: &str = "\
SELECT m.struct_type_id, m.member_name, m.member_type_id \
FROM plc_type_member m JOIN plc_type t ON t.id = m.struct_type_id \
WHERE t.machine_id = $1 ORDER BY m.struct_type_id, m.ordinal";

const LOCK_DERIVED_TYPE: &str = "SELECT pg_advisory_xact_lock($1, hashtext($2::text || ':' || $3))";

/// A new `STRING(n)`, `WSTRING(n)`, or array row. A conflict on any unique index skips the row.
const INSERT_DERIVED: &str = "\
INSERT INTO plc_type (id, kind, name, platforms, size_bytes, string_length, leaf_class, \
element_type_id, dim_lower, dim_upper, machine_id, revision, supersedes_type_id) \
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
ON CONFLICT DO NOTHING RETURNING to_jsonb(plc_type)";

/// A new struct row. A conflict is an error, and the error map names the unique index.
const INSERT_STRUCT: &str = "\
INSERT INTO plc_type (id, kind, name, platforms, size_bytes, string_length, leaf_class, \
element_type_id, dim_lower, dim_upper, machine_id, revision, supersedes_type_id) \
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
RETURNING to_jsonb(plc_type)";

/// All member rows of a struct in one statement. `unnest` of several arrays walks them in step,
/// so the row order and the ordinals follow the input.
const INSERT_MEMBERS: &str = "\
INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
SELECT $1, t.ordinal, t.member_name, t.member_type_id \
FROM unnest($2::int2[], $3::text[], $4::uuid[]) AS t(ordinal, member_name, member_type_id)";

/// The key of `plc_type_string_unique`. The `OR` keeps the index usable for a machine row.
const SELECT_EXISTING_STRING: &str = "\
SELECT id FROM plc_type WHERE kind = 'primitive' AND name = $2 AND string_length = $3 \
AND (machine_id = $1 OR (machine_id IS NULL AND $1::uuid IS NULL))";

/// The key of `plc_type_array_unique`.
const SELECT_EXISTING_ARRAY: &str = "\
SELECT id FROM plc_type WHERE kind = 'array' AND machine_id = $1 AND element_type_id = $2 \
AND dim_lower = $3 AND dim_upper = $4";

/// The struct row with its member rows ordered by `ordinal`. A separate statement reads it
/// after the inserts, because a data-changing CTE does not see its sibling statements (D5).
const SELECT_STRUCT_IMAGE: &str = "\
SELECT to_jsonb(t) || jsonb_build_object('members', COALESCE( \
(SELECT jsonb_agg(to_jsonb(m) ORDER BY m.ordinal) FROM plc_type_member m \
WHERE m.struct_type_id = t.id), '[]'::jsonb)) \
FROM plc_type t WHERE t.id = $1";

#[derive(sqlx::FromRow)]
struct TypeRecord {
    id: Uuid,
    kind: String,
    name: Option<String>,
    platforms: Vec<String>,
    size_bytes: Option<i32>,
    string_length: Option<i16>,
    leaf_class: Option<String>,
    element_type_id: Option<Uuid>,
    dim_lower: Option<Vec<i32>>,
    dim_upper: Option<Vec<i32>>,
    machine_id: Option<Uuid>,
    revision: i32,
    supersedes_type_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct MemberRecord {
    struct_type_id: Uuid,
    member_name: String,
    member_type_id: Uuid,
}

/// The columns of a new `plc_type` row.
struct NewTypeRow {
    id: Uuid,
    kind: &'static str,
    name: Option<String>,
    platforms: Vec<String>,
    size_bytes: Option<i32>,
    string_length: Option<i16>,
    leaf_class: Option<&'static str>,
    element_type_id: Option<Uuid>,
    dim_lower: Option<Vec<i32>>,
    dim_upper: Option<Vec<i32>>,
    machine_id: Option<Uuid>,
    revision: i32,
    supersedes_type_id: Option<Uuid>,
}

/// An insert statement that returns the image of its row.
type ImageQuery = sqlx::query::QueryScalar<'static, Postgres, Value, PgArguments>;

/// Name a stored row that the domain rejects.
fn bad(reason: impl fmt::Debug) -> StorageError {
    corrupt("plc_type", format!("{reason:?}"))
}

/// Return the value of a column that the kind of the row requires.
fn required<T>(value: Option<T>, column: &'static str) -> Result<T, StorageError> {
    value.ok_or_else(|| corrupt("plc_type", format!("the column {column} is empty")))
}

/// Build the shape of a stored row.
fn shape_of(row: &TypeRecord, members: Vec<TypeMember>) -> Result<RecordShape, StorageError> {
    match row.kind.as_str() {
        "primitive" => {
            let name = required(row.name.as_deref(), "name")?;
            let leaf_class = required(row.leaf_class.as_deref(), "leaf_class")?;
            Ok(RecordShape::Primitive {
                primitive: Primitive::parse(name).map_err(bad)?,
                leaf_class: LeafClass::parse(leaf_class).map_err(bad)?,
                size_bytes: u32::try_from(required(row.size_bytes, "size_bytes")?).map_err(bad)?,
                string_length: row
                    .string_length
                    .map(u16::try_from)
                    .transpose()
                    .map_err(bad)?,
            })
        }
        "array" => {
            let lower = required(row.dim_lower.as_ref(), "dim_lower")?;
            let upper = required(row.dim_upper.as_ref(), "dim_upper")?;
            let dimensions = lower
                .iter()
                .zip(upper)
                .map(|(lower, upper)| ArrayDimension::parse(*lower, *upper))
                .collect::<Result<Vec<_>, _>>()
                .map_err(bad)?;
            Ok(RecordShape::Array {
                element: PlcTypeId::from(required(row.element_type_id, "element_type_id")?),
                dimensions: ArrayDimensions::parse(dimensions).map_err(bad)?,
            })
        }
        "struct" => Ok(RecordShape::Struct {
            name: TypeName::parse(required(row.name.as_deref(), "name")?).map_err(bad)?,
            revision: u32::try_from(row.revision).map_err(bad)?,
            supersedes: row.supersedes_type_id.map(PlcTypeId::from),
            members,
        }),
        other => Err(corrupt("plc_type", format!("the kind {other}"))),
    }
}

/// Build the domain type of a stored row and its member rows.
fn build_type(row: TypeRecord, members: Vec<TypeMember>) -> Result<PlcType, StorageError> {
    let platforms = row
        .platforms
        .iter()
        .map(|platform| PlcPlatform::parse(platform))
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(bad)?;
    let record = PlcTypeRecord {
        id: PlcTypeId::from(row.id),
        machine: row.machine_id.map(MachineId::from),
        platforms,
        shape: shape_of(&row, members)?,
    };
    PlcType::try_from(record).map_err(bad)
}

/// Group the member rows by struct. The rows arrive ordered by struct and `ordinal`.
fn group_members(rows: Vec<MemberRecord>) -> Result<BTreeMap<Uuid, Vec<TypeMember>>, StorageError> {
    let mut grouped: BTreeMap<Uuid, Vec<TypeMember>> = BTreeMap::new();
    for row in rows {
        let member = TypeMember {
            name: MemberName::parse(&row.member_name).map_err(bad)?,
            type_id: PlcTypeId::from(row.member_type_id),
        };
        grouped.entry(row.struct_type_id).or_default().push(member);
    }
    Ok(grouped)
}

/// Read the global rows and the rows of `machine`. The statements take no lock.
pub(super) async fn rows(
    connection: &mut PgConnection,
    machine: MachineId,
) -> Result<Vec<PlcType>, StorageError> {
    let records = sqlx::query_as::<_, TypeRecord>(SELECT_TYPES)
        .bind(*machine.as_uuid())
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let member_rows = sqlx::query_as::<_, MemberRecord>(SELECT_MEMBERS)
        .bind(*machine.as_uuid())
        .fetch_all(&mut *connection)
        .await
        .map_err(map_error)?;
    let mut members = group_members(member_rows)?;
    records
        .into_iter()
        .map(|record| {
            let own = members.remove(&record.id).unwrap_or_default();
            build_type(record, own)
        })
        .collect()
}

/// Take the class-2 advisory lock of each key, one statement for each key, in the given order.
pub(super) async fn lock_derived(
    connection: &mut PgConnection,
    machine: MachineId,
    keys: &[DerivedTypeLock],
) -> Result<(), StorageError> {
    for key in keys {
        sqlx::query(LOCK_DERIVED_TYPE)
            .bind(DERIVED_TYPE_LOCK_CLASS)
            .bind(machine.to_string())
            .bind(key.key_text())
            .execute(&mut *connection)
            .await
            .map_err(map_error)?;
    }
    Ok(())
}

/// Convert a count to the column type or name the row as corrupt.
fn column<T, U>(value: T) -> Result<U, StorageError>
where
    U: TryFrom<T>,
    U::Error: fmt::Debug,
{
    U::try_from(value).map_err(bad)
}

/// Build the columns that every new row shares, with the defaults of a primitive row.
fn base_row(plc_type: &PlcType) -> NewTypeRow {
    NewTypeRow {
        id: *plc_type.id().as_uuid(),
        kind: "primitive",
        name: None,
        platforms: plc_type
            .platforms()
            .iter()
            .map(|platform| platform.as_str().to_owned())
            .collect(),
        size_bytes: None,
        string_length: None,
        leaf_class: None,
        element_type_id: None,
        dim_lower: None,
        dim_upper: None,
        machine_id: plc_type.machine().map(|machine| *machine.as_uuid()),
        revision: 1,
        supersedes_type_id: None,
    }
}

/// Build the columns of a new row from a valid domain type.
fn new_row(plc_type: &PlcType) -> Result<NewTypeRow, StorageError> {
    let mut row = base_row(plc_type);
    match plc_type.shape() {
        RecordShape::Primitive {
            primitive,
            leaf_class,
            size_bytes,
            string_length,
        } => {
            row.name = Some(primitive.name().to_owned());
            row.size_bytes = Some(column(*size_bytes)?);
            row.string_length = string_length.map(column).transpose()?;
            row.leaf_class = Some(leaf_class.as_sql());
        }
        RecordShape::Array {
            element,
            dimensions,
        } => {
            row.kind = "array";
            row.element_type_id = Some(*element.as_uuid());
            row.dim_lower = Some(dimensions.iter().map(|d| d.lower()).collect());
            row.dim_upper = Some(dimensions.iter().map(|d| d.upper()).collect());
        }
        RecordShape::Struct {
            name,
            revision,
            supersedes,
            ..
        } => {
            row.kind = "struct";
            row.name = Some(name.as_str().to_owned());
            row.revision = column(*revision)?;
            row.supersedes_type_id = supersedes.map(|id| *id.as_uuid());
        }
    }
    Ok(row)
}

/// Bind the columns of a new row to an insert statement.
fn bind_row(statement: &'static str, row: &NewTypeRow) -> ImageQuery {
    sqlx::query_scalar(statement)
        .bind(row.id)
        .bind(row.kind)
        .bind(row.name.clone())
        .bind(row.platforms.clone())
        .bind(row.size_bytes)
        .bind(row.string_length)
        .bind(row.leaf_class)
        .bind(row.element_type_id)
        .bind(row.dim_lower.clone())
        .bind(row.dim_upper.clone())
        .bind(row.machine_id)
        .bind(row.revision)
        .bind(row.supersedes_type_id)
}

/// Read the id of the row that holds the key of `candidate`.
async fn existing_id(
    connection: &mut PgConnection,
    candidate: &NewTypeRow,
) -> Result<Option<Uuid>, StorageError> {
    let found = match candidate.kind {
        "array" => {
            sqlx::query_scalar(SELECT_EXISTING_ARRAY)
                .bind(candidate.machine_id)
                .bind(candidate.element_type_id)
                .bind(candidate.dim_lower.clone())
                .bind(candidate.dim_upper.clone())
                .fetch_optional(connection)
                .await
        }
        _ => {
            sqlx::query_scalar(SELECT_EXISTING_STRING)
                .bind(candidate.machine_id)
                .bind(candidate.name.clone())
                .bind(candidate.string_length)
                .fetch_optional(connection)
                .await
        }
    };
    found.map_err(map_error)
}

/// Tell whether the port accepts the row: a `STRING(n)`, a `WSTRING(n)`, or an array.
const fn is_derived(plc_type: &PlcType) -> bool {
    match plc_type.shape() {
        RecordShape::Primitive { primitive, .. } => {
            matches!(primitive, Primitive::String | Primitive::Wstring)
        }
        RecordShape::Array { .. } => true,
        RecordShape::Struct { .. } => false,
    }
}

/// Insert a derived row, or find the row that holds the same key.
pub(super) async fn ensure_derived(
    connection: &mut PgConnection,
    record: PlcTypeRecord,
) -> Result<Ensured, StorageError> {
    let candidate = PlcType::try_from(record).map_err(|_| check_violation())?;
    if !is_derived(&candidate) {
        return Err(check_violation());
    }
    let row = new_row(&candidate)?;
    let inserted = bind_row(INSERT_DERIVED, &row)
        .fetch_optional(&mut *connection)
        .await
        .map_err(map_error)?;
    if let Some(image) = inserted {
        return Ok(Ensured {
            id: candidate.id(),
            created: Some(RowImage::new(image)),
        });
    }
    match existing_id(connection, &row).await? {
        Some(id) => Ok(Ensured {
            id: PlcTypeId::from(id),
            created: None,
        }),
        None => Err(StorageError::Duplicate {
            constraint: TYPE_PKEY.to_owned(),
        }),
    }
}

/// Insert a struct row and its member rows, then read the image with the members.
pub(super) async fn insert_struct(
    connection: &mut PgConnection,
    plc_type: &PlcType,
) -> Result<RowImage, StorageError> {
    let RecordShape::Struct { members, .. } = plc_type.shape() else {
        return Err(check_violation());
    };
    let row = new_row(plc_type)?;
    bind_row(INSERT_STRUCT, &row)
        .fetch_one(&mut *connection)
        .await
        .map_err(map_error)?;
    let ordinals: Vec<i16> = (1_i16..).take(members.len()).collect();
    let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
    let types: Vec<Uuid> = members
        .iter()
        .map(|member| *member.type_id.as_uuid())
        .collect();
    sqlx::query(INSERT_MEMBERS)
        .bind(*plc_type.id().as_uuid())
        .bind(ordinals)
        .bind(names)
        .bind(types)
        .execute(&mut *connection)
        .await
        .map_err(map_error)?;
    let image: Value = sqlx::query_scalar(SELECT_STRUCT_IMAGE)
        .bind(*plc_type.id().as_uuid())
        .fetch_one(connection)
        .await
        .map_err(map_error)?;
    Ok(RowImage::new(image))
}
