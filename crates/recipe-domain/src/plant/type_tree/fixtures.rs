//! Row builders that the tests of `type_tree` share.

use std::collections::BTreeSet;

use uuid::Uuid;

use crate::plant::{
    ArrayDimension, ArrayDimensions, MachineId, MemberName, PlatformTypes, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeMember, TypeName,
};

/// Platform of every machine row that these builders create.
pub(crate) const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

/// Build a type identifier from a number.
pub(crate) fn id(n: u128) -> PlcTypeId {
    PlcTypeId::from(Uuid::from_u128(n))
}

/// The machine that owns every machine row that these builders create.
pub(crate) fn machine() -> MachineId {
    MachineId::from(Uuid::from_u128(0xAA))
}

/// Another machine, whose rows must stay invisible to `machine()`.
pub(crate) fn other_machine() -> MachineId {
    MachineId::from(Uuid::from_u128(0xBB))
}

fn row(record: PlcTypeRecord) -> PlcType {
    PlcType::try_from(record).expect("fixture row is valid")
}

/// A seed row of a fixed-size primitive on all platforms.
pub(crate) fn primitive_row(n: u128, primitive: Primitive) -> PlcType {
    row(PlcTypeRecord {
        id: id(n),
        machine: None,
        platforms: PlcPlatform::ALL.into_iter().collect(),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().expect("fixed size"),
            string_length: None,
        },
    })
}

/// A seed `INT` row on one given platform only.
pub(crate) fn int_row_on(n: u128, platform: PlcPlatform) -> PlcType {
    row(PlcTypeRecord {
        id: id(n),
        machine: None,
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Primitive {
            primitive: Primitive::Int,
            leaf_class: Primitive::Int.leaf_class(),
            size_bytes: 2,
            string_length: None,
        },
    })
}

/// A machine `STRING(length)` row on `PLATFORM`.
pub(crate) fn string_row(n: u128, length: u16) -> PlcType {
    let size_bytes = u32::from(length).checked_add(2).expect("small length");
    row(PlcTypeRecord {
        id: id(n),
        machine: Some(machine()),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes,
            string_length: Some(length),
        },
    })
}

/// Build dimensions from `(lower, upper)` pairs.
pub(crate) fn dimensions(bounds: &[(i32, i32)]) -> ArrayDimensions {
    let list = bounds
        .iter()
        .map(|(lower, upper)| ArrayDimension::parse(*lower, *upper).expect("valid bounds"))
        .collect();
    ArrayDimensions::parse(list).expect("one to three dimensions")
}

/// A machine array row on `PLATFORM`.
pub(crate) fn array_row(n: u128, element: u128, bounds: &[(i32, i32)]) -> PlcType {
    row(PlcTypeRecord {
        id: id(n),
        machine: Some(machine()),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Array {
            element: id(element),
            dimensions: dimensions(bounds),
        },
    })
}

/// A machine `WSTRING(length)` row on `PLATFORM`.
pub(crate) fn wstring_row(n: u128, length: u16) -> PlcType {
    let size_bytes = u32::from(length)
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(4))
        .expect("small length");
    row(PlcTypeRecord {
        id: id(n),
        machine: Some(machine()),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Primitive {
            primitive: Primitive::Wstring,
            leaf_class: Primitive::Wstring.leaf_class(),
            size_bytes,
            string_length: Some(length),
        },
    })
}

/// A machine struct row on `PLATFORM`.
pub(crate) fn struct_row(n: u128, name: &str, members: &[(&str, u128)]) -> PlcType {
    struct_row_of(machine(), n, name, members)
}

/// A struct row of the given machine on `PLATFORM`.
pub(crate) fn struct_row_of(
    owner: MachineId,
    n: u128,
    name: &str,
    members: &[(&str, u128)],
) -> PlcType {
    row(PlcTypeRecord {
        id: id(n),
        machine: Some(owner),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).expect("valid name"),
            revision: 1,
            supersedes: None,
            members: members
                .iter()
                .map(|(member, type_id)| TypeMember {
                    name: MemberName::parse(member).expect("valid member"),
                    type_id: id(*type_id),
                })
                .collect(),
        },
    })
}

/// Collect rows into the types of `machine()` on `PLATFORM`.
pub(crate) fn types(rows: Vec<PlcType>) -> PlatformTypes {
    PlatformTypes::collect(machine(), PLATFORM, rows).expect("fixture ids are unique")
}
