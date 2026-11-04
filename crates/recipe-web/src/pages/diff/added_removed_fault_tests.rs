//! HTTP tests of the rule fault lines on the rows of an added and a removed parameter
//! (ADR-0118 (a), (c), (h), review finding R5). Test code.
//!
//! A scalar value can pass the earlier rule set and break the current one. The row of an added
//! parameter shows the value of the new version, and the row of a removed parameter shows the value
//! of the old version. The row then has the field line with the code under the value.

use axum::http::StatusCode;
use recipe_app::memory::seed_type_id;
use recipe_domain::plant::{PlcTypeId, Primitive, UnitId};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, RecipeVersionId, StoredParameter, StoredValue,
};
use recipe_domain::text::LongText;
use serde_json::json;
use uuid::Uuid;

use super::stored_tests::{
    DUE_KEY, RULE_DATE, RULE_NEW, RULE_OLD, RULE_UNIT, seed_date_type,
    seed_two_versions_on_s7_1500, text,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{World, main_of};

/// The diff of the draft version 2 of `R-160` against the approved version 1.
const DIFF: &str = "/recipes/R-160/versions/2/diff/1";
/// The approval preview of the draft version 2 of `R-160`.
const PREVIEW: &str = "/recipes/R-160/versions/2/approve";
/// The stored `DATE` count of `D#2169-01-17`. Only the earlier rule set allows it on `S7_1500`.
const EARLIER_ONLY_DAY: i64 = 72_700;
/// A `REAL` value with 8 digits. It breaks the precision rule of the current rule set.
const TOO_PRECISE: i64 = 16_777_217;
/// The key of the `REAL` parameter `BIG`.
const BIG_KEY: Uuid = Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0f2b_0003);
/// The line of `E-RCP-005`: the class of the code span and the code.
const RCP_005_LINE: &str = "<span class=\"field-error-code\">E-RCP-005</span>";
/// The line of `E-RCP-055`: the class of the code span and the code.
const RCP_055_LINE: &str = "<span class=\"field-error-code\">E-RCP-055</span>";
/// The column of the value in the row of an added or a removed parameter.
const VALUE_COLUMN: usize = 4;

/// Seed the parameter `tag` of the type `plc_type` with the stored value `value` into `version`.
///
/// The parameter has no limit row and no address. `key` is the key and the row identifier.
fn seed_alone(
    world: &World,
    (version, key): (Uuid, Uuid),
    (tag, plc_type): (&str, PlcTypeId),
    value: i64,
) {
    let parameter = StoredParameter {
        id: ParameterId::from(key),
        key: ParameterKey::from(key),
        unit: UnitId::from(RULE_UNIT),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: text(tag),
        description_en: LongText::parse("A seeded parameter").unwrap(),
        translations: Vec::new(),
        plc_type,
        s7_address: None,
        limits: Vec::new(),
        value: StoredValue::from(json!(value)),
        ordinal: Ordinal::parse(1).unwrap(),
    };
    world
        .fixture
        .store
        .seed_parameter(RecipeVersionId::from(version), &parameter);
}

/// Build the world where only the new version 2 holds the `REAL` parameter `BIG`.
fn world_with_added_real() -> World {
    let world = recipe_world();
    seed_two_versions_on_s7_1500(&world);
    let real = seed_type_id(Primitive::Real, None).unwrap();
    seed_alone(&world, (RULE_NEW, BIG_KEY), ("BIG", real), TOO_PRECISE);
    world
}

/// Build the world where only the old version 1 holds the `DATE` parameter `DUE`.
fn world_with_removed_date() -> World {
    let world = recipe_world();
    seed_two_versions_on_s7_1500(&world);
    seed_date_type(&world);
    seed_alone(
        &world,
        (RULE_OLD, DUE_KEY),
        ("DUE", PlcTypeId::from(RULE_DATE)),
        EARLIER_ONLY_DAY,
    );
    world
}

/// Return the cells of the first row of the section `section` in `main`, from the tag to the state.
///
/// Each cell starts at `<td` and ends before the next one.
fn first_row_cells<'a>(main: &'a str, section: &str) -> Vec<&'a str> {
    let start = main.find(&format!(
        "<section class=\"diff-section\" id=\"diff-{section}\""
    ));
    assert!(start.is_some(), "no {section} section: {main}");
    let body = main
        .get(start.unwrap()..)
        .unwrap()
        .split("<tbody>")
        .nth(1)
        .unwrap();
    body.split("</tr>")
        .next()
        .unwrap()
        .split("<td")
        .skip(1)
        .collect()
}

/// Assert that the value cell of the first row of `section` holds `shown` and the line `line`.
///
/// The page must hold the line once, so no other cell shows it.
async fn assert_value_line(world: &World, path: &str, (section, shown, line): (&str, &str, &str)) {
    let reply = world.get_en(&world.engineer, path).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    assert_eq!(main.matches(line).count(), 1, "{main}");
    let cells = first_row_cells(main, section);
    // The cells are: the tag, the name, the unit, the type, the value, and the state.
    assert_eq!(cells.len(), 6, "{cells:?}");
    assert!(
        cells[VALUE_COLUMN].contains(shown),
        "value: {}",
        cells[VALUE_COLUMN]
    );
    assert!(
        cells[VALUE_COLUMN].contains(line),
        "value: {}",
        cells[VALUE_COLUMN]
    );
}

#[tokio::test]
async fn diff_page_shows_the_line_e_rcp_055_in_the_value_cell_of_an_added_row() {
    let world = world_with_added_real();

    assert_value_line(&world, DIFF, ("added", "16777217", RCP_055_LINE)).await;
}

#[tokio::test]
async fn diff_page_shows_the_line_e_rcp_005_in_the_value_cell_of_a_removed_row() {
    let world = world_with_removed_date();

    assert_value_line(&world, DIFF, ("removed", "D#2169-01-17", RCP_005_LINE)).await;
}

#[tokio::test]
async fn approval_preview_shows_the_line_e_rcp_055_in_the_value_cell_of_an_added_row() {
    let world = world_with_added_real();

    assert_value_line(&world, PREVIEW, ("added", "16777217", RCP_055_LINE)).await;
}
