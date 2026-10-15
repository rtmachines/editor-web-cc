//! The extra data of the tests of the unit pages (design D12, D16). This module is test code.
//!
//! The module solves one problem: the tests of the unit form, the unit page, and the commands of a
//! unit need two machines that `seed_plant` does not hold. The shared world lives once in
//! `test_support::plant` (design D20 U13). This file adds only what the unit pages alone need. The
//! store holds the data of `seed_plant` and three more rows:
//!
//! - The live machine `M30` with the main unit `U1` and the 64 live units `U1.1` to `U1.64`.
//! - The live machine `M31` with the live units `U1`, `U1.1`, and `U2`, and the archived unit `U2`.
//!   `U1` is the last live main unit of `M31`.

use std::collections::BTreeSet;

use recipe_app::memory::seed_type_id;
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlatformTypes, PlcPlatform, PlcType, PlcTypeRecord, Primitive,
    RecordShape, TypeTree, UnitId, UnitPath, UnitUsage,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, RecipeVersionId,
    StoredParameter,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::test_support::plant::{ENGLISH, HTMX, M7_ID, World, world as plant_world};
use crate::test_support::{Fixture, Reply, TestSession};

/// The header of an HTMX request that targets the region of the unit form.
const TARGET_FORM: (&str, &str) = ("hx-target", "unit-form");
/// The first field of the identifier of a machine that this module seeds.
const MACHINE_TAG: u32 = 0x7357_0020;
/// The first field of the identifier of a unit that this module seeds.
const UNIT_TAG: u32 = 0x7357_0021;
/// The first field of the identifier of a unit of `seed_plant`. `Fixture::seed_plant` owns it.
const PLANT_UNIT_TAG: u32 = 0x7357_0002;
/// The instant of the archive of the unit `U2` of `M31`, in Unix seconds.
const ARCHIVED_AT_SECONDS: i64 = 600_000;
/// The identifier of the approved version 2 of the recipe `R-100`. `Fixture::seed_plant` owns it.
const APPROVED_VERSION: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0002);
/// The identifier of the parameter that `seed_approved_use` makes.
const USED_PARAMETER: Uuid = Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0000_00a1);
/// The key of the parameter that `seed_approved_use` makes.
const USED_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_00a1);

/// Build the world: the shared world and the machines `M30` and `M31`.
pub(super) fn world() -> World {
    let world = plant_world();
    world.fixture.store.seed_machine(&full_machine());
    world.fixture.store.seed_machine(&spare_machine());
    world
}

impl World {
    /// Send `GET path` as an HTMX request that targets the region `#unit-form`, in English.
    pub(super) async fn get_form(&self, session: &TestSession, path: &str) -> Reply {
        self.get(session, path, &[ENGLISH, HTMX, TARGET_FORM]).await
    }
}

/// Parse a unit path.
fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

/// Parse a short text.
fn name(text: &str) -> Text {
    Text::parse(text).unwrap()
}

/// Make the identifier of a unit of a machine that this module seeds.
fn unit_id(machine: u16, serial: u32) -> UnitId {
    let bytes = serial.to_be_bytes();
    let tail = [0, 0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3]];
    UnitId::from(Uuid::from_fields(UNIT_TAG, machine, 0, &tail))
}

/// Build the live machine `M<number>` with the main unit `U1`.
fn machine(number: u16, text: &str) -> Machine {
    let bytes = u32::from(number).to_be_bytes();
    let tail = [0, 0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3]];
    Machine::create(
        MachineId::from(Uuid::from_fields(MACHINE_TAG, 0, 0, &tail)),
        MachineCode::parse(&format!("M{number}")).unwrap(),
        name(text),
        PlcPlatform::TwinCat3,
        (unit_id(number, 0), name("Infeed")),
    )
}

/// Build `M30`: the main unit `U1` with the 64 live units `U1.1` to `U1.64` under it.
fn full_machine() -> Machine {
    let mut machine = machine(30, "Full line");
    let main = path("U1");
    for segment in 1..=64_u8 {
        machine
            .add_unit(
                unit_id(30, u32::from(segment)),
                Some(&main),
                &segment.to_string(),
                name("Cell"),
            )
            .unwrap();
    }
    machine
}

/// Build `M31`: the live units `U1`, `U1.1`, `U2` and then the archived unit `U2`.
fn spare_machine() -> Machine {
    let mut machine = machine(31, "Spare line");
    machine
        .add_unit(unit_id(31, 11), Some(&path("U1")), "1", name("Feeder"))
        .unwrap();
    machine
        .add_unit(unit_id(31, 2), None, "2", name("Outfeed"))
        .unwrap();
    let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(ARCHIVED_AT_SECONDS).unwrap());
    machine
        .archive_unit(&path("U2"), at, &UnitUsage::default())
        .unwrap();
    machine
}

/// Build the type tree of the primitive `INT`.
fn int_tree() -> TypeTree {
    let id = seed_type_id(Primitive::Int, None).unwrap();
    let row = PlcType::try_from(PlcTypeRecord {
        id,
        machine: None,
        platforms: BTreeSet::from(PlcPlatform::ALL),
        shape: RecordShape::Primitive {
            primitive: Primitive::Int,
            leaf_class: Primitive::Int.leaf_class(),
            size_bytes: Primitive::Int.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap();
    let types =
        PlatformTypes::collect(MachineId::from(M7_ID), PlcPlatform::S7_300, vec![row]).unwrap();
    TypeTree::resolve(id, &types).unwrap()
}

/// Seed a parameter of the approved version 2 of `R-100` on the unit `unit_text` of `M7`.
///
/// The parameter makes `ArchiveUnit` give `E-PLT-014` for that unit and its ancestors.
pub(super) fn seed_approved_use(fixture: &Fixture, unit_text: &str) {
    let mut tail = [0_u8; 8];
    for (slot, segment) in tail.iter_mut().zip(path(unit_text).segments()) {
        *slot = segment;
    }
    let unit = UnitId::from(Uuid::from_fields(PLANT_UNIT_TAG, 7, 0, &tail));
    let value = ParameterValue::parse("5", &int_tree()).unwrap();
    let parameter = StoredParameter {
        id: ParameterId::from(USED_PARAMETER),
        key: ParameterKey::from(USED_KEY),
        unit,
        tag: ParameterTag::parse("USED").unwrap(),
        name_en: name("Used"),
        description_en: LongText::parse("A parameter of the approved version").unwrap(),
        translations: Vec::new(),
        plc_type: seed_type_id(Primitive::Int, None).unwrap(),
        s7_address: None,
        limits: Vec::new(),
        value: value.to_stored(),
        ordinal: Ordinal::parse(0).unwrap(),
    };
    fixture
        .store
        .seed_parameter(RecipeVersionId::from(APPROVED_VERSION), &parameter);
}
