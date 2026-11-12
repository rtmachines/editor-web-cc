//! Tests of the rule faults on the parameter page (tasks 8.9 and 8.10, design D7, ADR-0118).
//!
//! A stored part can pass the earlier rule set and break the current rule set. The page of the
//! draft shows the stored text and a field line with the code under the input of that part. The
//! page of an approved version and the page of a draft that `CreateVersion` copied from it show
//! the same lines. The tests seed such a part in the memory adapter and send
//! `GET .../parameters/{key}`.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use recipe_app::memory::seed_type_id;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafPath, Machine, MachineCode, MachineId, PlcPlatform,
    PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, S7Address, UnitId,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, Recipe, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredLimit, StoredParameter, StoredRecipe, StoredValue, StoredVersionHeader,
    VersionState,
};
use recipe_domain::text::{LongText, Text};
use serde_json::json;
use uuid::Uuid;

use super::super::test_kit::post_htmx;
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::Reply;
use crate::test_support::plant::{World, main_of};

/// The first field of the identifier and of the key of a parameter that these tests seed.
const FAULT_BASE: u128 = 0x7357_0006_0000_0000_0000_0000_0f20_0000;
/// The identifier of the machine `M15` on `S7_1500`. Only that platform has the earlier `DATE` range.
const MACHINE_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0f20_0001);
/// The identifier of the unit `U1` of `M15`.
const UNIT_ID: Uuid = Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0f20_0001);
/// The identifier of the recipe `R-150`.
const RECIPE_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0f20_0001);
/// The identifier of the version 1 of `R-150`. The state of the version depends on the test.
const DRAFT_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f20_0001);
/// The identifier of the global type `DATE` on `S7_1500` that these tests seed.
const DATE_TYPE_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0f20);
/// The key of the `REAL` parameter `RATE`. Its stored value has 8 digits, so it breaks `E-RCP-055`.
const RATE: u128 = 1;
/// The key of the `DATE` parameter `DUE`. Its stored limit `max` breaks `E-RCP-006`.
const DUE: u128 = 2;
/// The key of the `DINT` parameter `OFFSET`. Its stored address breaks `E-RCP-056`.
const OFFSET: u128 = 3;

/// The key of the `REAL` parameter `PRECISE`. Its stored limit `max` has 8 digits and breaks `E-RCP-055`.
const PRECISE: u128 = 4;

/// Parse a short text.
fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Return the key of the seeded parameter `n`.
fn key_of(n: u128) -> Uuid {
    Uuid::from_u128(FAULT_BASE | n)
}

/// Seed the global type `DATE` on `S7_1500`. The in-memory plant has no `DATE` row.
fn seed_date_type(world: &World) -> PlcTypeId {
    let shape = RecordShape::Primitive {
        primitive: Primitive::Date,
        leaf_class: Primitive::Date.leaf_class(),
        size_bytes: Primitive::Date.fixed_size_bytes().unwrap(),
        string_length: None,
    };
    let row = PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(DATE_TYPE_ID),
        machine: None,
        platforms: BTreeSet::from([PlcPlatform::S7_1500]),
        shape,
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
    PlcTypeId::from(DATE_TYPE_ID)
}

/// A stored limit row on the whole value with the upper bound `max`.
fn whole_limit_max(max: i64) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(max.to_string().parse().unwrap()),
        max_length: None,
    }
}

/// A stored limit row on the whole value with the upper bound `max` as decimal text.
fn whole_limit_max_text(max: &str) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(max.parse().unwrap()),
        max_length: None,
    }
}

/// Seed the machine `M15` on `S7_1500` with the recipe `R-150` and its version 1 in `state`.
fn seed_version_on_s7_1500(world: &World, state: VersionState) {
    let machine = Machine::create(
        MachineId::from(MACHINE_ID),
        MachineCode::parse("M15").unwrap(),
        text("Test line"),
        PlcPlatform::S7_1500,
        Some(recipe_domain::plant::DataBlockNumber::parse("1").unwrap()),
        (UnitId::from(UNIT_ID), text("Infeed")),
    )
    .unwrap();
    world.fixture.store.seed_machine(&machine);
    let recipe = Recipe::try_from(StoredRecipe {
        id: RecipeId::from(RECIPE_ID),
        machine: MachineId::from(MACHINE_ID),
        reference: ReferenceNumber::parse("R-150").unwrap(),
        name: text("Fault recipe"),
        deleted_at: None,
        versions: vec![StoredVersionHeader {
            id: RecipeVersionId::from(DRAFT_ID),
            number: 1,
            state,
            row_version: 1,
        }],
    })
    .unwrap();
    let creator = UserId::from(Uuid::from_u128(1));
    world
        .fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::S7_1500, creator);
}

/// Seed the parameter `tag` on the unit `U1` of the draft 1 of `R-150`.
///
/// `n` makes the key and the ordinal unique. `stored` holds the type, the value, the limit rows,
/// and the address, in the stored form that an earlier release could write.
fn seed_stored(
    world: &World,
    (n, tag): (u128, &str),
    stored: (PlcTypeId, StoredValue, Vec<StoredLimit>, Option<&str>),
) {
    let (plc_type, value, limits, address) = stored;
    let id = key_of(n);
    let parameter = StoredParameter {
        id: ParameterId::from(id),
        key: ParameterKey::from(id),
        unit: UnitId::from(UNIT_ID),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse(tag).unwrap(),
        description_en: LongText::parse("A seeded parameter").unwrap(),
        translations: Vec::new(),
        plc_type,
        s7_address: address.map(|text| S7Address::parse(text).unwrap()),
        limits,
        value,
        ordinal: Ordinal::parse(u32::try_from(n).unwrap()).unwrap(),
    };
    let version = RecipeVersionId::from(DRAFT_ID);
    world.fixture.store.seed_parameter(version, &parameter);
}

/// Build the world with the three parameters `RATE`, `DUE`, and `OFFSET` in version 1 of `R-150`.
///
/// Each parameter breaks one rule of the current set and passes the earlier set. `state` is the
/// state of version 1.
fn fault_world(state: VersionState) -> World {
    let world = recipe_world();
    seed_version_on_s7_1500(&world, state);
    let date = seed_date_type(&world);
    let real = seed_type_id(Primitive::Real, None).unwrap();
    let dint = seed_type_id(Primitive::Dint, None).unwrap();
    let value = |json| StoredValue::from(json);
    seed_stored(
        &world,
        (RATE, "RATE"),
        (real, value(json!(16_777_217)), Vec::new(), None),
    );
    seed_stored(
        &world,
        (DUE, "DUE"),
        (
            date,
            value(json!(72_683)),
            vec![whole_limit_max(72_700)],
            None,
        ),
    );
    seed_stored(
        &world,
        (OFFSET, "OFFSET"),
        (dint, value(json!(0)), Vec::new(), Some("DB1.DBD65534")),
    );
    seed_stored(
        &world,
        (PRECISE, "PRECISE"),
        (
            real,
            value(json!(1)),
            vec![whole_limit_max_text("100.00001")],
            None,
        ),
    );
    world
}

/// Send `GET` to the parameter page of the seeded parameter `n` in version `number` of `R-150`.
///
/// The function asserts the status 200 and returns the `main` element of the reply.
async fn version_page_of(world: &World, number: u32, n: u128) -> String {
    let reply = world
        .get_en(
            &world.engineer,
            &format!("/recipes/R-150/versions/{number}/parameters/{}", key_of(n)),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    main_of(&reply.body).to_owned()
}

/// Send `GET` to the parameter page of the seeded parameter `n` in version 1.
async fn page_of(world: &World, n: u128) -> String {
    version_page_of(world, 1, n).await
}

/// Return the start tag of the input `id`, from `<input id="..."` to the next `>`.
fn input_tag<'a>(main: &'a str, id: &str) -> &'a str {
    let start = format!("<input id=\"{id}\"");
    let found = main.find(&start);
    assert!(found.is_some(), "no input {id}: {main}");
    let rest = main.get(found.unwrap()..).unwrap();
    rest.split('>').next().unwrap()
}

/// Return the error element of the input `id`, from its opening tag to its closing `</div>`.
fn error_element<'a>(main: &'a str, id: &str) -> &'a str {
    let start = format!("<div class=\"field-error\" id=\"{id}-error\">");
    let found = main.find(&start);
    assert!(found.is_some(), "no error element {id}-error: {main}");
    let rest = main.get(found.unwrap()..).unwrap();
    rest.split("</div>").next().unwrap()
}

/// Assert that the error element of the input `id` holds the code `code` exactly once.
fn assert_line(main: &str, id: &str, code: &str) {
    let element = error_element(main, id);
    let marker = format!("<span class=\"field-error-code\">{code}</span>");
    assert_eq!(element.matches(&marker).count(), 1, "{element}");
    let input = input_tag(main, id);
    assert!(input.contains("aria-invalid=\"true\""), "{input}");
    assert!(input.contains(&format!("{id}-error")), "{input}");
}

/// Assert that the read-only page shows the line `code` under the cell `id` and has no form.
fn assert_read_only_line(main: &str, id: &str, code: &str) {
    let element = error_element(main, id);
    let marker = format!("<span class=\"field-error-code\">{code}</span>");
    assert_eq!(element.matches(&marker).count(), 1, "{element}");
    assert!(!main.contains("<form id=\"parameter-edit-form\""), "{main}");
    assert!(!main.contains(&format!("<input id=\"{id}\"")), "{main}");
}

#[tokio::test]
async fn parameter_page_shows_the_stored_real_with_the_line_e_rcp_055_under_the_value() {
    let world = fault_world(VersionState::LiveDraft);
    let main = page_of(&world, RATE).await;
    let input = input_tag(&main, "value-0");
    assert!(input.contains("value=\"16777217\""), "{input}");
    assert_line(&main, "value-0", "E-RCP-055");
}

#[tokio::test]
async fn parameter_page_shows_the_line_e_rcp_006_under_the_limit_max() {
    let world = fault_world(VersionState::LiveDraft);
    let main = page_of(&world, DUE).await;
    let input = input_tag(&main, "limits-0-max");
    assert!(input.contains("value=\"D#2169-01-17\""), "{input}");
    assert_line(&main, "limits-0-max", "E-RCP-006");
    assert!(!main.contains("id=\"value-0-error\""), "{main}");
    assert!(!main.contains("id=\"limits-0-min-error\""), "{main}");
}

#[tokio::test]
async fn parameter_page_shows_the_line_e_rcp_056_under_the_s7_address() {
    let world = fault_world(VersionState::LiveDraft);
    let main = page_of(&world, OFFSET).await;
    let input = input_tag(&main, "s7_address");
    assert!(input.contains("value=\"DB1.DBD65534\""), "{input}");
    assert_line(&main, "s7_address", "E-RCP-056");
}

#[tokio::test]
async fn approved_version_page_shows_the_stored_real_with_the_line_e_rcp_055() {
    let world = fault_world(VersionState::Approved);
    let main = page_of(&world, RATE).await;
    assert!(main.contains(">16777217<"), "{main}");
    assert_read_only_line(&main, "value-0", "E-RCP-055");
}

#[tokio::test]
async fn approved_version_page_shows_the_line_e_rcp_006_under_the_limit_max() {
    let world = fault_world(VersionState::Approved);
    let main = page_of(&world, DUE).await;
    assert!(main.contains(">D#2169-01-17<"), "{main}");
    assert_read_only_line(&main, "limits-0-max", "E-RCP-006");
    assert!(!main.contains("id=\"value-0-error\""), "{main}");
}

#[tokio::test]
async fn approved_version_page_shows_the_line_e_rcp_056_under_the_s7_address() {
    let world = fault_world(VersionState::Approved);
    let main = page_of(&world, OFFSET).await;
    assert!(main.contains(">DB1.DBD65534<"), "{main}");
    assert_read_only_line(&main, "s7_address", "E-RCP-056");
}

/// Start the draft version 2 of `R-150` from the approved version 1 with `CreateVersion`.
async fn copy_to_draft(world: &World) {
    let reply = world
        .post(
            &world.engineer,
            "/recipes/R-150/versions",
            "based_on=1&change_note=Copy",
            false,
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
}

#[tokio::test]
async fn copied_draft_shows_the_stored_real_with_the_line_e_rcp_055_under_the_value() {
    let world = fault_world(VersionState::Approved);
    copy_to_draft(&world).await;
    let main = version_page_of(&world, 2, RATE).await;
    let input = input_tag(&main, "value-0");
    assert!(input.contains("value=\"16777217\""), "{input}");
    assert_line(&main, "value-0", "E-RCP-055");
}

#[tokio::test]
async fn copied_draft_shows_the_line_e_rcp_006_under_the_limit_max() {
    let world = fault_world(VersionState::Approved);
    copy_to_draft(&world).await;
    let main = version_page_of(&world, 2, DUE).await;
    let input = input_tag(&main, "limits-0-max");
    assert!(input.contains("value=\"D#2169-01-17\""), "{input}");
    assert_line(&main, "limits-0-max", "E-RCP-006");
}

#[tokio::test]
async fn copied_draft_shows_the_line_e_rcp_056_under_the_s7_address() {
    let world = fault_world(VersionState::Approved);
    copy_to_draft(&world).await;
    let main = version_page_of(&world, 2, OFFSET).await;
    let input = input_tag(&main, "s7_address");
    assert!(input.contains("value=\"DB1.DBD65534\""), "{input}");
    assert_line(&main, "s7_address", "E-RCP-056");
}

/// The sent fields of the edit form of `PRECISE` with the type `type_text` in `type`.
fn precise_fields(type_text: &str) -> Vec<(&str, &str)> {
    vec![
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "PRECISE"),
        ("name_en", "PRECISE"),
        ("description_en", "A seeded parameter"),
        ("type", type_text),
        ("value_type", "REAL"),
        ("s7_address", ""),
        ("ordinal", "4"),
        ("value", "1"),
        ("limits[0].leaf_path", ""),
        ("limits[0].max", "100.00001"),
        ("fields", "1"),
    ]
}

/// Send the fields of `PRECISE` with `fields=1` to the edit route of version 1 as an HTMX request.
async fn fields_answer_of_precise(world: &World, type_text: &str) -> Reply {
    let path = format!(
        "/recipes/R-150/versions/1/parameters/{}/definition",
        key_of(PRECISE)
    );
    post_htmx(world, &path, &precise_fields(type_text)).await
}

#[tokio::test]
async fn fields_fragment_shows_the_line_e_rcp_055_under_the_stored_limit_max() {
    let world = fault_world(VersionState::LiveDraft);
    let reply = fields_answer_of_precise(&world, "REAL").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let input = input_tag(&reply.body, "limits-0-max");
    assert!(input.contains("value=\"100.00001\""), "{input}");
    assert_line(&reply.body, "limits-0-max", "E-RCP-055");
    assert!(
        !reply.body.contains("id=\"value-0-error\""),
        "{}",
        reply.body
    );
}

/// The identifier of the global type `ARRAY[1..2] OF REAL` on `S7_1500` that the array tests seed.
const PAIR_TYPE_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0f21);
/// The key of the parameter `PAIR` of type `ARRAY[1..2] OF REAL`. Its limit `[*]` breaks `E-RCP-055`.
const PAIR: u128 = 5;

/// Seed the world with the parameter `PAIR`: two `REAL` values and the shared limit `[*]`.
fn array_fault_world() -> World {
    let world = fault_world(VersionState::LiveDraft);
    let real = seed_type_id(Primitive::Real, None).unwrap();
    let shape = RecordShape::Array {
        element: real,
        dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(1, 2).unwrap()]).unwrap(),
    };
    let row = PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(PAIR_TYPE_ID),
        machine: Some(MachineId::from(MACHINE_ID)),
        platforms: BTreeSet::from([PlcPlatform::S7_1500]),
        shape,
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
    let limit = StoredLimit {
        leaf_path: LeafPath::parse("[*]").unwrap(),
        min: None,
        max: Some("100.00001".parse().unwrap()),
        max_length: None,
    };
    seed_stored(
        &world,
        (PAIR, "PAIR"),
        (
            PlcTypeId::from(PAIR_TYPE_ID),
            StoredValue::from(json!([1, 2])),
            vec![limit],
            None,
        ),
    );
    world
}

/// The sent fields of the edit form of `PAIR` with the shared limit `[*]`.
fn pair_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "PAIR"),
        ("name_en", "PAIR"),
        ("description_en", "A seeded parameter"),
        ("type", "ARRAY[1..2] OF REAL"),
        ("value_type", "ARRAY[1..2] OF REAL"),
        ("s7_address", ""),
        ("ordinal", "5"),
        ("value[1]", "1"),
        ("value[2]", "2"),
        ("limits[0].leaf_path", "[*]"),
        ("limits[0].max", "100.00001"),
        ("fields", "1"),
    ]
}

#[tokio::test]
async fn fields_fragment_shows_the_line_e_rcp_055_under_the_shared_limit_of_an_array() {
    let world = array_fault_world();
    let path = format!(
        "/recipes/R-150/versions/1/parameters/{}/definition",
        key_of(PAIR)
    );
    let reply = post_htmx(&world, &path, &pair_fields()).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let input = input_tag(&reply.body, "limits-0-max");
    assert!(input.contains("value=\"100.00001\""), "{input}");
    assert_line(&reply.body, "limits-0-max", "E-RCP-055");
}

#[tokio::test]
async fn fields_fragment_shows_the_line_e_rcp_055_under_each_element_of_a_split_shared_limit() {
    let world = array_fault_world();
    let path = format!(
        "/recipes/R-150/versions/1/parameters/{}/definition",
        key_of(PAIR)
    );
    let mut fields = pair_fields();
    fields.push(("limit_mode", "[*]"));
    let reply = post_htmx(&world, &path, &fields).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    for id in ["limits-0-max", "limits-1-max"] {
        let input = input_tag(&reply.body, id);
        assert!(input.contains("value=\"100.00001\""), "{input}");
        assert_line(&reply.body, id, "E-RCP-055");
    }
}

#[tokio::test]
async fn parameter_page_shows_the_line_e_rcp_055_under_the_shared_limit_of_an_array() {
    let world = array_fault_world();
    let main = page_of(&world, PAIR).await;
    assert_line(&main, "limits-0-max", "E-RCP-055");
}

/// Return `fields` with the value of the field `name` set to `text`.
fn with_field<'a>(
    fields: Vec<(&'a str, &'a str)>,
    name: &str,
    text: &'a str,
) -> Vec<(&'a str, &'a str)> {
    fields
        .into_iter()
        .map(|(field, old)| {
            if field == name {
                (field, text)
            } else {
                (field, old)
            }
        })
        .collect()
}

#[tokio::test]
async fn fields_fragment_shows_no_line_under_a_limit_max_that_the_user_changed() {
    let world = fault_world(VersionState::LiveDraft);
    let path = format!(
        "/recipes/R-150/versions/1/parameters/{}/definition",
        key_of(PRECISE)
    );
    let fields = with_field(precise_fields("REAL"), "limits[0].max", "100");
    let reply = post_htmx(&world, &path, &fields).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let input = input_tag(&reply.body, "limits-0-max");
    assert!(input.contains("value=\"100\""), "{input}");
    assert!(!reply.body.contains("field-error-code"), "{}", reply.body);
}

#[tokio::test]
async fn fields_fragment_shows_no_line_under_an_s7_address_that_the_user_changed() {
    let world = fault_world(VersionState::LiveDraft);
    let path = format!(
        "/recipes/R-150/versions/1/parameters/{}/definition",
        key_of(OFFSET)
    );
    let fields = vec![
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "OFFSET"),
        ("name_en", "OFFSET"),
        ("description_en", "A seeded parameter"),
        ("type", "DINT"),
        ("value_type", "DINT"),
        ("s7_address", "DB1.DBD0"),
        ("ordinal", "3"),
        ("value", "0"),
        ("fields", "1"),
    ];
    let reply = post_htmx(&world, &path, &fields).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let input = input_tag(&reply.body, "s7_address");
    assert!(input.contains("value=\"DB1.DBD0\""), "{input}");
    assert!(!reply.body.contains("field-error-code"), "{}", reply.body);
}

#[tokio::test]
async fn fields_fragment_shows_no_line_under_a_value_that_the_user_changed() {
    let world = fault_world(VersionState::LiveDraft);
    let path = format!(
        "/recipes/R-150/versions/1/parameters/{}/definition",
        key_of(RATE)
    );
    let stored = vec![
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "RATE"),
        ("name_en", "RATE"),
        ("description_en", "A seeded parameter"),
        ("type", "REAL"),
        ("value_type", "REAL"),
        ("s7_address", ""),
        ("ordinal", "1"),
        ("value", "16777217"),
        ("fields", "1"),
    ];
    let kept = post_htmx(&world, &path, &stored).await;
    assert_line(&kept.body, "value-0", "E-RCP-055");
    let changed = post_htmx(&world, &path, &with_field(stored, "value", "5")).await;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
    assert!(
        !changed.body.contains("field-error-code"),
        "{}",
        changed.body
    );
}

#[tokio::test]
async fn fields_fragment_shows_the_line_under_the_unchanged_element_of_a_split_shared_limit() {
    let world = array_fault_world();
    let path = format!(
        "/recipes/R-150/versions/1/parameters/{}/definition",
        key_of(PAIR)
    );
    let fields = vec![
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "PAIR"),
        ("name_en", "PAIR"),
        ("description_en", "A seeded parameter"),
        ("type", "ARRAY[1..2] OF REAL"),
        ("value_type", "ARRAY[1..2] OF REAL"),
        ("s7_address", ""),
        ("ordinal", "5"),
        ("value[1]", "1"),
        ("value[2]", "2"),
        ("limits[0].leaf_path", "[1]"),
        ("limits[0].max", "100.00001"),
        ("limits[1].leaf_path", "[2]"),
        ("limits[1].max", "100"),
        ("fields", "1"),
    ];
    let reply = post_htmx(&world, &path, &fields).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let first = input_tag(&reply.body, "limits-0-max");
    assert!(first.contains("value=\"100.00001\""), "{first}");
    assert_line(&reply.body, "limits-0-max", "E-RCP-055");
    let second = input_tag(&reply.body, "limits-1-max");
    assert!(second.contains("value=\"100\""), "{second}");
    assert!(
        !reply.body.contains("id=\"limits-1-max-error\""),
        "{}",
        reply.body
    );
    assert_eq!(
        reply.body.matches("field-error-code").count(),
        1,
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn fields_fragment_of_another_type_shows_no_line_of_the_stored_type() {
    let world = fault_world(VersionState::LiveDraft);
    let reply = fields_answer_of_precise(&world, "LREAL").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("field-error-code"), "{}", reply.body);
}
