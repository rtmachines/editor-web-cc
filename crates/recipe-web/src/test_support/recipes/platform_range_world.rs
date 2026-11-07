//! Approved versions with a stored value or address that the current rules refuse
//! (ADR-0139 (f), ADR-0140 (e)).
//!
//! The export route tests need one world for each case. Each world holds one machine, one unit, one
//! recipe, and the approved version 1 with one parameter. The version has the real content hash,
//! because an export route checks the stored hash first. This module is test code.

use std::collections::BTreeSet;

use recipe_app::memory::seed_type_id;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    RecordShape, S7Address, UnitId,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, Recipe, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredParameter, StoredRecipe, StoredValue, StoredVersionHeader, VersionState,
};
use recipe_domain::text::{LongText, Text};
use serde_json::json;
use uuid::Uuid;

use super::date_world::{VersionTarget, approve_version_with_real_hash};
use crate::test_support::plant::{World, world};

/// The reference number of the recipe with the `LDT` after the current range.
pub(crate) const LDT_REFERENCE: &str = "R-420";
/// The reference number of the recipe with the data block 60000.
pub(crate) const DATA_BLOCK_REFERENCE: &str = "R-430";

/// The count of 1 January 2300 in nanoseconds. It is after the end of the current `LDT` range.
const LDT_AFTER_CURRENT_RANGE: u64 = 10_413_792_000_000_000_000;
/// The identifier of the version 1 of the recipe with the `LDT` after the current range.
pub(crate) const LDT_VERSION_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f30_0001);
/// The identifier of the type `LDT` of `TwinCat3`. The seed has no row of it.
const LDT_TYPE_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0f30_0001);

/// The fixed data of one world. The tests of the two worlds share the seed code.
struct WorldSpec {
    machine: Uuid,
    unit: Uuid,
    recipe: Uuid,
    version: Uuid,
    parameter: Uuid,
    machine_code: &'static str,
    platform: PlcPlatform,
    reference: &'static str,
    tag: &'static str,
    plc_type: PlcTypeId,
    s7_address: Option<&'static str>,
    value: serde_json::Value,
}

/// Build the plant world with the approved version 1 of `R-420` on the `TwinCat3` machine `M60`.
///
/// The parameter `STAMP` of the type `LDT` holds a count above 2^63-1. Only the earlier rule set
/// accepts it. `TwinCat3` has no S7 address (I5).
pub(crate) async fn ldt_after_current_range_world() -> World {
    let world = world();
    world.fixture.store.seed_type(&ldt_type_row());
    let spec = WorldSpec {
        machine: Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0f30_0001),
        unit: Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0f30_0001),
        recipe: Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0f30_0001),
        version: LDT_VERSION_ID,
        parameter: Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0f30_0001),
        machine_code: "M60",
        platform: PlcPlatform::TwinCat3,
        reference: LDT_REFERENCE,
        tag: "STAMP",
        plc_type: PlcTypeId::from(LDT_TYPE_ID),
        s7_address: None,
        value: json!(LDT_AFTER_CURRENT_RANGE),
    };
    seed_approved_world(&world, spec).await;
    world
}

/// Build the plant world with the approved version 1 of `R-430` on the `S7_1500` machine `M61`.
///
/// The parameter `SPEED` of the type `INT` sits at `DB60000.DBW0`. Only the earlier rule set
/// accepts the data block 60000.
pub(crate) async fn data_block_above_59999_world() -> World {
    let world = world();
    let spec = WorldSpec {
        machine: Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0f31_0001),
        unit: Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0f31_0001),
        recipe: Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0f31_0001),
        version: Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f31_0001),
        parameter: Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0f31_0001),
        machine_code: "M61",
        platform: PlcPlatform::S7_1500,
        reference: DATA_BLOCK_REFERENCE,
        tag: "SPEED",
        plc_type: seed_type_id(Primitive::Int, None).unwrap(),
        s7_address: Some("DB60000.DBW0"),
        value: json!(120),
    };
    seed_approved_world(&world, spec).await;
    world
}

/// Return the global row of the type `LDT` for `TwinCat3`.
fn ldt_type_row() -> PlcType {
    let primitive = Primitive::Ldt;
    PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(LDT_TYPE_ID),
        machine: None,
        platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap()
}

/// Seed the machine, the recipe, and the parameter of `spec`. Then approve the version.
async fn seed_approved_world(world: &World, spec: WorldSpec) {
    let machine_id = MachineId::from(spec.machine);
    let unit_id = UnitId::from(spec.unit);
    let machine = Machine::create(
        machine_id,
        MachineCode::parse(spec.machine_code).unwrap(),
        Text::parse("Range line").unwrap(),
        spec.platform,
        (unit_id, Text::parse("Infeed").unwrap()),
    );
    world.fixture.store.seed_machine(&machine);
    let recipe_id = RecipeId::from(spec.recipe);
    let version_id = RecipeVersionId::from(spec.version);
    let reference = ReferenceNumber::parse(spec.reference).unwrap();
    let recipe = Recipe::try_from(StoredRecipe {
        id: recipe_id,
        machine: machine_id,
        reference: reference.clone(),
        name: Text::parse("Range recipe").unwrap(),
        deleted_at: None,
        versions: vec![StoredVersionHeader {
            id: version_id,
            number: 1,
            state: VersionState::LiveDraft,
            row_version: 1,
        }],
    })
    .unwrap();
    let creator = UserId::from(Uuid::from_u128(1));
    world
        .fixture
        .store
        .seed_recipe(&recipe, spec.platform, creator);
    let parameter = StoredParameter {
        id: ParameterId::from(spec.parameter),
        key: ParameterKey::from(spec.parameter),
        unit: unit_id,
        tag: ParameterTag::parse(spec.tag).unwrap(),
        name_en: Text::parse("range value").unwrap(),
        description_en: LongText::parse("A seeded value").unwrap(),
        translations: Vec::new(),
        plc_type: spec.plc_type,
        s7_address: spec.s7_address.map(|text| S7Address::parse(text).unwrap()),
        limits: Vec::new(),
        value: StoredValue::from(spec.value),
        ordinal: Ordinal::parse(0).unwrap(),
    };
    world.fixture.store.seed_parameter(version_id, &parameter);
    let target = VersionTarget {
        version: version_id,
        recipe: recipe_id,
        machine: machine_id,
        reference,
    };
    approve_version_with_real_hash(&world.fixture, target).await;
}
