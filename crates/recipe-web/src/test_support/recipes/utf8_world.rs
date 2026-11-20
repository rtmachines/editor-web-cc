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

use super::date_world::{VersionTarget, approve_version_with_real_hash};
use crate::test_support::plant::{World, world};

/// The reference number of the recipe of the world.
pub(crate) const REFERENCE: &str = "R-410";
/// The reference number of the recipe of the approved world.
pub(crate) const APPROVED_REFERENCE: &str = "R-300";
/// The key of the parameter `LABEL`, a `STRING(4)`.
pub(crate) const LABEL_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0f40_0001);

/// The stored value of `NOTE` that breaks a current rule of `CodesysIec` (ADR-0201 (a)).
///
/// The scalar `U+1F600` lies outside the Basic Multilingual Plane. The earlier rule set accepts it.
const OUTSIDE_PLANE_NOTE: &str = "A\u{1F600}";
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
/// The identifier of the type `WSTRING(10)` of `M10`.
const WSTRING_TYPE_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0f40_0002);
/// The identifier of the parameter `NOTE`.
const NOTE_ID: Uuid = Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0f40_0002);
/// The length of the `WSTRING` of `NOTE`.
const NOTE_LENGTH: u32 = 10;
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
    seed_label_recipe(&world, REFERENCE, "abc");
    world
}

/// Build the plant world with the approved version 1 of `R-300` on the `Utf8` machine `M10`.
///
/// The parameter `LABEL` is a `STRING(4)` with the value `"abc"` on the unit `U1`. The version has
/// the real content hash, because an export route checks the stored hash first.
pub(crate) async fn utf8_approved_world() -> World {
    let world = world();
    seed_label_recipe(&world, APPROVED_REFERENCE, "abc");
    approve_label_recipe(&world).await;
    world
}

/// Build the plant world as `utf8_approved_world` does, with one more parameter in version 1.
///
/// The parameter `NOTE` is a `WSTRING(10)` with the value `"A\u{1F600}"`. The current rules of
/// `CodesysIec` refuse that scalar, and the earlier rules accept it. The approved version then
/// breaks a current rule with `E-RCP-005`, as a part stored under an earlier rule set does.
pub(crate) async fn utf8_approved_faulty_world() -> World {
    let world = world();
    seed_label_recipe(&world, APPROVED_REFERENCE, "abc");
    seed_note(&world);
    approve_label_recipe(&world).await;
    world
}

/// Approve the version 1 of `R-300` with the real content hash.
async fn approve_label_recipe(world: &World) {
    let target = VersionTarget {
        version: RecipeVersionId::from(VERSION_ID),
        recipe: RecipeId::from(RECIPE_ID),
        machine: MachineId::from(MACHINE_ID),
        reference: ReferenceNumber::parse(APPROVED_REFERENCE).unwrap(),
    };
    approve_version_with_real_hash(&world.fixture, target).await;
}

/// Seed the row `WSTRING(10)` of `M10` and the parameter `NOTE` of version 1 of `R-300`.
fn seed_note(world: &World) {
    let store = &world.fixture.store;
    let length =
        StringLength::parse(StringKind::Wstring, PlcPlatform::CodesysIec, NOTE_LENGTH).unwrap();
    let row = PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(WSTRING_TYPE_ID),
        machine: Some(MachineId::from(MACHINE_ID)),
        platforms: BTreeSet::from([PlcPlatform::CodesysIec]),
        shape: RecordShape::Primitive {
            primitive: Primitive::Wstring,
            leaf_class: Primitive::Wstring.leaf_class(),
            size_bytes: length.size_bytes(StringKind::Wstring).unwrap(),
            string_length: Some(length.value()),
        },
    })
    .unwrap();
    store.seed_type(&row);
    store.seed_parameter(
        RecipeVersionId::from(VERSION_ID),
        &StoredParameter {
            id: ParameterId::from(NOTE_ID),
            key: ParameterKey::from(NOTE_ID),
            unit: UnitId::from(UNIT_ID),
            tag: ParameterTag::parse("NOTE").unwrap(),
            name_en: text("Note"),
            description_en: LongText::parse("A wide note of ten units").unwrap(),
            translations: Vec::new(),
            plc_type: PlcTypeId::from(WSTRING_TYPE_ID),
            s7_address: None,
            limits: Vec::new(),
            value: StoredValue::from(json!(OUTSIDE_PLANE_NOTE)),
            ordinal: Ordinal::parse(1).unwrap(),
        },
    );
}

/// Seed `M10`, its `STRING(4)` row, and the recipe `reference` with its live draft version 1.
///
/// The parameter `LABEL` holds the stored text `value`.
fn seed_label_recipe(world: &World, reference: &str, value: &str) {
    let store = &world.fixture.store;
    store.seed_machine(&machine());
    store.seed_type(&string_row());
    let version = RecipeVersionId::from(VERSION_ID);
    let recipe = Recipe::create(
        RecipeId::from(RECIPE_ID),
        MachineId::from(MACHINE_ID),
        ReferenceNumber::parse(reference).unwrap(),
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
            value: StoredValue::from(json!(value)),
            ordinal: Ordinal::parse(0).unwrap(),
        },
    );
}
