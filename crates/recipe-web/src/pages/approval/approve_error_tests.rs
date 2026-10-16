//! Tests of the error answers of `POST /recipes/{ref}/versions/{n}/approve`: one HTMX test for
//! each code of design D9 that the test data can cause (task 10.2, ADR-0047 (b), spec "Errors of
//! the approval").

use axum::http::StatusCode;
use recipe_domain::recipe::VersionState;

use super::approve_tests::{R500_APPROVE, approvals, approve, assert_refusal};
use crate::pages::audit::test_world::audit_world;
use crate::pages::recipes::test_world::{seed_parameter_on_unknown_unit, seed_recipe_of_m20};

#[tokio::test]
async fn approve_answers_the_codes_of_the_recipe_and_the_version() {
    let world = audit_world().await;
    let cases = [
        (
            "/recipes/R-999/versions/2/approve",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/recipes/R-600/versions/1/approve",
            StatusCode::CONFLICT,
            "E-RCP-007",
        ),
        (
            "/recipes/R-700/versions/1/approve",
            StatusCode::CONFLICT,
            "E-RCP-012",
        ),
    ];
    for (path, status, code) in cases {
        let reply = approve(&world, &world.data.admin, path, "row_version=1").await;
        assert_refusal(&reply, status, code);
    }
}

#[tokio::test]
async fn approve_of_the_creator_answers_the_separation_of_duties_and_writes_nothing() {
    let world = audit_world().await;
    let entries = world.fixture.store.audit_entries().len();
    let body = format!("row_version={}", world.data.r500_v2_row_version);
    let reply = approve(&world, &world.data.anowak, R500_APPROVE, &body).await;
    assert_refusal(&reply, StatusCode::CONFLICT, "E-RCP-015");
    assert_eq!(world.fixture.store.audit_entries().len(), entries);
}

#[tokio::test]
async fn approve_of_an_editor_of_the_draft_answers_the_separation_of_duties() {
    let world = audit_world().await;
    let jsmith = world.data.jsmith_user();
    let rv = world
        .fixture
        .add_zone_to_r500(&jsmith, world.data.r500_v2_row_version)
        .await;
    let body = format!("row_version={rv}&allow_outdated=true");
    let reply = approve(&world, &world.data.jsmith, R500_APPROVE, &body).await;
    assert_refusal(&reply, StatusCode::CONFLICT, "E-RCP-015");
}

/// Return the number of the summary lines of `code` in `body`.
fn lines_of(body: &str, code: &str) -> usize {
    body.matches(&format!("<li><span class=\"alert-code\">{code}</span>"))
        .count()
}

#[tokio::test]
async fn approve_lists_each_missing_limit_on_its_own_line() {
    let world = audit_world().await;
    let path = "/recipes/R-100/versions/3/approve";
    let reply = approve(&world, &world.data.jsmith, path, "row_version=1").await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-016");
    let shown = lines_of(&reply.body, "E-RCP-016");
    assert!(shown > 1, "{}", reply.body);
    // The summary shows the first 20 lines and counts the other lines (ADR-0041 (d)).
    let more = reply
        .body
        .split_once("Other errors: ")
        .map_or(0, |(_before, rest)| {
            rest.split('.').next().unwrap().parse::<usize>().unwrap()
        });
    let total = shown.checked_add(more).unwrap();
    assert!(
        reply.body.contains(&format!("Errors: {total}.")),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn approve_lists_each_item_of_a_failed_snapshot_check_on_its_own_line() {
    let world = audit_world().await;
    let ids = seed_recipe_of_m20(&world.fixture, (801, "R-801"), &[VersionState::LiveDraft]);
    let draft = *ids.first().unwrap();
    seed_parameter_on_unknown_unit(&world.fixture, draft, (1, "P_ONE"));
    seed_parameter_on_unknown_unit(&world.fixture, draft, (2, "P_TWO"));
    let path = "/recipes/R-801/versions/1/approve";
    let reply = approve(&world, &world.data.admin, path, "row_version=1").await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-020");
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    for tag in ["P_ONE", "P_TWO"] {
        let line = format!(
            "<span class=\"alert-code\">E-PLT-020</span> <span class=\"data\">{tag}</span>"
        );
        assert!(reply.body.contains(&line), "{tag}: {}", reply.body);
    }
}

#[tokio::test]
async fn approve_of_a_draft_without_parameters_answers_its_code() {
    let world = audit_world().await;
    seed_recipe_of_m20(&world.fixture, (800, "R-800"), &[VersionState::LiveDraft]);
    let count = approvals(&world).len();
    let path = "/recipes/R-800/versions/1/approve";
    let reply = approve(&world, &world.data.admin, path, "row_version=1").await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-008");
    assert_eq!(approvals(&world).len(), count);
}
