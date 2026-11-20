//! A live draft on a machine with the STRING encoding `Utf8` (ADR-0202). This module is test code.
//!
//! The parameter form tests need a `Utf8` machine with a `STRING(4)` parameter. The module builds
//! the plant world, the `CodesysIec` machine `M10`, the recipe `R-410`, and its draft version 1.

use std::collections::BTreeSet;

use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    RecordShape, StoredMachine, StoredUnit, StringEncoding, StringKind, StringLength, UnitId,
    UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, Recipe, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredParameter, StoredValue,
};
use recipe_domain::text::{LongText, Text};
use serde_json::json;
use uuid::Uuid;

use crate::test_support::plant::{World, world};

/// The reference number of the recipe of the world.
pub(crate) const REFERENCE: &str = "R-410";
/// The key of the parameter `LABEL`, a `STRING(4)`.
pub(crate) const LABEL_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0f40_0001);

/// The length of the `STRING` of `LABEL`.
const LABEL_LENGTH: u32 = 4;
/// The identifier of the machine `M10`.
const MACHINE_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0f40_0001);
/// The identifier of the unit `U1` of `M10`.
const UNIT_ID: Uuid = Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0f40_0001);
/// The identifier of the recipe `R-410`.
const RECIPE_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0f40_0001);
/// The identifier of the draft version 1 of `R-410`.
const VERSION_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f40_0001);
/// The identifier of the type `STRING(4)` of `M10`.
const STRING_TYPE_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0f40_0001);
/// The identifier of the parameter `LABEL`.
const LABEL_ID: Uuid = Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0f40_0001);

/// Parse a short text.
fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// The `Utf8` machine `M10` on `CodesysIec` with the unit `U1`.
fn machine() -> Machine {
    Machine::try_from(StoredMachine {
        id: MachineId::from(MACHINE_ID),
        code: MachineCode::parse("M10").unwrap(),
        name: text("Labeller"),
        platform: PlcPlatform::CodesysIec,
        db_number: None,
        string_encoding: StringEncoding::Utf8,
        archived_at: None,
        units: vec![StoredUnit {
            id: UnitId::from(UNIT_ID),
            path: UnitPath::parse("U1").unwrap(),
            name: text("Main"),
            archived_at: None,
        }],
    })
    .unwrap()
}

/// The row `STRING(4)` of `M10`.
fn string_row() -> PlcType {
    let length =
        StringLength::parse(StringKind::String, PlcPlatform::CodesysIec, LABEL_LENGTH).unwrap();
    PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(STRING_TYPE_ID),
        machine: Some(MachineId::from(MACHINE_ID)),
        platforms: BTreeSet::from([PlcPlatform::CodesysIec]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: length.size_bytes(StringKind::String).unwrap(),
            string_length: Some(length.value()),
        },
    })
    .unwrap()
}

/// Build the plant world with the draft version 1 of `R-410` on the `Utf8` machine `M10`.
///
/// The parameter `LABEL` is a `STRING(4)` with the value `"abc"` on the unit `U1`.
pub(crate) fn utf8_world() -> World {
    let world = world();
    let store = &world.fixture.store;
    store.seed_machine(&machine());
    store.seed_type(&string_row());
    let version = RecipeVersionId::from(VERSION_ID);
    let recipe = Recipe::create(
        RecipeId::from(RECIPE_ID),
        MachineId::from(MACHINE_ID),
        ReferenceNumber::parse(REFERENCE).unwrap(),
        text("Label recipe"),
        version,
    );
    store.seed_recipe(
        &recipe,
        PlcPlatform::CodesysIec,
        UserId::from(Uuid::from_u128(1)),
    );
    store.seed_parameter(
        version,
        &StoredParameter {
            id: ParameterId::from(LABEL_ID),
            key: ParameterKey::from(LABEL_KEY),
            unit: UnitId::from(UNIT_ID),
            tag: ParameterTag::parse("LABEL").unwrap(),
            name_en: text("Label"),
            description_en: LongText::parse("A label of four bytes").unwrap(),
            translations: Vec::new(),
            plc_type: PlcTypeId::from(STRING_TYPE_ID),
            s7_address: None,
            limits: Vec::new(),
            value: StoredValue::from(json!("abc")),
            ordinal: Ordinal::parse(0).unwrap(),
        },
    );
    world
}
