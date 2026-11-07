//! Tests of the data block number of `AddParameter` on an `S7_1500` machine (ADR-0140 (f)).
//!
//! The tests of `add_error_tests` use the `S7_300` of `M7`, which ends at the data block 16000.
//! These tests seed the machine `M15` on `S7_1500` with an empty draft. The highest data block
//! of `S7_1200` and `S7_1500` is 59999, so `DB60000` gives `E-RCP-057` and `DB59999` passes.

use axum::http::StatusCode;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{Machine, MachineCode, MachineId, PlcPlatform, UnitId};
use recipe_domain::recipe::{
    Recipe, RecipeId, RecipeVersionId, ReferenceNumber, StoredRecipe, StoredVersionHeader,
    VersionState,
};
use recipe_domain::text::Text;
use uuid::Uuid;

use super::test_kit::{INT_FIELDS, assert_panel_error, changed, post_htmx};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::summary_lines;
use crate::test_support::plant::World;

/// The identifier of the machine `M15` on `S7_1500`.
const MACHINE_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0f21_0001);
/// The identifier of the unit `U1` of `M15`.
const UNIT_ID: Uuid = Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0f21_0001);
/// The identifier of the recipe `R-151`.
const RECIPE_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0f21_0001);
/// The identifier of the draft 1 of `R-151`.
const DRAFT_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f21_0001);
/// The path of `AddParameter` on the draft 1 of `R-151`.
const ADD_1500: &str = "/recipes/R-151/versions/1/parameters";

/// Parse a short text.
fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Build the world with the machine `M15` on `S7_1500` and the empty draft 1 of `R-151`.
fn s7_1500_world() -> World {
    let world = recipe_world();
    let machine = Machine::create(
        MachineId::from(MACHINE_ID),
        MachineCode::parse("M15").unwrap(),
        text("Test line"),
        PlcPlatform::S7_1500,
        (UnitId::from(UNIT_ID), text("Infeed")),
    );
    world.fixture.store.seed_machine(&machine);
    let recipe = Recipe::try_from(StoredRecipe {
        id: RecipeId::from(RECIPE_ID),
        machine: MachineId::from(MACHINE_ID),
        reference: ReferenceNumber::parse("R-151").unwrap(),
        name: text("Data block recipe"),
        deleted_at: None,
        versions: vec![StoredVersionHeader {
            id: RecipeVersionId::from(DRAFT_ID),
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
        .seed_recipe(&recipe, PlcPlatform::S7_1500, creator);
    world
}

/// Return the text of the error line `s7_address-error` of `body`, up to its closing tag.
fn address_line_text(body: &str) -> &str {
    let line = "<div class=\"field-error\" id=\"s7_address-error\">";
    let (_before, rest) = body.split_once(line).unwrap();
    rest.split("</div>").next().unwrap()
}

#[tokio::test]
async fn parameters_add_on_s7_1500_with_data_block_60000_gives_e_rcp_057_under_the_address() {
    let world = s7_1500_world();
    let before = world.fixture.snapshot();
    let pairs = changed(&INT_FIELDS, &[("s7_address", Some("DB60000.DBW0"))]);
    let reply = post_htmx(&world, ADD_1500, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-057");
    let text = address_line_text(&reply.body);
    assert!(text.contains("E-RCP-057"), "{}", reply.body);
    assert!(text.contains("above 59999"), "{}", reply.body);
    assert!(
        reply.body.contains("href=\"#s7_address\""),
        "{}",
        reply.body
    );
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_add_on_s7_1500_with_data_block_59999_gives_no_line() {
    let world = s7_1500_world();
    let audit = world.audit_count();
    let pairs = changed(&INT_FIELDS, &[("s7_address", Some("DB59999.DBW0"))]);
    let reply = post_htmx(&world, ADD_1500, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("E-RCP-057"), "{}", reply.body);
    assert!(!reply.body.contains("s7_address-error"), "{}", reply.body);
    assert!(world.audit_count() > audit);
}
