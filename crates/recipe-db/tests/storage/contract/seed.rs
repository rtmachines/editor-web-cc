//! Fixtures of the contract cases. They go through the ports and `UnitOfWork::commit`.

use std::collections::BTreeSet;

use recipe_app::ports::{Ensured, InsertedMachine};
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, Machine, MachineCode, MachineId, MemberName, PlcPlatform,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, StoredMachine, StoredUnit, StringKind,
    StringLength, TypeMember, TypeName, UnitId, UnitPath,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::harness::{Harness, begin, commit};

/// The offset that separates the identifiers of units from the identifiers of machines.
const UNIT_ID_OFFSET: u128 = 1_000;

/// A machine identifier with a fixed order: a larger `n` sorts after a smaller `n`.
pub fn machine_id(n: u128) -> MachineId {
    MachineId::from(Uuid::from_u128(n))
}

/// A unit identifier that no machine identifier of the cases uses.
pub fn unit_id(n: u128) -> UnitId {
    UnitId::from(Uuid::from_u128(UNIT_ID_OFFSET.checked_add(n).unwrap()))
}

/// Parse a text for a fixture.
pub fn name(text: &str) -> Text {
    Text::parse(text).unwrap()
}

/// Parse a unit path for a fixture.
pub fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

/// Parse a machine code for a fixture.
pub fn code(text: &str) -> MachineCode {
    MachineCode::parse(text).unwrap()
}

/// A live `TwinCat3` machine with the unit `U1`. Its identifiers come from `n`.
pub fn machine(n: u128, code_text: &str) -> Machine {
    Machine::create(
        machine_id(n),
        code(code_text),
        name("Press"),
        PlcPlatform::TwinCat3,
        (unit_id(n.checked_mul(10).unwrap()), name("Main unit")),
    )
}

/// A fixed whole-second instant in UTC. The database keeps it without a loss.
pub fn instant() -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap())
}

/// A machine with the live unit `U1` and the archived unit `U2`.
pub fn machine_with_archived_unit(n: u128, code_text: &str) -> Machine {
    let stored = StoredMachine {
        id: machine_id(n),
        code: code(code_text),
        name: name("Press"),
        platform: PlcPlatform::TwinCat3,
        archived_at: None,
        units: vec![
            StoredUnit {
                id: unit_id(n.checked_mul(10).unwrap()),
                path: path("U1"),
                name: name("Main unit"),
                archived_at: None,
            },
            StoredUnit {
                id: unit_id(n.checked_mul(10).unwrap().checked_add(1).unwrap()),
                path: path("U2"),
                name: name("Old unit"),
                archived_at: Some(instant()),
            },
        ],
    };
    Machine::try_from(stored).unwrap()
}

/// Insert a machine in its own committed transaction and return the images.
pub async fn insert(harness: &dyn Harness, machine: &Machine) -> InsertedMachine {
    let mut tx = begin(harness).await;
    let inserted = tx.plant().insert_machine(machine).await.unwrap();
    commit(harness, tx, machine.id()).await;
    inserted
}

/// Archive a machine in its own committed transaction.
pub async fn archive(harness: &dyn Harness, id: MachineId) {
    let mut tx = begin(harness).await;
    tx.plant().archive_machine(id).await.unwrap();
    commit(harness, tx, id).await;
}

/// Read a machine by identifier in a transaction that nothing commits.
pub async fn read(harness: &dyn Harness, id: MachineId) -> Option<Machine> {
    let mut tx = begin(harness).await;
    tx.plant().machine_by_id(id).await.unwrap()
}

/// The offset of the unit `U1.1` in `machine_with_tree`.
pub const U1_1: u128 = 5;
/// The offset of the unit `U1.1.1` in `machine_with_tree`.
pub const U1_1_1: u128 = 3;
/// The offset of the unit `U1.1.2` in `machine_with_tree`.
pub const U1_1_2: u128 = 2;
/// The offset of the unit `U1.2` in `machine_with_tree`.
pub const U1_2: u128 = 1;
/// The offset of the unit `U1.10` in `machine_with_tree`.
pub const U1_10: u128 = 9;
/// The offset of the unit `U2` in `machine_with_tree`.
pub const U2: u128 = 8;

/// The identifier of a unit of `machine_with_tree`. `offset` is one of the constants above.
pub fn tree_unit_id(n: u128, offset: u128) -> UnitId {
    unit_id(
        n.checked_mul(100)
            .and_then(|base| base.checked_add(offset))
            .unwrap(),
    )
}

/// A machine with the units `U1`, `U1.1`, `U1.1.1`, `U1.1.2`, `U1.2`, `U1.10`, and `U2`.
///
/// The identifiers do not follow the path order. The unit `U1.1` sorts after `U1.1.1` and
/// `U1.1.2` by `id`.
pub fn machine_with_tree(n: u128, code_text: &str) -> Machine {
    let mut built = machine(n, code_text);
    let main = path("U1");
    let first = path("U1.1");
    for (offset, parent, segment) in [
        (U1_1, Some(&main), "1"),
        (U1_1_1, Some(&first), "1"),
        (U1_1_2, Some(&first), "2"),
        (U1_2, Some(&main), "2"),
        (U1_10, Some(&main), "10"),
        (U2, None, "2"),
    ] {
        built
            .add_unit(tree_unit_id(n, offset), parent, segment, name("Unit"))
            .unwrap();
    }
    built
}

/// A type identifier that no seed row and no machine identifier uses.
pub fn type_id(n: u128) -> PlcTypeId {
    PlcTypeId::from(Uuid::from_u128(
        0x7e57_0000_0000_0000_0000_0000_0000_0000 | n,
    ))
}

/// The one platform of a machine as the set of a new type row.
fn platform_set(machine: &Machine) -> BTreeSet<PlcPlatform> {
    BTreeSet::from([machine.platform()])
}

/// The record of a `STRING(length)` or `WSTRING(length)` row of a machine.
pub fn string_record(
    machine: &Machine,
    id: PlcTypeId,
    kind: StringKind,
    length: u16,
) -> PlcTypeRecord {
    let primitive = match kind {
        StringKind::String => Primitive::String,
        StringKind::Wstring => Primitive::Wstring,
    };
    let size_bytes = StringLength::parse(kind, machine.platform(), u32::from(length))
        .unwrap()
        .size_bytes(kind)
        .unwrap();
    PlcTypeRecord {
        id,
        machine: Some(machine.id()),
        platforms: platform_set(machine),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes,
            string_length: Some(length),
        },
    }
}

/// The record of an array row of a machine. `bounds` lists `(lower, upper)` for each dimension.
pub fn array_record(
    machine: &Machine,
    id: PlcTypeId,
    element: PlcTypeId,
    bounds: &[(i32, i32)],
) -> PlcTypeRecord {
    let dimensions = bounds
        .iter()
        .map(|(lower, upper)| ArrayDimension::parse(*lower, *upper).unwrap())
        .collect();
    PlcTypeRecord {
        id,
        machine: Some(machine.id()),
        platforms: platform_set(machine),
        shape: RecordShape::Array {
            element,
            dimensions: ArrayDimensions::parse(dimensions).unwrap(),
        },
    }
}

/// The record of a struct row of a machine. `members` lists `(name, type)` in order.
pub fn struct_record(
    machine: &Machine,
    id: PlcTypeId,
    struct_name: &str,
    revision: u32,
    supersedes: Option<PlcTypeId>,
    members: &[(&str, PlcTypeId)],
) -> PlcTypeRecord {
    let members = members
        .iter()
        .map(|(member, type_id)| TypeMember {
            name: MemberName::parse(member).unwrap(),
            type_id: *type_id,
        })
        .collect();
    PlcTypeRecord {
        id,
        machine: Some(machine.id()),
        platforms: platform_set(machine),
        shape: RecordShape::Struct {
            name: TypeName::parse(struct_name).unwrap(),
            revision,
            supersedes,
            members,
        },
    }
}

/// Find the global seed row of a primitive without length through `type_rows`.
pub async fn seed_type(harness: &dyn Harness, machine: MachineId, wanted: Primitive) -> PlcTypeId {
    let mut tx = begin(harness).await;
    let rows = tx.types().type_rows(machine).await.unwrap();
    rows.iter()
        .find(|row| {
            row.machine().is_none()
                && matches!(row.shape(),
                    RecordShape::Primitive { primitive, string_length: None, .. }
                        if *primitive == wanted)
        })
        .map(recipe_domain::plant::PlcType::id)
        .expect("the seed has the primitive")
}

/// Run `ensure_derived` in its own committed transaction.
pub async fn ensure(harness: &dyn Harness, machine: MachineId, record: PlcTypeRecord) -> Ensured {
    let mut tx = begin(harness).await;
    let ensured = tx.types().ensure_derived(record).await.unwrap();
    commit(harness, tx, machine).await;
    ensured
}
