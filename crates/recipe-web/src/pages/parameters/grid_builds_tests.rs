//! Tests of the number of grids that a save of a parameter form builds (ADR-0223 (p), task 4.5).
//!
//! `ready` builds no grid for a valid save. `failed` builds the sent grid only after the command
//! refused the save. The count of `GridView::build` is thread-local. A route test runs the router
//! on its own thread, so each test resets the count before the request.
use axum::extract::FromRequestParts as _;
use axum::http::{Method, Request, StatusCode};
use recipe_app::error::{AppError, StorageError};
use recipe_domain::identity::SessionToken;
use recipe_domain::recipe::{ParameterKey, ReferenceNumber};

use super::definition::check;
use super::fields::{FieldSet, read_body};
use super::grid::build_count::{builds, reset};
use super::reply::{Ctx, Target, failed, load_shown};
use super::test_kit::{
    ADD, INT_FIELDS, SPEED_FIELDS, assert_panel_error, changed, form, parameter_path, post_htmx,
};
use crate::client::ClientAddr;
use crate::pages::PageRequest;
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::plant::HTMX;
use crate::test_support::recipes::SPEED_KEY;
use crate::test_support::{PEER, plant::World};

#[tokio::test]
async fn grid_builds_zero_for_a_valid_edit_that_answers_303() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    reset();
    let reply = world
        .post(&world.engineer, &path, &form(&SPEED_FIELDS), false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(builds(), 0);
}

#[tokio::test]
async fn grid_builds_zero_for_a_valid_add_that_answers_303() {
    let world = recipe_world();
    reset();
    let reply = world
        .post(&world.engineer, ADD, &form(&INT_FIELDS), false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(builds(), 0);
}

#[tokio::test]
async fn grid_builds_one_for_the_htmx_success_of_the_edit() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    reset();
    let reply = post_htmx(&world, &path, &SPEED_FIELDS).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(builds(), 1, "only the clean form of the answer has a grid");
}

#[tokio::test]
async fn grid_builds_zero_for_a_409_conflict() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let first = post_htmx(&world, &path, &SPEED_FIELDS).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    reset();
    let reply = world
        .post_with(&world.admin, &path, &form(&SPEED_FIELDS), &[HTMX])
        .await;
    assert_panel_error(&reply, StatusCode::CONFLICT, "E-WEB-001");
    assert_eq!(builds(), 0);
}

#[tokio::test]
async fn grid_builds_one_for_a_value_that_the_command_refuses() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let pairs = changed(&SPEED_FIELDS, &[("value", Some("5000"))]);
    reset();
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-004");
    let line = "<div class=\"field-error\" id=\"value-0-error\">";
    assert!(reply.body.contains(line), "{}", reply.body);
    assert_eq!(builds(), 1);
}

#[tokio::test]
async fn grid_builds_one_for_a_field_that_the_checks_refuse() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let pairs = changed(&SPEED_FIELDS, &[("tag", Some("bad tag"))]);
    reset();
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(reply.body.contains("id=\"tag-error\""), "{}", reply.body);
    assert_eq!(builds(), 1);
}

#[tokio::test]
async fn grid_builds_one_for_a_bad_maximum_length_in_the_summary() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let pairs = changed(&SPEED_FIELDS, &[("limits[0].max_length", Some("abc"))]);
    reset();
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    let link = "href=\"#limits-0-max_length\"";
    assert!(reply.body.contains(link), "{}", reply.body);
    let line = "id=\"limits-0-max_length-error\"";
    assert!(!reply.body.contains(line), "{}", reply.body);
    assert_eq!(builds(), 1);
}

/// The number of the seeded ProcessEngineer session of the plant world.
const ENGINEER_SESSION: u8 = 2;

/// Build the context data of the parameter `SPEED` of the draft 3 of `R-100` for `world`.
async fn speed_request(world: &World) -> (PageRequest, Target) {
    let state = &world.fixture.state;
    let token = SessionToken::from_bytes([ENGINEER_SESSION; 16]);
    let auth = recipe_app::identity::authenticate(state.identity(), &token)
        .await
        .unwrap()
        .unwrap();
    let uri = parameter_path(SPEED_KEY, "/definition");
    let request = Request::builder().method(Method::POST).uri(uri);
    let (mut parts, ()) = request.body(()).unwrap().into_parts();
    parts.extensions.insert(ClientAddr::of(PEER.ip()));
    parts.extensions.insert(Some(auth));
    let page = PageRequest::from_request_parts(&mut parts, state)
        .await
        .map_err(|_rejection| "rejected")
        .unwrap();
    let target = Target {
        reference: ReferenceNumber::parse("R-100").unwrap(),
        number: 3,
        key: Some(ParameterKey::from(SPEED_KEY)),
    };
    (page, target)
}

#[tokio::test]
async fn grid_builds_zero_for_a_storage_fault() {
    let world = recipe_world();
    let (page, target) = speed_request(&world).await;
    let state = &world.fixture.state;
    let ctx = Ctx {
        state,
        request: &page,
        target: &target,
    };
    let sent = read_body(form(&SPEED_FIELDS).as_bytes(), FieldSet::Parameter).unwrap();
    let type_expr = check(&sent).type_expr.unwrap();
    let shown = load_shown(ctx, Some(type_expr), None).await.unwrap();
    let loaded = shown.form;
    assert!(
        loaded.preview.is_some(),
        "the form must have a preview tree"
    );
    let error = AppError::Storage(StorageError::PoolTimeout);
    reset();
    let answer = failed(ctx, (&loaded, &sent, shown.previewed.as_ref()), error).await;
    assert!(matches!(
        answer,
        crate::error::WebError::App(AppError::Storage(StorageError::PoolTimeout))
    ));
    assert_eq!(builds(), 0, "a storage fault gives no form, so no grid");
}
