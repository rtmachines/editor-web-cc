//! Tests of the shared parts of the matrix (design D12, task 4.3).
//!
//! The tests check the placeholders, the two new answers `Redirect` and `Status`, and the plant data
//! that `world` seeds. The rows of the matrix themselves live in `routes::tests`.

use axum::http::{HeaderMap, Method, StatusCode};
use recipe_domain::plant::MachineId;

use super::*;
use crate::test_support::plant::{M7_ID, M12_ID};

/// Build a reply with a status, an optional `Location`, and a body.
fn reply(status: StatusCode, location: Option<&str>, body: &str) -> Reply {
    let mut headers = HeaderMap::new();
    if let Some(location) = location {
        headers.insert("location", location.parse().unwrap());
    }
    Reply {
        status,
        headers,
        body: body.to_owned(),
    }
}

#[test]
fn matrix_fill_replaces_each_placeholder_of_a_path() {
    assert_eq!(fill("/users/{login}"), "/users/user2");
    assert_eq!(fill("/machines/{code}/types"), "/machines/M7/types");
    assert_eq!(
        fill("/machines/{code}/units/{path}"),
        "/machines/M7/units/U1.2"
    );
    assert_eq!(
        fill("/machines/{code}/types/{name}/usage"),
        "/machines/M7/types/ST_Zone/usage"
    );
    assert_eq!(
        fill("/machines/by-id/{uuid}"),
        format!("/machines/by-id/{M12_ID}")
    );
}

#[test]
fn matrix_fill_leaves_a_text_without_a_placeholder() {
    assert_eq!(fill("/recipes"), "/recipes");
    assert_eq!(fill(""), "");
    assert_eq!(fill("lang=en&next=/me"), "lang=en&next=/me");
}

#[test]
fn matrix_fill_replaces_a_placeholder_that_occurs_twice() {
    assert_eq!(fill("{code}/{code}"), "M7/M7");
}

#[test]
fn matrix_redirect_want_accepts_a_303_to_the_path() {
    let found = reply(StatusCode::SEE_OTHER, Some("/machines/M7"), "");
    assert_want(&found, Want::Redirect("/machines/M7"), "redirect");
}

#[test]
#[should_panic(expected = "wrong place")]
fn matrix_redirect_want_rejects_another_location() {
    let found = reply(StatusCode::SEE_OTHER, Some("/login"), "");
    assert_want(&found, Want::Redirect("/machines/M7"), "wrong place");
}

#[test]
#[should_panic(expected = "wrong status")]
fn matrix_redirect_want_rejects_a_status_that_is_not_303() {
    let found = reply(StatusCode::OK, None, "");
    assert_want(&found, Want::Redirect("/machines/M7"), "wrong status");
}

#[test]
fn matrix_status_want_accepts_the_status_with_the_code_in_the_body() {
    let found = reply(StatusCode::CONFLICT, None, "<p>E-PLT-001</p>");
    assert_want(
        &found,
        Want::Status(StatusCode::CONFLICT, "E-PLT-001"),
        "status",
    );
}

#[test]
#[should_panic(expected = "no code")]
fn matrix_status_want_rejects_a_body_without_the_code() {
    let found = reply(StatusCode::CONFLICT, None, "<p>E-PLT-002</p>");
    assert_want(
        &found,
        Want::Status(StatusCode::CONFLICT, "E-PLT-001"),
        "no code",
    );
}

#[test]
#[should_panic(expected = "other status")]
fn matrix_status_want_rejects_another_status() {
    let found = reply(StatusCode::UNPROCESSABLE_ENTITY, None, "E-PLT-001");
    assert_want(
        &found,
        Want::Status(StatusCode::CONFLICT, "E-PLT-001"),
        "other status",
    );
}

#[test]
fn matrix_the_five_old_answers_keep_their_meaning() {
    let page = reply(StatusCode::OK, None, "");
    assert_want(&page, Want::Page, "page");
    let login = reply(StatusCode::SEE_OTHER, Some("/login"), "");
    assert_want(&login, Want::Login, "login");
    let password = reply(StatusCode::SEE_OTHER, Some("/me/password"), "");
    assert_want(&password, Want::Password, "password");
    let recipes = reply(StatusCode::SEE_OTHER, Some("/recipes"), "");
    assert_want(&recipes, Want::Recipes, "recipes");
    let forbidden = reply(StatusCode::FORBIDDEN, None, "E-WEB-002");
    assert_want(&forbidden, Want::Forbidden, "forbidden");
}

#[tokio::test]
async fn matrix_world_seeds_the_plant_data_for_each_principal() {
    for principal in PRINCIPALS {
        let (fixture, _session) = world(principal).await;
        let m7 = fixture.store.state().machine(MachineId::from(M7_ID));
        assert!(m7.is_some(), "{principal:?}");
        let m12 = fixture.store.state().machine(MachineId::from(M12_ID));
        assert!(
            m12.is_some_and(|machine| !machine.is_live()),
            "{principal:?}"
        );
    }
}

#[tokio::test]
async fn matrix_answer_fills_the_placeholders_of_the_body() {
    let lang = endpoint(Method::POST, "/lang", "lang=en&next=/machines/{code}");
    let found = answer(&lang, Principal::NoSession).await;
    assert_want(&found, Want::Redirect("/machines/M7"), "lang next");
}
