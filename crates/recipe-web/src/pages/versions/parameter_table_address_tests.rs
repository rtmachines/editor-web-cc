//! Tests of the field lines under the S7 address of the parameter table (ADR-0118 (a), (c), (f)).
//!
//! The tests send `GET /recipes/{ref}/versions/{n}` through the router with the memory adapter.
//! The seeded address breaks `E-RCP-056` (end of the data block) or `E-RCP-057` (S7-300 data
//! block above 16000). The page shows the stored address and the line under it.

use axum::http::StatusCode;
use recipe_app::memory::seed_type_id;
use recipe_domain::plant::{
    MachineId, PlatformTypes, PlcPlatform, Primitive, S7Address, TypeTree, UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, RecipeVersionId,
    StoredParameter, VersionState,
};
use recipe_domain::text::Text;
use uuid::Uuid;

use super::parameter_table_value_tests::{DATE_PAGE, seed_date_world};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{DRAFT_ID, World, main_of};
use crate::test_support::recipes::date_world::{MACHINE_ID, VERSION_ID};

/// The version page of the draft version 3 of `R-100` on the S7-300 machine `M7`.
const DRAFT_PAGE: &str = "/recipes/R-100/versions/3";
/// The first field of the identifier and of the key of a parameter that a test adds.
const EXTRA_BASE: u128 = 0x7357_0006_0000_0000_0000_0000_0e00_0000;
/// The count of a `DATE` that passes the current rule set.
const GOOD_DATE_COUNT: i64 = 72_683;

/// Return the part of `html` from the row of the parameter `tag` to the end of that row.
fn row_of<'a>(html: &'a str, tag: &str) -> &'a str {
    // The outdated list stands before the table, and it links the same tags.
    let table = html.rsplit("<tbody id=\"parameter-rows\">").next().unwrap();
    let start = table.find(&format!(">{tag}</a></td>")).unwrap();
    let rest = table.get(start..).unwrap();
    let end = rest.find("</tr>").unwrap();
    rest.get(..end).unwrap()
}

/// Return the identifier of the parameter that test number `n` adds.
fn extra_id(n: u32) -> Uuid {
    Uuid::from_u128(EXTRA_BASE | u128::from(n))
}

/// Return the address cell block of the row `row` for the parameter that test number `n` adds.
fn address_block(n: u32) -> String {
    format!("address-{}-error", extra_id(n))
}

/// Seed a copy of the parameter `source` of `version` under the tag `tag` and the address `address`.
///
/// `n` makes the identifier, the key, and the ordinal unique. The copy keeps the type, the value,
/// and the limits of `source`.
fn seed_copy(
    world: &World,
    version: RecipeVersionId,
    source: &str,
    (n, tag, address): (u32, &str, &str),
) {
    let state = world.fixture.store.state();
    let found = state
        .parameters_of(version)
        .into_iter()
        .find(|parameter| parameter.tag.as_str() == source)
        .unwrap();
    drop(state);
    let copy = StoredParameter {
        id: ParameterId::from(extra_id(n)),
        key: ParameterKey::from(extra_id(n)),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse(tag).unwrap(),
        s7_address: Some(S7Address::parse(address).unwrap()),
        ordinal: Ordinal::parse(n.checked_add(100).unwrap()).unwrap(),
        ..found
    };
    world.fixture.store.seed_parameter(version, &copy);
}

/// Seed a `DINT` parameter `tag` with the value 7 at `address` in the version of `R-400` on the S7-1500.
fn seed_dint(world: &World, (n, tag, address): (u32, &str, &str)) {
    let type_id = seed_type_id(Primitive::Dint, None).unwrap();
    let state = world.fixture.store.state();
    let machine = state.machine(MachineId::from(MACHINE_ID)).unwrap();
    let unit = machine.unit(&UnitPath::parse("U1").unwrap()).unwrap().id();
    let rows = vec![state.plc_type(type_id).cloned().unwrap()];
    drop(state);
    let types = PlatformTypes::collect(machine.id(), PlcPlatform::S7_1500, rows).unwrap();
    let tree = TypeTree::resolve(type_id, &types).unwrap();
    let value = ParameterValue::parse("7", &tree).unwrap().to_stored();
    let parameter = StoredParameter {
        id: ParameterId::from(extra_id(n)),
        key: ParameterKey::from(extra_id(n)),
        unit,
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse(tag).unwrap(),
        description_en: recipe_domain::text::LongText::parse("A seeded counter").unwrap(),
        translations: Vec::new(),
        plc_type: type_id,
        s7_address: Some(S7Address::parse(address).unwrap()),
        limits: Vec::new(),
        value,
        ordinal: Ordinal::parse(n.checked_add(100).unwrap()).unwrap(),
    };
    world
        .fixture
        .store
        .seed_parameter(RecipeVersionId::from(VERSION_ID), &parameter);
}

/// Assert that the address cell of `row` shows `address`, then the line `code` in the block `block`.
fn assert_address_line(row: &str, (address, code): (&str, &str), block: &str) {
    let cell = format!(
        "<td class=\"data\" aria-describedby=\"{block}\">{address}<div class=\"field-error\" id=\"{block}\""
    );
    assert!(row.contains(&cell), "{row}");
    assert!(row.contains(code), "{row}");
}

#[tokio::test]
async fn parameter_table_shows_e_rcp_056_under_a_dint_that_ends_past_the_data_block() {
    let world = recipe_world();
    seed_date_world(&world, GOOD_DATE_COUNT, VersionState::LiveDraft);
    seed_dint(&world, (1, "COUNTER", "DB1.DBD65534"));
    let reply = world.get_en(&world.engineer, DATE_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(main_of(&reply.body), "COUNTER");
    assert_address_line(row, ("DB1.DBD65534", "E-RCP-056"), &address_block(1));
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn parameter_table_shows_no_line_under_a_dint_that_ends_at_the_last_byte() {
    let world = recipe_world();
    seed_date_world(&world, GOOD_DATE_COUNT, VersionState::LiveDraft);
    seed_dint(&world, (2, "LAST", "DB1.DBD65532"));
    let reply = world.get_en(&world.engineer, DATE_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(main_of(&reply.body), "LAST");
    assert!(
        row.contains("<td class=\"data\">DB1.DBD65532</td>"),
        "{row}"
    );
    assert!(!row.contains("field-error"), "{row}");
}

#[tokio::test]
async fn parameter_table_shows_e_rcp_057_under_an_s7_300_data_block_above_16000() {
    let world = recipe_world();
    seed_copy(
        &world,
        RecipeVersionId::from(DRAFT_ID),
        "SPEED",
        (3, "FAR", "DB16001.DBW0"),
    );
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(main_of(&reply.body), "FAR");
    assert_address_line(row, ("DB16001.DBW0", "E-RCP-057"), &address_block(3));
}

#[tokio::test]
async fn parameter_table_shows_the_address_line_of_a_struct_parameter() {
    let world = recipe_world();
    seed_copy(
        &world,
        RecipeVersionId::from(DRAFT_ID),
        "LINE_A",
        (4, "FAR_LINE", "DB16001.DBB0"),
    );
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(main_of(&reply.body), "FAR_LINE");
    assert_address_line(row, ("DB16001.DBB0", "E-RCP-057"), &address_block(4));
}

#[tokio::test]
async fn parameter_table_shows_the_address_line_of_an_array_parameter_that_ends_past_the_block() {
    let world = recipe_world();
    seed_copy(
        &world,
        RecipeVersionId::from(DRAFT_ID),
        "SPEED_TABLE",
        (5, "LONG_TABLE", "DB1.DBB65530"),
    );
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(main_of(&reply.body), "LONG_TABLE");
    assert_address_line(row, ("DB1.DBB65530", "E-RCP-056"), &address_block(5));
}

#[tokio::test]
async fn parameter_table_shows_no_address_line_for_the_good_addresses_of_the_draft() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(!main.contains("field-error"), "{main}");
    assert!(!main.contains("aria-describedby=\"address-"), "{main}");
}
