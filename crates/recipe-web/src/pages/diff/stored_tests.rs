//! HTTP tests of a stored row that does not fit its type (ADR-0050 (c), design D9).
//! Test code.
//!
//! Every write path checks a value and a limit row, so such a row is corrupt data. The diff page
//! and the approval preview answer `500 E-DB-008` with the error page. They show no row of the
//! diff and no raw stored text. The approval preview gives no blocker for a row of the Approved
//! version. The `recipe-app` tests of `DiffVersions` and `GetApprovalPreview` cover the use cases.
//!
//! A stored limit that passes the earlier rule set and breaks the current one is a rule fault
//! (ADR-0118). The diff page then keeps the status 200 and shows a field line with the code in the
//! cell of the side that holds the limit (task 8.11, design D7).

use std::collections::BTreeSet;

use axum::http::StatusCode;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    LeafPath, Machine, MachineCode, MachineId, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    Primitive, RecordShape, UnitId,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, Recipe, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredLimit, StoredParameter, StoredRecipe, StoredValue, StoredVersionHeader,
    VersionState,
};
use recipe_domain::text::{LongText, Text};
use serde_json::json;
use uuid::Uuid;

use super::test_world::R500_DIFF;
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error};
use crate::test_support::Reply;
use crate::test_support::plant::{World, main_of};

/// The approval preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";
/// The value of `HX-Reswap` of every HTMX error answer (ADR-0036).
const RESWAP: &str = "innerHTML show:#error-panel:top";
/// The lower bound of the bad limit row. It does not fit the type `INT`.
const BAD_BOUND: &str = "0.5";
/// An `E-DB-008` answer must not show these texts. They are an element of the diff, the tag of the
/// bad row, and the keys of the stored value. The value is the JSON of the parameter `LINE_A`.
/// The entries do not depend on the order of the keys. Askama escapes the quote.
const FORBIDDEN: [&str; 4] = ["diff-section", "SPEED", "&#34;head&#34;", "&#34;count&#34;"];

/// Give the `INT` parameter `SPEED` of `version` the stored value of the struct `LINE_A`.
///
/// That value does not fit the type `INT`. The row keeps its identifier, so the seed replaces it.
async fn corrupt_value(world: &AuditWorld, version: RecipeVersionId) {
    let speed = world.stored_of(version, "SPEED").await;
    let line_a = world.stored_of(version, "LINE_A").await;
    let broken = StoredParameter {
        value: line_a.value,
        ..speed
    };
    world.fixture.store.seed_parameter(version, &broken);
}

/// Give the limit rows of `SPEED` of `version` a lower bound that does not fit the type `INT`.
async fn corrupt_limit(world: &AuditWorld, version: RecipeVersionId) {
    let speed = world.stored_of(version, "SPEED").await;
    let limits: Vec<StoredLimit> = speed
        .limits
        .iter()
        .map(|limit| StoredLimit {
            min: Some(BAD_BOUND.parse().unwrap()),
            ..limit.clone()
        })
        .collect();
    assert!(!limits.is_empty(), "SPEED must have a limit row");
    let broken = StoredParameter { limits, ..speed };
    world.fixture.store.seed_parameter(version, &broken);
}

/// Assert the answer `500 E-DB-008`: the code, a `request_id`, and none of the forbidden text.
/// The answer must not show the texts of `raw` either.
fn assert_db_008(reply: &Reply, what: &str, raw: &[&str]) {
    assert_error(reply, StatusCode::INTERNAL_SERVER_ERROR, "E-DB-008", what);
    let request_id = reply.header("x-request-id");
    assert!(!request_id.is_empty(), "{what}: no request id");
    assert!(reply.body.contains(&request_id), "{what}: {}", reply.body);
    for text in FORBIDDEN.iter().chain(raw) {
        assert!(!reply.body.contains(text), "{what}: {text}: {}", reply.body);
    }
}

/// Send `path` as the Administrator `admin` as a full page request and as an HTMX request. Assert
/// the answer `500 E-DB-008` to both. The HTMX answer has the three error headers of `web.md`.
/// Neither answer shows the texts of `raw`.
async fn assert_both_answers_db_008(world: &AuditWorld, path: &str, raw: &[&str]) {
    let page = world.get_en(&world.data.admin, path).await;
    assert_db_008(&page, "full page", raw);
    assert_eq!(page.header("hx-retarget"), "", "full page: {}", page.body);
    let fragment = world.get_content(&world.data.admin, path).await;
    assert_db_008(&fragment, "htmx", raw);
    assert_htmx_error(&fragment, StatusCode::INTERNAL_SERVER_ERROR, "E-DB-008");
    assert_eq!(fragment.header("hx-reswap"), RESWAP, "{}", fragment.body);
    assert_eq!(fragment.header("hx-push-url"), "false", "{}", fragment.body);
}

#[tokio::test]
async fn diff_page_with_a_stored_value_of_the_new_side_that_does_not_fit_gives_db_008() {
    let world = audit_world().await;
    corrupt_value(&world, world.data.r500_v2).await;

    assert_both_answers_db_008(&world, R500_DIFF, &[]).await;
}

#[tokio::test]
async fn diff_page_with_a_stored_limit_row_that_does_not_fit_gives_db_008() {
    let world = audit_world().await;
    corrupt_limit(&world, world.data.r500_v2).await;

    assert_both_answers_db_008(&world, R500_DIFF, &[BAD_BOUND]).await;
}

#[tokio::test]
async fn approval_preview_with_a_stored_value_of_the_approved_version_that_does_not_fit_gives_db_008()
 {
    let world = audit_world().await;
    corrupt_value(&world, world.data.r500_v1).await;

    assert_both_answers_db_008(&world, R500_PREVIEW, &[]).await;
}

/// The identifier of the machine `M16` on `S7_1500`. Only that platform has the earlier `DATE` range.
pub(crate) const RULE_MACHINE: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0f2b_0001);
/// The identifier of the unit `U1` of `M16`.
pub(crate) const RULE_UNIT: Uuid = Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0f2b_0001);
/// The identifier of the recipe `R-160`.
pub(crate) const RULE_RECIPE: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0f2b_0001);
/// The identifier of the approved version 1 of `R-160`.
pub(crate) const RULE_OLD: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f2b_0001);
/// The identifier of the draft version 2 of `R-160`.
pub(crate) const RULE_NEW: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f2b_0002);
/// The identifier of the global type `DATE` on `S7_1500` that the rule fault test seeds.
pub(crate) const RULE_DATE: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0f2b);
/// The key of the `DATE` parameter `DUE`.
pub(crate) const DUE_KEY: Uuid = Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0f2b_0002);
/// The diff of the draft version 2 of `R-160` against the approved version 1.
const RULE_DIFF: &str = "/recipes/R-160/versions/2/diff/1";
/// The stored `DATE` count of `D#2169-01-17`. Only the earlier rule set allows it on `S7_1500`.
const EARLIER_ONLY_DAY: i64 = 72_700;
/// The line that the test expects: the class of the code span and the code.
const RCP_006_LINE: &str = "<span class=\"field-error-code\">E-RCP-006</span>";

/// Parse a short text.
pub(crate) fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Seed the global type `DATE` on `S7_1500`. The in-memory plant has no `DATE` row.
pub(crate) fn seed_date_type(world: &World) -> PlcTypeId {
    let shape = RecordShape::Primitive {
        primitive: Primitive::Date,
        leaf_class: Primitive::Date.leaf_class(),
        size_bytes: Primitive::Date.fixed_size_bytes().unwrap(),
        string_length: None,
    };
    let row = PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(RULE_DATE),
        machine: None,
        platforms: BTreeSet::from([PlcPlatform::S7_1500]),
        shape,
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
    PlcTypeId::from(RULE_DATE)
}

/// Seed the machine `M16` on `S7_1500` with `R-160`: the approved version 1 and the draft 2.
pub(crate) fn seed_two_versions_on_s7_1500(world: &World) {
    let machine = Machine::create(
        MachineId::from(RULE_MACHINE),
        MachineCode::parse("M16").unwrap(),
        text("Test line"),
        PlcPlatform::S7_1500,
        (UnitId::from(RULE_UNIT), text("Infeed")),
    );
    world.fixture.store.seed_machine(&machine);
    let header = |id: Uuid, number: u32, state: VersionState| StoredVersionHeader {
        id: RecipeVersionId::from(id),
        number,
        state,
        row_version: 1,
    };
    let recipe = Recipe::try_from(StoredRecipe {
        id: RecipeId::from(RULE_RECIPE),
        machine: MachineId::from(RULE_MACHINE),
        reference: ReferenceNumber::parse("R-160").unwrap(),
        name: text("Rule fault recipe"),
        deleted_at: None,
        versions: vec![
            header(RULE_OLD, 1, VersionState::Approved),
            header(RULE_NEW, 2, VersionState::LiveDraft),
        ],
    })
    .unwrap();
    let creator = UserId::from(Uuid::from_u128(1));
    world
        .fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::S7_1500, creator);
}

/// Seed `DUE` with the stored count 10 000 and the upper bound `max` into `version`.
///
/// `id` is the row identifier. Both versions share the key, so the diff pairs the two rows.
fn seed_due(world: &World, (version, id): (Uuid, u128), max: i64) {
    let limit = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(max.to_string().parse().unwrap()),
        max_length: None,
    };
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
        limits: vec![limit],
        value: StoredValue::from(json!(10_000)),
        ordinal: Ordinal::parse(1).unwrap(),
    };
    world
        .fixture
        .store
        .seed_parameter(RecipeVersionId::from(version), &parameter);
}

/// Return the cells of the first row of the limits section in `main`, from the tag to the state.
///
/// Each cell starts at `<td` and ends before the next one.
fn limit_row_cells(main: &str) -> Vec<&str> {
    let start = main.find("<section class=\"diff-section\" id=\"diff-limits\"");
    assert!(start.is_some(), "no limits section: {main}");
    let section = main.get(start.unwrap()..).unwrap();
    let body = section.split("<tbody>").nth(1).unwrap();
    let row = body.split("</tr>").next().unwrap();
    row.split("<td").skip(1).collect()
}

#[tokio::test]
async fn diff_page_shows_the_line_e_rcp_006_in_the_new_cell_of_the_stored_limit_row() {
    let world = recipe_world();
    seed_two_versions_on_s7_1500(&world);
    seed_date_type(&world);
    seed_due(
        &world,
        (RULE_OLD, 0x7357_0007_0000_0000_0000_0000_0f2b_0001),
        20_000,
    );
    seed_due(
        &world,
        (RULE_NEW, 0x7357_0007_0000_0000_0000_0000_0f2b_0002),
        EARLIER_ONLY_DAY,
    );

    let reply = world.get_en(&world.engineer, RULE_DIFF).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    assert_eq!(main.matches(RCP_006_LINE).count(), 1, "{main}");
    let cells = limit_row_cells(main);
    // The cells are: the tag, the path, version 1, version 2, and the state.
    assert_eq!(cells.len(), 5, "{cells:?}");
    assert!(!cells[2].contains("field-error"), "old side: {}", cells[2]);
    assert!(cells[3].contains(RCP_006_LINE), "new side: {}", cells[3]);
    assert!(cells[3].contains("D#2169-01-17"), "new side: {}", cells[3]);
}
