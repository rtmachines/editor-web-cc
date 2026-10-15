//! Machines that the tests of the plant queries share.
//!
//! The helpers build machines with a known tree. A test stores the result with
//! `MemoryStore::seed_machine`.

use std::collections::BTreeSet;

use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafClass, Machine, MachineCode, MemberName, PlcPlatform,
    PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeMember, TypeName, UnitPath,
    UnitUsage,
};

use crate::memory::fixtures::{mid, text, tid, timestamp, uid};

/// A `TwinCat3` machine with the live main unit `U1`.
///
/// The machine identifier is `n`. The identifier of `U1` is `n * 100 + 1`.
pub(super) fn machine_named(n: u128, code: &str, name: &str) -> Machine {
    machine_of(PlcPlatform::TwinCat3, n, code, name)
}

/// A machine of `platform` with the live main unit `U1`.
pub(super) fn machine_of(platform: PlcPlatform, n: u128, code: &str, name: &str) -> Machine {
    let unit_id = n
        .checked_mul(100)
        .and_then(|base| base.checked_add(1))
        .unwrap();
    Machine::create(
        mid(n),
        MachineCode::parse(code).unwrap(),
        text(name),
        platform,
        (uid(unit_id), text("Main")),
    )
}

/// Add live units with the given paths, in the order of the slice.
///
/// A parent must come before its children. The unit with the position `k` in the machine has
/// the identifier `n * 100 + k`, where `n` is the number of the machine identifier and `U1`
/// has the position 1. Each new unit is named `Unit <path>`.
pub(super) fn with_units(mut machine: Machine, paths: &[&str]) -> Machine {
    let known = u128::try_from(machine.units().count()).unwrap();
    let first = machine
        .id()
        .as_uuid()
        .as_u128()
        .checked_mul(100)
        .and_then(|base| base.checked_add(known))
        .and_then(|base| base.checked_add(1))
        .unwrap();
    for (id, path_text) in (first..).zip(paths) {
        let path = UnitPath::parse(path_text).unwrap();
        let parent = path.parent();
        let segment = path.segments().last().unwrap().to_string();
        machine
            .add_unit(
                uid(id),
                parent.as_ref(),
                &segment,
                text(&format!("Unit {path_text}")),
            )
            .unwrap();
    }
    machine
}

/// Archive the unit at `path` and its live descendants.
pub(super) fn with_archived_unit(mut machine: Machine, path: &str) -> Machine {
    machine
        .archive_unit(
            &UnitPath::parse(path).unwrap(),
            timestamp(),
            &UnitUsage::default(),
        )
        .unwrap();
    machine
}

/// Archive the machine. Its units keep their state.
pub(super) fn archived(mut machine: Machine) -> Machine {
    machine.archive(timestamp(), None).unwrap();
    machine
}

/// The struct row `name@revision` of the machine `owner`, with `members` in their order.
///
/// `owner` holds the number of the machine identifier and the platform of the machine.
/// `supersedes` names the number of the row that this revision replaces.
pub(super) fn struct_row(
    owner: (u128, PlcPlatform),
    id: u128,
    (name, revision): (&str, u32),
    members: &[(&str, PlcTypeId)],
    supersedes: Option<u128>,
) -> PlcType {
    let members = members
        .iter()
        .map(|(member, type_id)| TypeMember {
            name: MemberName::parse(member).unwrap(),
            type_id: *type_id,
        })
        .collect();
    let (machine, platform) = owner;
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(machine)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision,
            supersedes: supersedes.map(tid),
            members,
        },
    })
    .unwrap()
}

/// The row `STRING(length)` of the machine `owner`.
pub(super) fn string_row(owner: (u128, PlcPlatform), id: u128, length: u16) -> PlcType {
    let (machine, platform) = owner;
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(machine)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: LeafClass::String,
            size_bytes: u32::from(length).checked_add(2).unwrap(),
            string_length: Some(length),
        },
    })
    .unwrap()
}

/// The row `ARRAY[1..4] OF <element>` of the machine `owner`.
pub(super) fn array_row(owner: (u128, PlcPlatform), id: u128, element: u128) -> PlcType {
    let dimensions = vec![ArrayDimension::parse(1, 4).unwrap()];
    let (machine, platform) = owner;
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
