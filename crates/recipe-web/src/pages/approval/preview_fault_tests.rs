//! HTTP test of a stored value on the base side that breaks the current rule set (task 8.12,
//! ADR-0118, design D7). Test code.
//!
//! The base version stays as the stored text says. The approval preview keeps the status 200 and
//! shows the field line with the code in the base cell of the value row.

use axum::http::StatusCode;
use recipe_domain::plant::{PlcTypeId, UnitId};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, RecipeVersionId, StoredParameter, StoredValue,
};
use recipe_domain::text::LongText;
use serde_json::json;
use uuid::Uuid;

use crate::pages::diff::stored_tests::{
    DUE_KEY, RULE_DATE, RULE_NEW, RULE_OLD, RULE_UNIT, seed_date_type,
    seed_two_versions_on_s7_1500, text,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{World, main_of};

/// The approval preview of the draft version 2 of `R-160`.
const RULE_PREVIEW: &str = "/recipes/R-160/versions/2/approve";
/// The stored `DATE` count of `D#2169-01-17`. Only the earlier rule set allows it on `S7_1500`.
const EARLIER_ONLY_DAY: i64 = 72_700;
/// The last `DATE` count of the current rule set on `S7_1500`: `D#2168-12-31`.
const CURRENT_LAST_DAY: i64 = 72_683;
/// The line that the test expects: the class of the code span and the code.
const RCP_005_LINE: &str = "<span class=\"field-error-code\">E-RCP-005</span>";

/// Seed `DUE` of type `DATE` with the stored count `count` and no limit row into `version`.
///
/// `id` is the row identifier. Both versions share the key, so the diff pairs the two rows.
fn seed_unbounded_due(world: &World, (version, id): (Uuid, u128), count: i64) {
    let parameter = StoredParameter {
        id: ParameterId::from(Uuid::from_u128(id)),
        key: ParameterKey::from(DUE_KEY),
        unit: UnitId::from(RULE_UNIT),
        tag: ParameterTag::parse("DUE").unwrap(),
        name_en: text("DUE"),
        description_en: LongText::parse("A seeded parameter").unwrap(),
        translations: Vec::new(),
        plc_type: PlcTypeId::from(RULE_DATE),
        s7_address: None,
        limits: Vec::new(),
        value: StoredValue::from(json!(count)),
        ordinal: Ordinal::parse(1).unwrap(),
    };
    world
        .fixture
        .store
        .seed_parameter(RecipeVersionId::from(version), &parameter);
}

/// Return the cells of the first row of the values section in `main`, from the tag to the state.
///
/// Each cell starts at `<td` and ends before the next one.
fn value_row_cells(main: &str) -> Vec<&str> {
    let start = main.find("<section class=\"diff-section\" id=\"diff-values\"");
    assert!(start.is_some(), "no values section: {main}");
    let section = main.get(start.unwrap()..).unwrap();
    let body = section.split("<tbody>").nth(1).unwrap();
    let row = body.split("</tr>").next().unwrap();
    row.split("<td").skip(1).collect()
}

#[tokio::test]
async fn approval_preview_shows_the_line_e_rcp_005_in_the_base_cell_of_the_value_row() {
    let world = recipe_world();
    seed_two_versions_on_s7_1500(&world);
    seed_date_type(&world);
    seed_unbounded_due(
        &world,
        (RULE_OLD, 0x7357_0007_0000_0000_0000_0000_0f2b_0001),
        EARLIER_ONLY_DAY,
    );
    seed_unbounded_due(
        &world,
        (RULE_NEW, 0x7357_0007_0000_0000_0000_0000_0f2b_0002),
        CURRENT_LAST_DAY,
    );

    let reply = world.get_en(&world.admin, RULE_PREVIEW).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    assert_eq!(main.matches(RCP_005_LINE).count(), 1, "{main}");
    let cells = value_row_cells(main);
    // The cells are: the tag, the path, version 1, version 2, the limit, and the state.
    assert_eq!(cells.len(), 6, "{cells:?}");
    assert!(cells[2].contains(RCP_005_LINE), "base side: {}", cells[2]);
    assert!(cells[2].contains("D#2169-01-17"), "base side: {}", cells[2]);
    assert!(
        !cells[3].contains("field-error"),
        "draft side: {}",
        cells[3]
    );
}
