//! Tests of the field `next` of the language switch after a content answer that pushes the address
//! of the version page (design D5 and D7).
//!
//! The approval, the retire, and the removal of a parameter answer a `POST` with the content of the
//! version page and `HX-Push-Url`. The answer sets `#lang-next` out of band to the pushed path, so a
//! language switch returns to the page that the address bar shows. A full page that answers a
//! `POST` keeps no switch. `R-500` has the approved version 1 and the draft version 2.

use axum::http::StatusCode;

use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::plant::{ENGLISH, HTMX, HTMX_CONTENT, TARGET_CONTENT};
use crate::test_support::recipes::SPEED_KEY;
use crate::test_support::{Reply, TestSession, post_fields};

/// The out-of-band input of the switch for the version page 2 of `R-500`.
const R500_V2_NEXT: &str = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/recipes/R-500/versions/2\" hx-swap-oob=\"true\">";
/// The out-of-band input of the switch for the version page 1 of `R-500`.
const R500_V1_NEXT: &str = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/recipes/R-500/versions/1\" hx-swap-oob=\"true\">";
/// The out-of-band input of the switch for the version page 3 of `R-100`.
const R100_V3_NEXT: &str = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/recipes/R-100/versions/3\" hx-swap-oob=\"true\">";

/// Send the form `body` to `path` as `session` with the headers `headers`.
async fn post(
    world: &AuditWorld,
    session: &TestSession,
    path: &str,
    (headers, body): (&[(&str, &str)], &str),
) -> Reply {
    post_fields(&world.fixture.router(), path, Some(session), headers, body).await
}

/// Assert that `reply` is a content answer with exactly one input `#lang-next` that equals `input`.
fn assert_next(reply: &Reply, input: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("<html"), "{}", reply.body);
    assert!(reply.body.contains(input), "{}", reply.body);
    assert_eq!(
        reply.body.matches("id=\"lang-next\"").count(),
        1,
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn lang_next_approval_for_the_content_sets_the_switch_to_the_version_page() {
    let world = audit_world().await;
    let body = format!("row_version={}", world.data.r500_v2_row_version);
    let headers = [ENGLISH, HTMX, TARGET_CONTENT];
    let path = "/recipes/R-500/versions/2/approve";
    let reply = post(&world, &world.data.admin, path, (&headers, &body)).await;
    assert_next(&reply, R500_V2_NEXT);
    assert_eq!(reply.header("hx-push-url"), "/recipes/R-500/versions/2");
}

#[tokio::test]
async fn lang_next_retire_for_the_content_sets_the_switch_to_the_version_page() {
    let world = audit_world().await;
    let body = format!("row_version={}", world.row_version_of("R-500", 1).await);
    let headers = [ENGLISH, HTMX, TARGET_CONTENT];
    let path = "/recipes/R-500/versions/1/retire";
    let reply = post(&world, &world.data.admin, path, (&headers, &body)).await;
    assert_next(&reply, R500_V1_NEXT);
    assert_eq!(reply.header("hx-push-url"), "/recipes/R-500/versions/1");
}

#[tokio::test]
async fn lang_next_approval_without_htmx_answers_a_full_page_without_a_switch_input() {
    let world = audit_world().await;
    let body = format!("row_version={}", world.data.r500_v2_row_version);
    let path = "/recipes/R-500/versions/2/approve";
    let reply = post(&world, &world.data.admin, path, (&[ENGLISH], &body)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("<html"), "{}", reply.body);
    assert!(!reply.body.contains("lang-next"), "{}", reply.body);
    assert!(
        !reply.body.contains("class=\"lang-switch\""),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn lang_next_parameter_removal_for_the_content_sets_the_switch_to_the_version_page() {
    let world = recipe_world();
    let path = format!("/recipes/R-100/versions/3/parameters/{SPEED_KEY}/delete");
    let reply = world
        .post_with(&world.engineer, &path, "row_version=1", &HTMX_CONTENT)
        .await;
    assert_next(&reply, R100_V3_NEXT);
    assert_eq!(reply.header("hx-push-url"), "/recipes/R-100/versions/3");
}

#[tokio::test]
async fn lang_next_get_of_the_version_page_keeps_the_path_and_the_query() {
    let world = audit_world().await;
    let path = "/recipes/R-500/versions/2?page=1";
    let content = world.get_content(&world.data.admin, path).await;
    let kept = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/recipes/R-500/versions/2?page=1\" hx-swap-oob=\"true\">";
    assert_next(&content, kept);
    let page = world.get_en(&world.data.admin, path).await;
    let form_input = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/recipes/R-500/versions/2?page=1\">";
    assert!(page.body.contains(form_input), "{}", page.body);
}
