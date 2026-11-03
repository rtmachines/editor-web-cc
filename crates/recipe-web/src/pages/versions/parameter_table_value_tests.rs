//! Tests of the column Value of the parameter table (task 4.1).
//!
//! Spec "Value column of the parameter table", ADR-0108 (a), design D1 step 4. The tests send
//! `GET /recipes/{ref}/versions/{n}` and `POST .../parameters` through the router with the memory
//! adapter.

use std::collections::BTreeSet;

use askama::Template;
use axum::http::StatusCode;
use recipe_app::memory::seed_type_id;
use recipe_domain::plant::{
    MachineId, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    RecordShape, StringKind, StringLength, TypeTree, UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, RecipeVersionId,
    StoredParameter, StoredValue,
};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use super::parameter_table::{ParameterTable, VALUE_CHARS, column_count, cut_value};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::parameters::test_kit::{ADD, INT_FIELDS, changed, post_htmx};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error};
use crate::test_support::plant::{APPROVED_ID, DRAFT_ID, M7_ID, POLISH, World, main_of};
use crate::test_support::text;

/// The version page of the approved version 2 of `R-100`.
const APPROVED_PAGE: &str = "/recipes/R-100/versions/2";
/// The version page of the draft version 3 of `R-100`.
const DRAFT_PAGE: &str = "/recipes/R-100/versions/3";
/// The version page of the draft of `R-200`. The draft has no parameter, and its machine has no
/// S7 address.
const EMPTY_PAGE: &str = "/recipes/R-200/versions/1";
/// The identifier of the type `STRING(254)` of `M7`.
const STRING_254_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0f01);
/// The first field of the identifier and of the key of a parameter that a test adds.
const EXTRA_BASE: u128 = 0x7357_0006_0000_0000_0000_0000_0f00_0000;
/// The length of the long text of the scenario "Long text" in characters.
const LONG_LENGTH: usize = 200;

/// Return the opening of the cell of a text value and the closing of the row cell.
fn data_cell(value: &str) -> String {
    format!("<td class=\"data\">{value}</td>")
}

/// Return the cell of a fixed word (`value-note`).
fn note_cell(word: &str) -> String {
    format!("<td class=\"value-note\">{word}</td>")
}

/// Return the part of `html` from the row of the parameter `tag` to the end of that row.
fn row_of<'a>(html: &'a str, tag: &str) -> &'a str {
    // The outdated list stands before the table, and it links the same tags.
    let table = html.rsplit("<tbody id=\"parameter-rows\">").next().unwrap();
    let start = table.find(&format!(">{tag}</a></td>")).unwrap();
    let rest = table.get(start..).unwrap();
    let end = rest.find("</tr>").unwrap();
    rest.get(..end).unwrap()
}

/// Seed the type `STRING(254)` on `M7`.
fn seed_string_254(world: &World) {
    let length = StringLength::parse(StringKind::String, PlcPlatform::S7_300, 254).unwrap();
    let row = PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(STRING_254_ID),
        machine: Some(MachineId::from(M7_ID)),
        platforms: BTreeSet::from([PlcPlatform::S7_300]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: length.size_bytes(StringKind::String).unwrap(),
            string_length: Some(length.value()),
        },
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
}

/// Resolve the tree of the primitive type `type_id` on the S7-300 of `M7`.
fn tree_of(world: &World, type_id: PlcTypeId) -> TypeTree {
    let state = world.fixture.store.state();
    let rows = vec![state.plc_type(type_id).cloned().unwrap()];
    let types = PlatformTypes::collect(MachineId::from(M7_ID), PlcPlatform::S7_300, rows).unwrap();
    TypeTree::resolve(type_id, &types).unwrap()
}

/// Seed a parameter `tag` of the type `type_id` with the stored value `stored` on `version`.
///
/// `n` makes the identifier, the key, and the ordinal unique.
fn seed_with(world: &World, version: RecipeVersionId, (n, tag): (u32, &str), stored: Seed) {
    let state = world.fixture.store.state();
    let machine = state.machine(MachineId::from(M7_ID)).unwrap();
    let unit = machine.unit(&UnitPath::parse("U1").unwrap()).unwrap().id();
    drop(state);
    let id = Uuid::from_u128(EXTRA_BASE | u128::from(n));
    let parameter = StoredParameter {
        id: ParameterId::from(id),
        key: ParameterKey::from(id),
        unit,
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse(tag).unwrap(),
        description_en: LongText::parse("A seeded parameter").unwrap(),
        translations: Vec::new(),
        plc_type: stored.type_id,
        s7_address: None,
        limits: Vec::new(),
        value: stored.value,
        ordinal: Ordinal::parse(n.checked_add(100).unwrap()).unwrap(),
    };
    world.fixture.store.seed_parameter(version, &parameter);
}

/// The type and the stored value of a parameter that a test seeds.
struct Seed {
    type_id: PlcTypeId,
    value: StoredValue,
}

/// Build the seed of a primitive `primitive` with the value `json` (a JSON text).
fn primitive_seed(world: &World, primitive: Primitive, json: &str) -> Seed {
    let type_id = seed_type_id(primitive, None).unwrap();
    let value = ParameterValue::parse(json, &tree_of(world, type_id)).unwrap();
    Seed {
        type_id,
        value: value.to_stored(),
    }
}

/// Return the identifier of the draft version 3 of `R-100`.
fn draft_id() -> RecipeVersionId {
    RecipeVersionId::from(DRAFT_ID)
}

#[tokio::test]
async fn parameter_table_value_column_follows_the_type_column_with_the_header() {
    let world = recipe_world();
    let reply = world.get_en(&world.operator, DRAFT_PAGE).await;
    let main = main_of(&reply.body);
    let type_then_value = "<th scope=\"col\">Type</th><th scope=\"col\">Value</th><th scope=\"col\">Machine unit</th>";
    assert!(main.contains(type_then_value), "{main}");
}

#[tokio::test]
async fn parameter_table_shows_the_elementary_value_of_an_approved_version() {
    let world = recipe_world();
    let speed = primitive_seed(&world, Primitive::Int, "1200");
    seed_with(
        &world,
        RecipeVersionId::from(APPROVED_ID),
        (1, "SPEED"),
        speed,
    );
    let reply = world.get_en(&world.operator, APPROVED_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(main_of(&reply.body), "SPEED");
    let cells = format!("{}{}", data_cell("INT"), data_cell("1200"));
    assert!(row.contains(&cells), "{row}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn parameter_table_shows_the_composite_word_for_a_struct_and_an_array() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let main = main_of(&reply.body);
    let word = text(Lang::En, "parameters-value-composite");
    for tag in ["LINE_A", "SPEED_TABLE", "GRID", "ZONES"] {
        let row = row_of(main, tag);
        assert!(row.contains(&note_cell(&word)), "{tag}: {row}");
    }
    assert!(!main.contains("&#34;head&#34;"), "{main}");
}

#[tokio::test]
async fn parameter_table_cuts_a_long_text_by_characters_and_ends_it_with_an_ellipsis() {
    let world = recipe_world();
    seed_string_254(&world);
    // The characters 61 to 200 have two bytes. A cut by bytes would keep fewer characters.
    let long: String = "a".repeat(60) + &"é".repeat(LONG_LENGTH - 60);
    assert_eq!(long.chars().count(), LONG_LENGTH);
    let tree = tree_of(&world, PlcTypeId::from(STRING_254_ID));
    let json = format!("\"{long}\"");
    let value = ParameterValue::parse(&json, &tree).unwrap().to_stored();
    let seed = Seed {
        type_id: PlcTypeId::from(STRING_254_ID),
        value,
    };
    seed_with(&world, draft_id(), (2, "NOTE"), seed);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(main_of(&reply.body), "NOTE");
    // Hand count: 60 `a`, then 4 `é` to reach 64 characters, then the ellipsis.
    let expected = data_cell(&format!("{}{}…", "a".repeat(60), "é".repeat(4)));
    assert!(row.contains(&expected), "{row}");
    assert!(!row.contains(&"é".repeat(5)), "{row}");
}

#[tokio::test]
async fn parameter_table_keeps_a_text_of_exactly_the_cut_length() {
    let world = recipe_world();
    seed_string_254(&world);
    let exact = "b".repeat(64);
    let tree = tree_of(&world, PlcTypeId::from(STRING_254_ID));
    let value = ParameterValue::parse(&format!("\"{exact}\""), &tree)
        .unwrap()
        .to_stored();
    let seed = Seed {
        type_id: PlcTypeId::from(STRING_254_ID),
        value,
    };
    seed_with(&world, draft_id(), (3, "EXACT"), seed);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let row = row_of(main_of(&reply.body), "EXACT");
    assert!(row.contains(&data_cell(&exact)), "{row}");
    assert!(!row.contains('…'), "{row}");
}

#[tokio::test]
async fn parameter_table_shows_the_empty_word_for_an_empty_text() {
    let world = recipe_world();
    seed_string_254(&world);
    let tree = tree_of(&world, PlcTypeId::from(STRING_254_ID));
    let value = ParameterValue::parse("\"\"", &tree).unwrap().to_stored();
    let seed = Seed {
        type_id: PlcTypeId::from(STRING_254_ID),
        value,
    };
    seed_with(&world, draft_id(), (4, "BLANK"), seed);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let row = row_of(main_of(&reply.body), "BLANK");
    let word = text(Lang::En, "parameters-value-empty");
    assert!(row.contains(&note_cell(&word)), "{row}");
}

#[tokio::test]
async fn parameter_table_shows_empty_text_for_an_empty_text_in_english() {
    let world = recipe_world();
    seed_string_254(&world);
    let tree = tree_of(&world, PlcTypeId::from(STRING_254_ID));
    let value = ParameterValue::parse("\"\"", &tree).unwrap().to_stored();
    let seed = Seed {
        type_id: PlcTypeId::from(STRING_254_ID),
        value,
    };
    seed_with(&world, draft_id(), (4, "BLANK"), seed);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let row = row_of(main_of(&reply.body), "BLANK");
    assert!(row.contains(&note_cell("Empty text")), "{row}");
}

#[tokio::test]
async fn parameter_table_shows_pusty_tekst_for_an_empty_text_in_polish() {
    let world = recipe_world();
    seed_string_254(&world);
    let tree = tree_of(&world, PlcTypeId::from(STRING_254_ID));
    let value = ParameterValue::parse("\"\"", &tree).unwrap().to_stored();
    let seed = Seed {
        type_id: PlcTypeId::from(STRING_254_ID),
        value,
    };
    seed_with(&world, draft_id(), (4, "BLANK"), seed);
    let reply = world.get(&world.engineer, DRAFT_PAGE, &[POLISH]).await;
    let row = row_of(main_of(&reply.body), "BLANK");
    assert!(row.contains(&note_cell("Pusty tekst")), "{row}");
}

#[tokio::test]
async fn parameter_table_shows_the_iec_literal_of_a_time_row() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let row = row_of(main_of(&reply.body), "HOLD");
    assert!(row.contains(&data_cell("T#1m30s")), "{row}");
}

#[tokio::test]
async fn parameter_table_shows_the_word_of_the_grid_for_a_bool_row() {
    let world = recipe_world();
    let enabled = primitive_seed(&world, Primitive::Bool, "true");
    seed_with(&world, draft_id(), (5, "ENABLED"), enabled);
    let disabled = primitive_seed(&world, Primitive::Bool, "false");
    seed_with(&world, draft_id(), (7, "DISABLED"), disabled);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let main = main_of(&reply.body);
    assert!(
        row_of(main, "ENABLED").contains(&data_cell("TRUE")),
        "{main}"
    );
    assert!(
        row_of(main, "DISABLED").contains(&data_cell("FALSE")),
        "{main}"
    );
}

#[tokio::test]
async fn parameter_table_shows_the_bool_word_of_the_polish_bundle() {
    let world = recipe_world();
    let enabled = primitive_seed(&world, Primitive::Bool, "true");
    seed_with(&world, draft_id(), (5, "ENABLED"), enabled);
    let disabled = primitive_seed(&world, Primitive::Bool, "false");
    seed_with(&world, draft_id(), (7, "DISABLED"), disabled);
    let reply = world.get(&world.engineer, DRAFT_PAGE, &[POLISH]).await;
    let main = main_of(&reply.body);
    let (yes, no) = (text(Lang::Pl, "grid-true"), text(Lang::Pl, "grid-false"));
    assert!(row_of(main, "ENABLED").contains(&data_cell(&yes)), "{main}");
    assert!(row_of(main, "DISABLED").contains(&data_cell(&no)), "{main}");
}

#[tokio::test]
async fn parameter_table_keeps_the_raw_text_of_a_string_row_that_reads_true() {
    let world = recipe_world();
    seed_string_254(&world);
    let tree = tree_of(&world, PlcTypeId::from(STRING_254_ID));
    let value = ParameterValue::parse("\"true\"", &tree)
        .unwrap()
        .to_stored();
    let seed = Seed {
        type_id: PlcTypeId::from(STRING_254_ID),
        value,
    };
    seed_with(&world, draft_id(), (8, "WORD"), seed);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let row = row_of(main_of(&reply.body), "WORD");
    assert!(row.contains(&data_cell("true")), "{row}");
}

#[tokio::test]
async fn parameter_table_answer_row_of_a_new_bool_parameter_shows_the_grid_word() {
    let world = recipe_world();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("tag", Some("FLAG")),
            ("type", Some("BOOL")),
            ("value_type", Some("BOOL")),
            ("s7_address", Some("DB9.DBX0.0")),
            ("value", Some("true")),
        ],
    );
    let reply = post_htmx(&world, ADD, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(&reply.body, "FLAG");
    let cells = format!("{}{}", data_cell("BOOL"), data_cell("TRUE"));
    assert!(row.contains(&cells), "{row}");
}

#[tokio::test]
async fn parameter_table_answer_row_of_a_new_parameter_shows_the_canonical_text() {
    let world = recipe_world();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("tag", Some("TEMP")),
            ("type", Some("REAL")),
            ("value_type", Some("REAL")),
            ("s7_address", Some("DB9.DBD0")),
            ("value", Some("85.50")),
        ],
    );
    let reply = post_htmx(&world, ADD, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(&reply.body, "TEMP");
    let cells = format!("{}{}", data_cell("REAL"), data_cell("85.5"));
    assert!(row.contains(&cells), "{row}");
    assert!(!row.contains("85.50"), "{row}");
}

#[tokio::test]
async fn parameter_table_with_a_stored_value_that_does_not_match_its_type_gives_db_008() {
    let world = recipe_world();
    // The stored text of a `STRING` is a JSON string. It does not fit the type `INT`.
    // The request id is a hex UUID. The marker has letters outside hex, so the id never matches it.
    const LEAK_MARKER: &str = "zqx-leak-marker";
    seed_string_254(&world);
    let tree = tree_of(&world, PlcTypeId::from(STRING_254_ID));
    let seed = Seed {
        type_id: seed_type_id(Primitive::Int, None).unwrap(),
        value: ParameterValue::parse(&format!("\"{LEAK_MARKER}\""), &tree)
            .unwrap()
            .to_stored(),
    };
    seed_with(
        &world,
        RecipeVersionId::from(APPROVED_ID),
        (6, "SPEED"),
        seed,
    );
    let reply = world.get_en(&world.operator, APPROVED_PAGE).await;
    assert_error(
        &reply,
        StatusCode::INTERNAL_SERVER_ERROR,
        "E-DB-008",
        "version page",
    );
    assert!(!reply.body.contains(LEAK_MARKER), "{}", reply.body);
}

#[tokio::test]
async fn parameter_table_empty_state_spans_every_header_cell() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, EMPTY_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_empty_colspan(main_of(&reply.body));
}

/// Render `parameter-table.html` for a table without rows.
#[derive(Template)]
#[template(path = "parameter-table.html")]
struct EmptyTable<'a> {
    t: Texts<'a>,
    table: ParameterTable,
}

#[test]
fn parameter_table_colspan_of_the_empty_state_equals_the_header_cells_with_and_without_address() {
    let bundles = Bundles::load().unwrap();
    for has_address in [false, true] {
        let table = ParameterTable {
            rows: Vec::new(),
            has_address,
            columns: column_count(has_address),
            units: Vec::new(),
            previous_url: String::new(),
            next_url: String::new(),
            current: 1,
            after_end: false,
            first_url: String::new(),
        };
        let html = EmptyTable {
            t: Texts::new(&bundles, Lang::En),
            table,
        }
        .render()
        .unwrap();
        assert_empty_colspan(&html);
    }
}

/// Assert that the `colspan` of the empty state equals the count of the header cells of `html`.
fn assert_empty_colspan(html: &str) {
    let head = html.split("</thead>").next().unwrap();
    let cells = head.matches("<th ").count();
    assert!(cells > 0, "{html}");
    let empty = format!("<td colspan=\"{cells}\">");
    assert!(html.contains(&empty), "{cells} header cells: {html}");
}

#[test]
fn parameter_table_cut_value_counts_characters_and_never_splits_one() {
    let two_bytes = "é".repeat(65);
    let cut = cut_value(&two_bytes);
    assert_eq!(cut, format!("{}…", "é".repeat(64)));
    assert_eq!(cut.chars().count(), 65);
}

#[test]
fn parameter_table_cut_value_keeps_a_short_text_and_a_text_of_the_limit() {
    assert_eq!(cut_value("1200"), "1200");
    let limit = "é".repeat(64);
    assert_eq!(cut_value(&limit), limit);
    let one_over = "x".repeat(65);
    assert_eq!(cut_value(&one_over), format!("{}…", "x".repeat(64)));
}

/// Return the value cell of the row `table.value_chars` in `reliability.md` section 7.11.
fn value_chars_cell() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/architecture/reliability.md"
    );
    let document = std::fs::read_to_string(path).unwrap();
    let row = document
        .lines()
        .find(|line| line.starts_with("| `table.value_chars` |"))
        .unwrap();
    row.split('|').nth(2).unwrap().trim().to_owned()
}

#[test]
fn value_chars_equals_the_table() {
    let cell = value_chars_cell();
    let number: usize = cell.split_whitespace().next().unwrap().parse().unwrap();
    assert_eq!(VALUE_CHARS, number);
    assert_eq!(number, 64);
}
