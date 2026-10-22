//! The data of the rebind tests (task 12.2). This module is test code.
//!
//! The module solves one problem: the tests of the rebind form and of `POST .../rebind` need the
//! same body builder and some extra rows on `M7`. Each seed function adds one struct group:
//!
//! - `ST_Zone@4` and the parameter `ZONE_ONE` of type `ST_Zone@2` with a limit on `.temp_max`.
//! - `ST_Gate@1` and `ST_Gate@2`. The new member `label` is a `STRING(10)`. The parameter `GATE`
//!   stands first in the list, so its group sorts first.
//! - `ST_Big@1` and `ST_Big@2`, and the parameter `BIG` with 8 193 elements. Its rebind passes
//!   the leaf limit and gives `E-PLT-018`.

use std::collections::BTreeSet;

use recipe_app::memory::seed_type_id;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafPath, MemberName, PlatformTypes, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, S7Address, StringKind, StringLength,
    TypeMember, TypeName, TypeTree, UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, StoredLimit, StoredParameter,
};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use crate::WebSettings;
// The rebind tests in `versions` import this name from here. It is their name of `assert_htmx_error`.
pub(super) use crate::pages::test_helpers::assert_htmx_error as assert_htmx_answer;
use crate::test_support::plant::World;
use crate::test_support::recipes::ZONES_KEY;
use crate::test_support::{CSRF_KEY_NEW, Fixture};

/// The version page of the draft of `R-100`.
pub(super) const DRAFT_PAGE: &str = "/recipes/R-100/versions/3";
/// The rebind route of the draft of `R-100`.
pub(super) const REBIND: &str = "/recipes/R-100/versions/3/rebind";
/// The identifier and the key of the parameter `ZONE_ONE`.
pub(super) const ZONE_ONE_KEY: Uuid = Uuid::from_u128(0x7357_0907_0000_0000_0000_0000_0000_0001);
/// The identifier and the key of the parameter `GATE`.
pub(super) const GATE_KEY: Uuid = Uuid::from_u128(0x7357_0907_0000_0000_0000_0000_0000_0002);
/// The identifier and the key of the parameter `BIG`.
const BIG_KEY: Uuid = Uuid::from_u128(0x7357_0907_0000_0000_0000_0000_0000_0003);
/// The identifier of the revision 4 of the struct `ST_Zone`.
const ZONE_V4_ID: u128 = 0x7357_0907_0000_0000_0000_0000_0000_0004;
/// The identifier of the type `STRING(10)` of `M7`.
const STRING_10_ID: u128 = 0x7357_0907_0000_0000_0000_0000_0000_0005;
/// The identifiers of `ST_Gate@1` and `ST_Gate@2`.
const GATE_IDS: (u128, u128) = (
    0x7357_0907_0000_0000_0000_0000_0000_0006,
    0x7357_0907_0000_0000_0000_0000_0000_0007,
);
/// The identifiers of `ST_Big@1`, `ST_Big@2`, and `ARRAY[1..8193] OF ST_Big@1`.
const BIG_IDS: (u128, u128, u128) = (
    0x7357_0907_0000_0000_0000_0000_0000_0008,
    0x7357_0907_0000_0000_0000_0000_0000_0009,
    0x7357_0907_0000_0000_0000_0000_0000_000a,
);
/// The element count of `BIG`. Two members per element pass the leaf limit of 16 384.
const BIG_COUNT: usize = 8_193;

/// Build the body of a rebind form from the row version, the two type rows, and the fields.
///
/// Each field is a parameter key, a leaf path, and a value text in the form encoding.
pub(super) fn rebind_body(
    row_version: &str,
    (from, to): (PlcTypeId, PlcTypeId),
    fields: &[(Uuid, String, &str)],
) -> String {
    let mut body = format!("row_version={row_version}&from_type_id={from}&to_type_id={to}");
    for (key, path, value) in fields {
        body.push_str(&format!("&new_member_values[{key}]{path}={value}"));
    }
    body
}

/// Return the fields `.temp_min` of the elements `indexes` of `ZONES`, each with `value`.
pub(super) fn zones_fields(
    indexes: &[u32],
    value: &'static str,
) -> Vec<(Uuid, String, &'static str)> {
    indexes
        .iter()
        .map(|index| (ZONES_KEY, format!("[{index}].temp_min"), value))
        .collect()
}

/// Return the `id` of the rebind form of the source revision `from` (design D6).
pub(super) fn form_id(from: PlcTypeId) -> String {
    format!("rebind-{from}")
}

/// Return the body of a rebind of `ZONES` from `ST_Zone@2` to `ST_Zone@3`.
pub(super) fn body_v3(
    world: &World,
    row_version: &str,
    indexes: &[u32],
    value: &'static str,
) -> String {
    let types = (world.ids.zone_v2, zone_v3(world));
    rebind_body(row_version, types, &zones_fields(indexes, value))
}

/// Return the number of the lines of the summary in `#error-panel`.
pub(super) fn summary_lines(body: &str) -> usize {
    let lines = body.split("<ul class=\"field-lines\">").nth(1).unwrap();
    lines.split("</ul>").next().unwrap().matches("<li>").count()
}

/// Return the markup of the out-of-band form `id` of an answer, after its `id` attribute.
pub(super) fn oob_form<'a>(body: &'a str, id: &str) -> &'a str {
    let (_before, rest) = body.split_once(&format!("<form id=\"{id}\"")).unwrap();
    rest.split("</form>").next().unwrap()
}

/// Build the world of `recipe_world` with the bound `form.rebind_leaves` set to `bound`.
pub(super) fn bounded_world(bound: usize) -> World {
    let settings = WebSettings {
        form_rebind_leaves: bound,
        ..WebSettings::default()
    };
    let fixture = Fixture::with_settings(settings, CSRF_KEY_NEW);
    let admin = fixture.seed_session(1, Role::Administrator, false);
    let engineer = fixture.seed_session(2, Role::ProcessEngineer, false);
    let operator = fixture.seed_session(3, Role::Operator, false);
    let ids = fixture.seed_plant();
    fixture.seed_recipes();
    World {
        fixture,
        ids,
        admin,
        engineer,
        operator,
    }
}

/// Return the revision 3 of `ST_Zone`. `seed_recipes` writes the same rows again.
pub(super) fn zone_v3(world: &World) -> PlcTypeId {
    world.fixture.seed_recipes().zone_v3
}

/// Return the seed row of `primitive`.
fn primitive(primitive: Primitive) -> PlcTypeId {
    seed_type_id(primitive, None).unwrap()
}

/// Build a struct shape with the members `members`.
fn struct_shape(
    (name, revision): (&str, u32),
    supersedes: Option<PlcTypeId>,
    members: &[(&str, PlcTypeId)],
) -> RecordShape {
    RecordShape::Struct {
        name: TypeName::parse(name).unwrap(),
        revision,
        supersedes,
        members: members
            .iter()
            .map(|(member, type_id)| TypeMember {
                name: MemberName::parse(member).unwrap(),
                type_id: *type_id,
            })
            .collect(),
    }
}

/// Seed one type row of `M7` on the S7-300 and return its identifier.
fn seed_row(world: &World, id: u128, shape: RecordShape) -> PlcTypeId {
    let row = PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(Uuid::from_u128(id)),
        machine: Some(world.ids.m7),
        platforms: BTreeSet::from([PlcPlatform::S7_300]),
        shape,
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
    row.id()
}

/// Seed `ST_Zone@4` with the members `members` of the given primitives, and return its row.
pub(super) fn seed_zone_v4(world: &World, members: &[(&str, Primitive)]) -> PlcTypeId {
    let typed: Vec<(&str, PlcTypeId)> = members
        .iter()
        .map(|(name, kind)| (*name, primitive(*kind)))
        .collect();
    let shape = struct_shape(("ST_Zone", 4), Some(zone_v3(world)), &typed);
    seed_row(world, ZONE_V4_ID, shape)
}

/// The fields of one seeded parameter that differ from parameter to parameter.
struct Seeded<'a> {
    key: Uuid,
    tag: &'a str,
    unit: &'a str,
    ordinal: u32,
    address: &'a str,
    plc_type: PlcTypeId,
    /// The rows that the tree of the type needs.
    rows: &'a [PlcTypeId],
    value: &'a str,
    limits: Vec<StoredLimit>,
}

/// Seed one parameter of the draft of `R-100`. The value is parsed against its tree.
fn seed_parameter(world: &World, seeded: Seeded<'_>) {
    let state = world.fixture.store.state();
    let rows: Vec<PlcType> = seeded
        .rows
        .iter()
        .map(|id| state.plc_type(*id).cloned().unwrap())
        .collect();
    let types = PlatformTypes::collect(world.ids.m7, PlcPlatform::S7_300, rows).unwrap();
    let tree = TypeTree::resolve(seeded.plc_type, &types).unwrap();
    let machine = state.machine(world.ids.m7).unwrap();
    let unit = machine
        .unit(&UnitPath::parse(seeded.unit).unwrap())
        .unwrap();
    let parameter = StoredParameter {
        id: ParameterId::from(seeded.key),
        key: ParameterKey::from(seeded.key),
        unit: unit.id(),
        tag: ParameterTag::parse(seeded.tag).unwrap(),
        name_en: Text::parse(seeded.tag).unwrap(),
        description_en: LongText::parse("A parameter of a rebind test").unwrap(),
        translations: Vec::new(),
        plc_type: seeded.plc_type,
        s7_address: Some(S7Address::parse(seeded.address).unwrap()),
        limits: seeded.limits,
        value: ParameterValue::parse(seeded.value, &tree)
            .unwrap()
            .to_stored(),
        ordinal: Ordinal::parse(seeded.ordinal).unwrap(),
    };
    world
        .fixture
        .store
        .seed_parameter(world.ids.draft, &parameter);
}

/// Seed the parameter `ZONE_ONE` of type `ST_Zone@2` on `U1` with a limit on `.temp_max`.
pub(super) fn seed_zone_one(world: &World) {
    let rows = [
        primitive(Primitive::Int),
        primitive(Primitive::Real),
        world.ids.zone_v2,
    ];
    seed_parameter(
        world,
        Seeded {
            key: ZONE_ONE_KEY,
            tag: "ZONE_ONE",
            unit: "U1",
            ordinal: 9,
            address: "DB9.DBB0",
            plc_type: world.ids.zone_v2,
            rows: &rows,
            value: r#"{"speed":0,"temp_max":10}"#,
            limits: vec![StoredLimit {
                leaf_path: LeafPath::parse(".temp_max").unwrap(),
                min: Some("0".parse().unwrap()),
                max: Some("100".parse().unwrap()),
                max_length: None,
            }],
        },
    );
}

/// Seed `ST_Gate@1`, `ST_Gate@2` with the new member `label`, and the parameter `GATE`.
///
/// Return the two revisions. `GATE` stands on `U1` with the ordinal 0, before every other row.
pub(super) fn seed_gate(world: &World) -> (PlcTypeId, PlcTypeId) {
    let length = StringLength::parse(StringKind::String, PlcPlatform::S7_300, 10).unwrap();
    let string = seed_row(
        world,
        STRING_10_ID,
        RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: length.size_bytes(StringKind::String).unwrap(),
            string_length: Some(length.value()),
        },
    );
    let int = primitive(Primitive::Int);
    let one = seed_row(
        world,
        GATE_IDS.0,
        struct_shape(("ST_Gate", 1), None, &[("open", int)]),
    );
    let members = [("open", int), ("label", string)];
    let two = seed_row(
        world,
        GATE_IDS.1,
        struct_shape(("ST_Gate", 2), Some(one), &members),
    );
    seed_parameter(
        world,
        Seeded {
            key: GATE_KEY,
            tag: "GATE",
            unit: "U1",
            ordinal: 0,
            address: "DB8.DBB0",
            plc_type: one,
            rows: &[int, one],
            value: r#"{"open":0}"#,
            limits: Vec::new(),
        },
    );
    (one, two)
}

/// Seed `ST_Big@1`, `ST_Big@2`, and the parameter `BIG` of 8 193 elements. Return both revisions.
pub(super) fn seed_big(world: &World) -> (PlcTypeId, PlcTypeId) {
    let int = primitive(Primitive::Int);
    let one = seed_row(
        world,
        BIG_IDS.0,
        struct_shape(("ST_Big", 1), None, &[("a", int)]),
    );
    let members = [("a", int), ("b", int)];
    let two = seed_row(
        world,
        BIG_IDS.1,
        struct_shape(("ST_Big", 2), Some(one), &members),
    );
    let upper = i32::try_from(BIG_COUNT).unwrap();
    let dimensions = ArrayDimensions::parse(vec![ArrayDimension::parse(1, upper).unwrap()]);
    let shape = RecordShape::Array {
        element: one,
        dimensions: dimensions.unwrap(),
    };
    let array = seed_row(world, BIG_IDS.2, shape);
    let value = format!("[{}]", vec![r#"{"a":0}"#; BIG_COUNT].join(","));
    seed_parameter(
        world,
        Seeded {
            key: BIG_KEY,
            tag: "BIG",
            unit: "U1.3",
            ordinal: 20,
            address: "DB7.DBB0",
            plc_type: array,
            rows: &[int, one, array],
            value: &value,
            limits: Vec::new(),
        },
    );
    (one, two)
}
