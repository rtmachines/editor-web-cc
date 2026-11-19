//! The struct rows that the tests of the plant data seed without a check (ADR-0037 u).
//!
//! The module solves one problem: the type and parameter page tests need a type tree that no
//! request can make, such as a cycle. One module keeps the seed helpers in one place.

use std::collections::BTreeSet;

use recipe_domain::plant::{
    MachineId, MemberName, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, RecordShape, TypeMember,
    TypeName,
};
use uuid::Uuid;

use super::World;

/// The identifier of the struct row that a test seeds with the number `number`.
pub(crate) fn struct_id(number: u128) -> PlcTypeId {
    PlcTypeId::from(Uuid::from_u128(
        0x7357_0e00_0000_0000_0000_0000_0000_0000 | number,
    ))
}

/// The machine that owns a seeded row, and the platform of that machine.
pub(crate) type RowOwner = (MachineId, PlcPlatform);

/// Seed the struct `name@1` of `M7` with the members `members` and return its identifier.
///
/// `MemoryStore::seed_type` writes the row without a check. The member types may name rows that
/// the test seeds later, so a test can build a tree that no request can make.
pub(crate) fn seed_struct(
    world: &World,
    number: u128,
    name: &str,
    members: &[(&str, PlcTypeId)],
) -> PlcTypeId {
    let owner = (world.ids.m7, PlcPlatform::S7_300);
    seed_struct_for(world, owner, number, (name, 1), members)
}

/// Seed the struct `name@revision` of `owner` with the members `members` and return its identifier.
pub(crate) fn seed_struct_for(
    world: &World,
    owner: RowOwner,
    number: u128,
    (name, revision): (&str, u32),
    members: &[(&str, PlcTypeId)],
) -> PlcTypeId {
    let id = struct_id(number);
    let row = PlcType::try_from(PlcTypeRecord {
        id,
        machine: Some(owner.0),
        platforms: BTreeSet::from([owner.1]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision,
            supersedes: None,
            members: members
                .iter()
                .map(|(member, type_id)| TypeMember {
                    name: MemberName::parse(member).unwrap(),
                    type_id: *type_id,
                })
                .collect(),
        },
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
    id
}

/// Seed the structs `ST_Alpha@1` and `ST_Beta@1` of `M7`, which contain each other.
///
/// The store then holds a cycle that no request can make (ADR-0037 u).
pub(crate) fn seed_cycle(world: &World) {
    let alpha = struct_id(1);
    let beta = struct_id(2);
    seed_struct(world, 1, "ST_Alpha", &[("beta", beta)]);
    seed_struct(world, 2, "ST_Beta", &[("alpha", alpha)]);
}
