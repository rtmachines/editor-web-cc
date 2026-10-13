//! Shared builders of the tests of the module `export`.
//!
//! Every builder takes the platform of its machine, so a test never mixes platforms.

use std::collections::BTreeSet;

use uuid::Uuid;

use crate::plant::{
    ArrayDimension, ArrayDimensions, MachineId, MemberName, PlatformTypes, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeMember, TypeName, TypeTrees,
};

/// The identifier of `STRING(16)` in the example of `api.md` section 6.2.
pub(crate) const STRING_16: &str = "5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f";
/// The identifier of `ZoneTemp@2` in the example of `api.md` section 6.2.
pub(crate) const ZONE_TEMP: &str = "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f";
/// The identifier of `ARRAY[1..4] OF STRING(16)`.
pub(crate) const STRING_ARRAY: &str = "30000000-0000-4000-8000-000000000001";
/// The identifier of `STRING(8)`.
pub(crate) const STRING_8: &str = "10000000-0000-4000-8000-000000000008";
/// The identifier of a second struct that holds `STRING(16)`.
pub(crate) const LABEL_BOX: &str = "40000000-0000-4000-8000-000000000001";
/// The identifier of `INT`.
pub(crate) const INT: &str = "00000000-0000-4000-8000-000000000001";
/// The identifier of `REAL`.
pub(crate) const REAL: &str = "00000000-0000-4000-8000-000000000002";
/// The identifier of `BOOL`.
pub(crate) const BOOL: &str = "00000000-0000-4000-8000-000000000003";
/// The identifier of `TIME`.
pub(crate) const TIME: &str = "00000000-0000-4000-8000-000000000004";

/// Build a type identifier from its text.
pub(crate) fn type_id(text: &str) -> PlcTypeId {
    PlcTypeId::from(Uuid::parse_str(text).unwrap())
}

/// The machine that owns every machine row.
pub(crate) fn machine_id() -> MachineId {
    MachineId::from(Uuid::from_u128(0xAA))
}

fn row(id: &str, owner: Option<MachineId>, platform: PlcPlatform, shape: RecordShape) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: type_id(id),
        machine: owner,
        platforms: BTreeSet::from([platform]),
        shape,
    })
    .unwrap()
}

fn seed(id: &str, platform: PlcPlatform, primitive: Primitive) -> PlcType {
    row(
        id,
        None,
        platform,
        RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    )
}

fn string_row(id: &str, platform: PlcPlatform, length: u16) -> PlcType {
    row(
        id,
        Some(machine_id()),
        platform,
        RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: u32::from(length).checked_add(2).unwrap(),
            string_length: Some(length),
        },
    )
}

fn array_row(id: &str, platform: PlcPlatform, element: &str, bounds: (i32, i32)) -> PlcType {
    let dimension = ArrayDimension::parse(bounds.0, bounds.1).unwrap();
    row(
        id,
        Some(machine_id()),
        platform,
        RecordShape::Array {
            element: type_id(element),
            dimensions: ArrayDimensions::parse(vec![dimension]).unwrap(),
        },
    )
}

fn struct_row(
    id: &str,
    platform: PlcPlatform,
    (name, revision): (&str, u32),
    members: &[(&str, &str)],
) -> PlcType {
    row(
        id,
        Some(machine_id()),
        platform,
        RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision,
            supersedes: None,
            members: members
                .iter()
                .map(|(member, type_text)| TypeMember {
                    name: MemberName::parse(member).unwrap(),
                    type_id: type_id(type_text),
                })
                .collect(),
        },
    )
}

/// Every test type of one machine on `platform`.
pub(crate) fn catalog(platform: PlcPlatform) -> PlatformTypes {
    let rows = vec![
        seed(INT, platform, Primitive::Int),
        seed(REAL, platform, Primitive::Real),
        seed(BOOL, platform, Primitive::Bool),
        seed(TIME, platform, Primitive::Time),
        string_row(STRING_16, platform, 16),
        string_row(STRING_8, platform, 8),
        array_row(STRING_ARRAY, platform, STRING_16, (1, 4)),
        struct_row(
            ZONE_TEMP,
            platform,
            ("ZoneTemp", 2),
            &[("setpoint", REAL), ("label", STRING_16)],
        ),
        struct_row(LABEL_BOX, platform, ("LabelBox", 1), &[("text", STRING_16)]),
    ];
    PlatformTypes::collect(machine_id(), platform, rows).unwrap()
}

/// The trees of every test type on `platform`.
pub(crate) fn trees(platform: PlcPlatform) -> TypeTrees {
    let ids = [
        INT,
        REAL,
        BOOL,
        TIME,
        STRING_16,
        STRING_8,
        STRING_ARRAY,
        ZONE_TEMP,
        LABEL_BOX,
    ]
    .map(type_id);
    TypeTrees::resolve(ids, &catalog(platform))
}
