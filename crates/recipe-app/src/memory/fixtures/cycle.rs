//! The cycles of stored type rows that the tests of a storage defect use (ADR-0195 (w)).

use std::collections::BTreeSet;

use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, MemberName, PlcPlatform, PlcType, PlcTypeRecord, RecordShape,
    TypeMember, TypeName,
};

use super::{Fixture, mid, tid};

/// The row number of `ST_Left`, the root of `seed_struct_cycle`.
pub(crate) const STRUCT_CYCLE_ROOT: u128 = 940;
/// The row number of the first array of `seed_array_cycle`, the root of that cycle.
pub(crate) const ARRAY_CYCLE_ROOT: u128 = 950;

/// Store `ST_Left@1` and `ST_Right@1` and return the two rows. Each struct has the other as member.
///
/// `owner` holds the number of the machine identifier and the platform of the rows. The rows
/// belong to that machine. The root is `STRUCT_CYCLE_ROOT`. Its resolution gives `TypeCycle`.
pub(crate) fn seed_struct_cycle(fixture: &Fixture, owner: (u128, PlcPlatform)) -> Vec<PlcType> {
    let rows = vec![
        struct_member_row(owner, (STRUCT_CYCLE_ROOT, "ST_Left"), ("right", 941)),
        struct_member_row(owner, (941, "ST_Right"), ("left", STRUCT_CYCLE_ROOT)),
    ];
    store(fixture, &rows);
    rows
}

/// Store two arrays that contain each other and return the two rows.
///
/// `owner` is as for `seed_struct_cycle`. The root is `ARRAY_CYCLE_ROOT`. No struct is on the
/// cycle, so its resolution gives `E-PLT-013` with the reason `RowShape`.
pub(crate) fn seed_array_cycle(fixture: &Fixture, owner: (u128, PlcPlatform)) -> Vec<PlcType> {
    let rows = vec![
        array_of_row(owner, ARRAY_CYCLE_ROOT, 951),
        array_of_row(owner, 951, ARRAY_CYCLE_ROOT),
    ];
    store(fixture, &rows);
    rows
}

fn store(fixture: &Fixture, rows: &[PlcType]) {
    for row in rows {
        fixture.store.seed_type(row);
    }
}

fn struct_member_row(
    (machine, platform): (u128, PlcPlatform),
    (id, name): (u128, &str),
    (member, member_type): (&str, u128),
) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(machine)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision: 1,
            supersedes: None,
            members: vec![TypeMember {
                name: MemberName::parse(member).unwrap(),
                type_id: tid(member_type),
            }],
        },
    })
    .unwrap()
}

fn array_of_row((machine, platform): (u128, PlcPlatform), id: u128, element: u128) -> PlcType {
    let dimensions = vec![ArrayDimension::parse(1, 4).unwrap()];
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(machine)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Array {
            element: tid(element),
            dimensions: ArrayDimensions::parse(dimensions).unwrap(),
        },
    })
    .unwrap()
}
