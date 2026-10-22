//! Tests of the approval preview: the blockers, the review state `rv`, and the error answers
//! (task 10.1, ADR-0049 (c), (d), ADR-0050 (h), (j), spec "Blockers on the approval preview",
//! "Review state of a paged preview", "Pages of a deleted recipe").

use axum::http::StatusCode;
use recipe_app::ports::{Isolation, Transaction as _};
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcTypeId;
use recipe_domain::recipe::{StoredParameter, VersionState};
use uuid::Uuid;

use super::approve_error_tests::seed_unlimited_ints;
use super::preview_tests::{assert_before, part_of};
use crate::pages::audit::test_world::audit_world;
use crate::pages::recipes::test_world::seed_recipe_of_m20;
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error, field_value};
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

/// Assert that `main` has no approve form, no jump link to it, and no `row_version` field.
///
/// The form of the preview starts with `<form id="approve-fields"`. The positive controls are
/// `preview_tests::approve_preview_form_sends_the_row_version_of_the_snapshot_with_the_token` and
/// `preview_tests::approve_preview_first_summary_item_links_to_the_heading_of_the_form`.
fn assert_no_form(main: &str) {
    for absent in [
        "id=\"approve-fields\"",
        "href=\"#approve-form\"",
        "name=\"row_version\"",
    ] {
        assert!(!main.contains(absent), "{absent}: {main}");
    }
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
    assert_no_form(main);
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
    // The Approved version 2 of `R-100` holds no parameter, so each parameter of version 3 is an
    // added parameter.
    let added = "<section class=\"diff-section\" id=\"diff-added\">";
    assert!(main.contains(added), "{main}");
    assert_before(main, "<h2 id=\"approve-blockers\">", added);
    assert_before(main, "<ul class=\"diff-summary\">", added);
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
    assert_no_form(main);
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
    assert_no_form(main);
    assert!(!main.contains("Go to the approval form"), "{main}");
}

#[tokio::test]
async fn approve_preview_with_another_review_state_shows_the_notice_and_no_form() {
    let world = audit_world().await;
    // The first page and a later page (a section page) of the paged preview (design D10).
    for query in ["rv=1", "kind=limits&rv=1"] {
        let path = format!("{R500_PREVIEW}?{query}");
        let reply = world.get_en(&world.data.admin, &path).await;
        assert_eq!(reply.status, StatusCode::OK, "{query}: {}", reply.body);
        let main = main_of(&reply.body);
        assert!(
            main.contains("The draft changed during the review. Start the review again."),
            "{query}: {main}"
        );
        let link = part_of(main, &format!("href=\"{R500_PREVIEW}\""), "</a>");
        assert!(link.contains("Start the review again"), "{query}: {link}");
        assert_no_form(main);
    }
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
    // The label of the one field line of each answer (design D3). The key `other` is not a field of
    // the page, so `query_values` names the whole query string with the label `field-query`.
    let cases = [
        ("kind=other", "Kind"),
        ("page=0", "Page"),
        ("page=x", "Page"),
        ("rv=abc", "Change number of the review"),
        ("rv=0", "Change number of the review"),
        ("rv=-3", "Change number of the review"),
        ("other=1", "Search"),
    ];
    for (query, label) in cases {
        let path = format!("{R500_PREVIEW}?{query}");
        let reply = world.get_en(&world.data.admin, &path).await;
        assert_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", query);
        let line = format!("<ul class=\"field-lines\"><li>{label}</li></ul>");
        assert!(reply.body.contains(&line), "{query}: {}", reply.body);
    }
}

#[tokio::test]
async fn approve_preview_htmx_bad_kind_line_has_no_link_because_the_page_has_no_kind_input() {
    let world = audit_world().await;
    let path = format!("{R500_PREVIEW}?kind=bogus");
    let reply = world.get_content(&world.data.admin, &path).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(
        reply
            .body
            .contains("<ul class=\"field-lines\"><li>Kind</li></ul>"),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("the address has an unknown parameter or a parameter that is not text"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("href=\"#kind\""), "{}", reply.body);
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

/// The number of the blocker lines that the notice shows (ADR-0049 (c), ADR-0041 (d)).
const SHOWN_LINES: usize = 20;
/// The code of a parameter without a limit.
const LIMIT_MISSING: &str = "<span class=\"alert-code\">E-RCP-016</span>";
/// The sentence `errors-more` for five other lines, in the language `code`.
fn five_more(code: &str) -> &'static str {
    match code {
        "pl" => "<li>Pozostałe błędy: 5. Każdy stoi pod swoim polem.</li>",
        _en => "<li>Other errors: 5. Each one stands under its field.</li>",
    }
}

/// Return the page of the preview of the draft `R-802` with `count` parameters without a limit.
///
/// The draft `R-802` has one version. The page is in the language `code`.
async fn limitless_preview(count: u32, code: &str) -> String {
    let world = audit_world().await;
    let ids = seed_recipe_of_m20(&world.fixture, (802, "R-802"), &[VersionState::LiveDraft]);
    seed_unlimited_ints(&world.fixture, *ids.first().unwrap(), count);
    let path = "/recipes/R-802/versions/1/approve";
    let reply = world.get(&world.data.jsmith, code, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body
}

#[tokio::test]
async fn approve_preview_caps_the_blocker_lines_at_20_and_counts_the_other_five() {
    for code in ["en", "pl"] {
        let body = limitless_preview(25, code).await;
        let main = main_of(&body);
        let notice = blockers(main);
        assert_eq!(
            notice.matches(LIMIT_MISSING).count(),
            SHOWN_LINES,
            "{code}: {notice}"
        );
        assert_eq!(
            notice.matches(five_more(code)).count(),
            1,
            "{code}: {notice}"
        );
        assert!(notice.contains(">P20 "), "{code}: {notice}");
        assert!(!notice.contains(">P21 "), "{code}: {notice}");
    }
}

#[tokio::test]
async fn approve_preview_with_exactly_20_blocker_lines_shows_no_sentence_about_the_others() {
    for (code, more) in [("en", "Other errors"), ("pl", "Pozostałe błędy")] {
        let body = limitless_preview(20, code).await;
        let notice = blockers(main_of(&body));
        assert_eq!(
            notice.matches(LIMIT_MISSING).count(),
            SHOWN_LINES,
            "{code}: {notice}"
        );
        assert!(!notice.contains(more), "{code}: {notice}");
    }
}
