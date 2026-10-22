//! Tests of two sessions on one draft: two approvers, and a draft edit between the preview and the
//! approval (task 10.4, ADR-0008, ADR-0049 (b), spec "Errors of the approval").

use axum::http::StatusCode;

use super::approve_tests::{R500_APPROVE, approvals, approve, assert_refusal};
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::field_value;
use crate::test_support::TestSession;
use crate::test_support::plant::main_of;

/// Read the preview of `R-500` version 2 as `session`. Return the row version of its form.
async fn preview_row_version(world: &AuditWorld, session: &TestSession) -> String {
    let reply = world.get_en(session, R500_APPROVE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    field_value(main_of(&reply.body), "row_version")
}

#[tokio::test]
async fn approve_in_a_second_session_with_the_same_row_version_answers_already_approved() {
    let world = audit_world().await;
    let first = preview_row_version(&world, &world.data.admin).await;
    let second = preview_row_version(&world, &world.data.jsmith).await;
    assert_eq!(first, second);
    let before = approvals(&world).len();
    let body = format!("row_version={first}");
    let reply = approve(&world, &world.data.admin, R500_APPROVE, &body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = format!("row_version={second}");
    let reply = approve(&world, &world.data.jsmith, R500_APPROVE, &body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("Version 2 is already approved."),
        "{}",
        reply.body
    );
    assert_eq!(approvals(&world).len(), before.checked_add(1).unwrap());
}

#[tokio::test]
async fn approve_after_a_draft_edit_in_another_session_answers_the_conflict() {
    let world = audit_world().await;
    let seen = preview_row_version(&world, &world.data.admin).await;
    let anowak = world.data.anowak_user();
    let current = world
        .fixture
        .add_zone_to_r500(&anowak, seen.parse().unwrap())
        .await;
    let before = approvals(&world).len();
    let body = format!("row_version={seen}&allow_outdated=true");
    let reply = approve(&world, &world.data.admin, R500_APPROVE, &body).await;
    assert_refusal(&reply, StatusCode::CONFLICT, "E-WEB-001");
    // The text `conflict-row-version` names the number of the server, not the sent number.
    assert_ne!(current.to_string(), seen);
    let line = format!("<li>Current change number: {current}</li>");
    assert!(reply.body.contains(&line), "{}", reply.body);
    assert!(
        reply.body.contains("href=\"/recipes/R-500/versions/2\""),
        "{}",
        reply.body
    );
    assert_eq!(approvals(&world).len(), before);
    let page = world.get_en(&world.data.admin, R500_APPROVE).await;
    assert_eq!(
        page.status,
        StatusCode::OK,
        "the draft stays: {}",
        page.body
    );
    assert!(page.body.contains("Approve version 2"), "{}", page.body);
}
