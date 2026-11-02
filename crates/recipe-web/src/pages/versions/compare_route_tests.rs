//! Tests of the compare route (task 4.3, ADR-0108 (e), design D3).
//!
//! Spec "Compare route". The route reads no data, so the tests need no recipe in the store. The
//! world is the approval world only because it holds a router and the sessions of its users.

use axum::http::StatusCode;

use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::test_support::plant::HTMX;
use crate::test_support::{Reply, get_page, get_with};

/// The address of the compare route of `R-100` with the query `query`.
fn route(query: &str) -> String {
    format!("/recipes/R-100/compare?{query}")
}

/// Assert a `303` with the location `location` and an empty body.
fn assert_redirect(reply: &Reply, location: &str) {
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), location);
    assert!(reply.body.is_empty(), "{}", reply.body);
}

/// Assert `422 E-WEB-008` and no `Location`.
fn assert_bad_query(reply: &Reply, what: &str) {
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{what}");
    assert!(reply.body.contains("E-WEB-008"), "{what}: {}", reply.body);
    assert_eq!(reply.header("location"), "", "{what}");
}

/// Send `GET path` as the Operator in English.
async fn as_operator(world: &AuditWorld, path: &str) -> Reply {
    world.get_en(&world.operator, path).await
}

#[tokio::test]
async fn compare_route_redirects_two_version_numbers_to_the_diff_of_the_higher_side() {
    let world = audit_world().await;

    let reply = as_operator(&world, &route("n=3&m=1")).await;

    assert_redirect(&reply, "/recipes/R-100/versions/3/diff/1");
}

#[tokio::test]
async fn compare_route_puts_the_higher_number_first_when_the_lower_comes_first() {
    let world = audit_world().await;

    let reply = as_operator(&world, &route("n=1&m=3")).await;

    assert_redirect(&reply, "/recipes/R-100/versions/3/diff/1");
}

#[tokio::test]
async fn compare_route_reads_the_keys_in_any_order() {
    let world = audit_world().await;

    let reply = as_operator(&world, &route("m=2&n=10")).await;

    assert_redirect(&reply, "/recipes/R-100/versions/10/diff/2");
}

#[tokio::test]
async fn compare_route_answers_422_for_a_value_that_is_not_a_version_number() {
    let world = audit_world().await;

    for query in [
        "n=3&m=x",
        "n=x&m=3",
        "n=3&m=",
        "n=3&m=0",
        "n=3&m=-1",
        "n=3&m=%2B1",
        "n=3&m=01",
        "n=3&m=1.5",
        "n=3&m=4294967296",
        "n=3&m=%2F..%2Fadmin",
    ] {
        let reply = as_operator(&world, &route(query)).await;

        assert_bad_query(&reply, query);
    }
}

#[tokio::test]
async fn compare_route_answers_422_for_a_missing_key() {
    let world = audit_world().await;

    for query in ["", "n=3", "m=1"] {
        let reply = as_operator(&world, &route(query)).await;

        assert_bad_query(&reply, query);
    }
}

#[tokio::test]
async fn compare_route_answers_422_for_an_unknown_key() {
    let world = audit_world().await;

    for query in ["n=3&m=1&kind=limits", "n=3&m=1&page=2", "n=3&m=1&n=2"] {
        let reply = as_operator(&world, &route(query)).await;

        assert_bad_query(&reply, query);
    }
}

/// Send `GET path` as the Operator in the language `code`.
async fn as_operator_in(world: &AuditWorld, code: &str, path: &str) -> Reply {
    world.get(&world.operator, code, path).await
}

/// Assert the exact sentence of `E-WEB-008` in the body of a `422` reply.
fn assert_sentence(reply: &Reply, sentence: &str) {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains(sentence), "{sentence}: {}", reply.body);
}

#[tokio::test]
async fn compare_route_names_the_field_and_the_version_reason_for_a_bad_n_in_english() {
    let world = audit_world().await;

    let reply = as_operator_in(&world, "en", &route("n=abc&m=1")).await;

    assert_sentence(
        &reply,
        "Field Version is not valid: choose a version from the list.",
    );
}

#[tokio::test]
async fn compare_route_names_the_field_and_the_version_reason_for_a_bad_n_in_polish() {
    let world = audit_world().await;

    let reply = as_operator_in(&world, "pl", &route("n=abc&m=1")).await;

    assert_sentence(
        &reply,
        "Pole Wersja jest niepoprawne: wybierz wersję z listy.",
    );
}

#[tokio::test]
async fn compare_route_names_the_field_and_the_version_reason_for_a_missing_m_in_english() {
    let world = audit_world().await;

    let reply = as_operator_in(&world, "en", &route("n=3")).await;

    assert_sentence(
        &reply,
        "Field Compare with version is not valid: choose a version from the list.",
    );
}

#[tokio::test]
async fn compare_route_names_the_field_and_the_version_reason_for_a_missing_m_in_polish() {
    let world = audit_world().await;

    let reply = as_operator_in(&world, "pl", &route("n=3")).await;

    assert_sentence(
        &reply,
        "Pole Porównaj z wersją jest niepoprawne: wybierz wersję z listy.",
    );
}

#[tokio::test]
async fn compare_route_keeps_the_query_reason_for_an_unknown_key() {
    let world = audit_world().await;

    let reply = as_operator_in(&world, "en", &route("n=3&m=1&kind=limits")).await;

    assert_sentence(
        &reply,
        "Field Search is not valid: the address has an unknown parameter or a parameter that is not text.",
    );
}

#[tokio::test]
async fn compare_route_reads_no_recipe_so_an_unknown_recipe_still_redirects() {
    let world = audit_world().await;

    let reply = as_operator(&world, "/recipes/R-999/compare?n=2&m=1").await;

    assert_redirect(&reply, "/recipes/R-999/versions/2/diff/1");
}

#[tokio::test]
async fn compare_route_answers_404_for_a_malformed_reference_number() {
    let world = audit_world().await;

    let reply = as_operator(&world, "/recipes/-R1/compare?n=2&m=1").await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-005"), "{}", reply.body);
}

#[tokio::test]
async fn compare_route_answers_an_htmx_request_with_the_same_303() {
    let world = audit_world().await;
    let cookie = format!("{}; recipe_lang=en", world.operator.cookie);

    let reply = get_with(
        &world.fixture.router(),
        &route("n=1&m=3"),
        &[("cookie", cookie.as_str()), HTMX],
    )
    .await;

    assert_redirect(&reply, "/recipes/R-100/versions/3/diff/1");
    assert_eq!(reply.header("hx-redirect"), "");
}

#[tokio::test]
async fn compare_route_without_a_session_redirects_to_the_login() {
    let world = audit_world().await;

    let plain = get_page(&world.fixture, None, "en", &route("n=1&m=3")).await;
    let cookie = "recipe_lang=en";
    let htmx = get_with(
        &world.fixture.router(),
        &route("n=1&m=3"),
        &[("cookie", cookie), HTMX],
    )
    .await;

    assert_redirect(&plain, "/login");
    assert_eq!(htmx.status, StatusCode::UNAUTHORIZED, "{}", htmx.body);
    assert_eq!(htmx.header("hx-redirect"), "/login");
}

#[tokio::test]
async fn compare_route_without_a_session_gives_the_login_before_the_query_error() {
    let world = audit_world().await;

    let reply = get_page(&world.fixture, None, "en", &route("n=x")).await;

    assert_redirect(&reply, "/login");
}

#[tokio::test]
async fn compare_route_redirect_ends_at_the_diff_page_for_an_existing_pair() {
    let world = audit_world().await;

    let reply = world
        .get_en(&world.data.admin, "/recipes/R-500/compare?n=1&m=2")
        .await;
    let location = reply.header("location");
    let diff = world.get_en(&world.data.admin, &location).await;

    assert_eq!(location, "/recipes/R-500/versions/2/diff/1");
    assert_eq!(diff.status, StatusCode::OK, "{}", diff.body);
}
