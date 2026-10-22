//! Tests of the error answers of the diff page: one test for each code of design D9 (task 12.1,
//! `api.md` section 2.5, spec "Diff page", "Unknown kind").

use axum::http::StatusCode;
use recipe_app::blocking::{AppSettings, Blocking};
use recipe_domain::identity::Role;

use super::test_world::R500_DIFF;
use crate::pages::audit::test_world::audit_world;
use crate::pages::test_helpers::{assert_error, assert_htmx_error};
use crate::test_support::{Fixture, get_page};

#[tokio::test]
async fn diff_page_refuses_a_bad_kind_or_page_or_key() {
    let world = audit_world().await;
    for query in [
        "kind=other",
        "kind=",
        "page=0",
        "page=abc",
        "page=-1",
        "rv=1",
        "kind=limits&kind=values",
    ] {
        let path = format!("{R500_DIFF}?{query}");
        let reply = world.get_en(&world.data.admin, &path).await;
        assert_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", query);
    }
}

#[tokio::test]
async fn diff_page_htmx_bad_kind_line_has_no_link_because_the_page_has_no_kind_input() {
    let world = audit_world().await;
    let path = format!("{R500_DIFF}?kind=bogus");
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
async fn diff_page_answers_not_found_for_an_unknown_recipe_or_version() {
    let world = audit_world().await;
    for path in [
        "/recipes/R-999/versions/2/diff/1",
        "/recipes/R-500/versions/9/diff/1",
        "/recipes/R-500/versions/2/diff/9",
        "/recipes/R-500/versions/2/diff/x",
        "/recipes/R-500/versions/x/diff/1",
        "/recipes/R-500/versions/2/diff/0",
    ] {
        let reply = world.get_en(&world.data.admin, path).await;
        assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", path);
    }
}

#[tokio::test]
async fn diff_page_of_a_deleted_recipe_gives_conflict_to_a_role_that_cannot_restore_it() {
    let world = audit_world().await;
    let path = "/recipes/R-700/versions/1/diff/1";
    for session in [&world.data.anowak, &world.operator] {
        let reply = world.get_en(session, path).await;
        assert_error(&reply, StatusCode::CONFLICT, "E-RCP-012", path);
    }
}

#[tokio::test]
async fn diff_page_answers_the_failures_of_the_diff_stage() {
    let cases = [
        (
            Blocking::elapsed(),
            StatusCode::SERVICE_UNAVAILABLE,
            "E-WEB-009",
        ),
        (
            Blocking::panicked(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "E-WEB-011",
        ),
    ];
    for (blocking, status, code) in cases {
        let fixture = Fixture::with_app_settings(AppSettings {
            blocking,
            ..AppSettings::default()
        });
        fixture.seed_recipes();
        let engineer = fixture.seed_session(2, Role::ProcessEngineer, false);
        let path = "/recipes/R-100/versions/3/diff/3";
        let reply = get_page(&fixture, Some(&engineer), "en", path).await;
        assert_error(&reply, status, code, code);
    }
}
