//! Tests of the approval preview: the blockers, the review state `rv`, and the error answers
//! (task 10.1, ADR-0049 (c), (d), ADR-0050 (h), (j), spec "Blockers on the approval preview",
//! "Review state of a paged preview", "Pages of a deleted recipe").

use axum::http::StatusCode;
use recipe_app::ports::{Isolation, Transaction as _};
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcTypeId;
use recipe_domain::recipe::StoredParameter;
use uuid::Uuid;

use super::preview_tests::part_of;
use crate::pages::audit::test_world::audit_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error, field_value};
use crate::test_support::plant::main_of;
use crate::test_support::recipes::HOLD_KEY;

/// The preview of the draft version 3 of `R-100`, which `user1` created.
const R100_PREVIEW: &str = "/recipes/R-100/versions/3/approve";
/// The preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";

/// Return the notice of the blockers in `main`.
fn blockers(main: &str) -> &str {
    part_of(main, "<h2 id=\"approve-blockers\">", "</ul>")
}

#[tokio::test]
async fn approve_preview_lists_a_missing_limit_with_a_link_to_its_parameter() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.jsmith, R100_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    let notice = blockers(main);
    assert!(notice.contains("The approval is not possible"), "{notice}");
    let href = format!("href=\"/recipes/R-100/versions/3/parameters/{HOLD_KEY}\"");
    let line = part_of(notice, &href, "</li>");
    assert!(line.contains("E-RCP-016"), "{line}");
    assert!(!main.contains("E-RCP-015"), "{main}");
    assert!(!main.contains("role=\"alert\""), "{main}");
    assert!(!main.contains("<form method=\"post\""), "{main}");
}

/// The review of `R-100` version 3 does not fail, so the diff shows next to the blockers (design D3).
#[tokio::test]
async fn approve_preview_with_a_draft_blocker_shows_the_diff_after_it_and_no_link_to_the_form() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.jsmith, R100_PREVIEW).await;
    let main = main_of(&reply.body);
    assert!(
        main.contains("<h1>Approval of version 3 of recipe R-100</h1>"),
        "{main}"
    );
    let notice = main.find("<h2 id=\"approve-blockers\">").unwrap();
    let summary = main.find("<ul class=\"diff-summary\">").unwrap();
    assert!(notice < summary, "{main}");
    assert!(!main.contains("Go to the approval form"), "{main}");
}

/// A draft type that does not resolve fails the review, so the page has no diff (design D3).
#[tokio::test]
async fn approve_preview_without_a_diff_shows_the_header_and_the_blockers_only() {
    let world = audit_world().await;
    let version = world.data.r500_v2;
    let mut tx = world
        .fixture
        .store
        .begin_memory(Isolation::ReadOnly)
        .unwrap();
    let stored = tx.recipes().version_parameters(version).await.unwrap();
    drop(tx);
    let broken = StoredParameter {
        plc_type: PlcTypeId::from(Uuid::from_u128(0xdead)),
        ..stored.first().unwrap().clone()
    };
    world.fixture.store.seed_parameter(version, &broken);
    let reply = world.get_en(&world.data.admin, R500_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("<h1>Approval of version 2 of recipe R-500</h1>"),
        "{main}"
    );
    assert!(blockers(main).contains("E-RCP-013"), "{main}");
    for absent in [
        "diff-summary",
        "diff-section",
        "approve-form",
        "retires version 1",
        "Outdated parameters",
        "Go to the approval form",
    ] {
        assert!(!main.contains(absent), "{absent}: {main}");
    }
}

#[tokio::test]
async fn approve_preview_of_the_creator_shows_the_separation_of_duties_with_the_other_blockers() {
    let world = audit_world().await;
    let creator = world.fixture.seed_session(1, Role::Administrator, false);
    let reply = world.get_en(&creator, R100_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(blockers(main).contains("E-RCP-016"), "{main}");
    let notice = part_of(
        main,
        "<span class=\"alert-code\">E-RCP-015</span>",
        "</div>",
    );
    assert!(
        notice.contains("Another user with the permission must approve this draft."),
        "{notice}"
    );
    assert!(!main.contains("<form method=\"post\""), "{main}");
}

#[tokio::test]
async fn approve_preview_of_an_editor_puts_the_blocker_before_the_summary() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.anowak, R500_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let code = main.find("E-RCP-015").unwrap();
    let summary = main.find("<ul class=\"diff-summary\">").unwrap();
    assert!(code < summary, "{main}");
    assert!(!main.contains("role=\"alert\""), "{main}");
    assert!(!main.contains("<form method=\"post\""), "{main}");
    assert!(!main.contains("Go to the approval form"), "{main}");
}

#[tokio::test]
async fn approve_preview_with_another_review_state_shows_the_notice_and_no_form() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.data.admin, &format!("{R500_PREVIEW}?rv=1"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("The draft changed during the review. Start the review again."),
        "{main}"
    );
    let link = part_of(main, &format!("href=\"{R500_PREVIEW}\""), "</a>");
    assert!(link.contains("Start the review again"), "{link}");
    assert!(!main.contains("<form method=\"post\""), "{main}");
}

#[tokio::test]
async fn approve_preview_with_the_same_review_state_keeps_the_form() {
    let world = audit_world().await;
    let rv = world.data.r500_v2_row_version;
    let reply = world
        .get_en(&world.data.admin, &format!("{R500_PREVIEW}?rv={rv}"))
        .await;
    let main = main_of(&reply.body);
    assert!(
        !main.contains("The draft changed during the review."),
        "{main}"
    );
    assert_eq!(field_value(main, "row_version"), rv.to_string(), "{main}");
}

#[tokio::test]
async fn approve_preview_refuses_a_bad_kind_page_or_review_state() {
    let world = audit_world().await;
    for query in [
        "kind=other",
        "page=0",
        "page=x",
        "rv=abc",
        "rv=0",
        "rv=-3",
        "other=1",
    ] {
        let path = format!("{R500_PREVIEW}?{query}");
        let reply = world.get_en(&world.data.admin, &path).await;
        assert_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", query);
    }
}

#[tokio::test]
async fn approve_preview_answers_the_codes_of_the_version_and_the_recipe() {
    let world = audit_world().await;
    let admin = &world.data.admin;
    let cases = [
        (
            "/recipes/R-500/versions/1/approve",
            StatusCode::CONFLICT,
            "E-RCP-007",
        ),
        (
            "/recipes/R-700/versions/1/approve",
            StatusCode::CONFLICT,
            "E-RCP-012",
        ),
        (
            "/recipes/R-999/versions/1/approve",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/recipes/R-500/versions/9/approve",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/recipes/R-500/versions/x/approve",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
    ];
    for (path, status, code) in cases {
        let reply = world.get_en(admin, path).await;
        assert_error(&reply, status, code, path);
    }
    let reply = world
        .get_en(&world.data.anowak, "/recipes/R-700/versions/1/approve")
        .await;
    assert_error(&reply, StatusCode::CONFLICT, "E-RCP-012", "engineer");
}
