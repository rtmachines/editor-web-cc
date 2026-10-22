//! HTTP tests of a stored row that does not fit its type (ADR-0050 (c), design D9, review item
//! CR-Z3-3). Test code.
//!
//! Every write path checks a value and a limit row, so such a row is corrupt data. The diff page
//! and the approval preview answer `500 E-DB-008` with the error page. They show no row of the
//! diff and no raw stored text. The approval preview gives no blocker for a row of the Approved
//! version. The `recipe-app` tests of `DiffVersions` and `GetApprovalPreview` cover the use cases.

use axum::http::StatusCode;
use recipe_domain::recipe::{RecipeVersionId, StoredLimit, StoredParameter};

use super::test_world::R500_DIFF;
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::{assert_error, assert_htmx_error};
use crate::test_support::Reply;

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
