//! Tests of a deleted recipe and of read-only mode on each parameter route (task 11.5).
//!
//! `api.md` sections 2.4 and 2.6, ADR-0043 (e): every parameter route of a deleted recipe answers
//! `409 E-RCP-012`, except a `GET` route of data for the Administrator. The permission check comes
//! first (ADR-0013), so the Operator gets `403 E-WEB-002` on each route that needs `ParameterEdit`.
//! In read-only mode each command route answers `503 E-WEB-006` and writes nothing (A24).

use axum::http::StatusCode;

use super::test_kit::{ADD, INT_FIELDS, SPEED_FIELDS, form, parameter_path};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_error;
use crate::test_support::TestSession;
use crate::test_support::plant::{ENGLISH, HTMX, World};
use crate::test_support::recipes::{LINE_KEY, SPEED_KEY};

/// The base of the routes of the retired version 1 of the deleted recipe `R-300`.
const DELETED: &str = "/recipes/R-300/versions/1/parameters";

/// The `GET` routes of data of `R-300`.
fn deleted_data_routes() -> Vec<String> {
    let key = LINE_KEY;
    vec![
        DELETED.to_owned(),
        format!("{DELETED}/{key}"),
        format!("{DELETED}/{key}/limits"),
        format!("{DELETED}/{key}/translations"),
    ]
}

/// The `POST` routes of `R-300` with a body that each route accepts.
fn deleted_command_routes() -> Vec<(String, String)> {
    let key = LINE_KEY;
    let row = "row_version=1".to_owned();
    vec![
        (DELETED.to_owned(), form(&INT_FIELDS)),
        (format!("{DELETED}/{key}/definition"), form(&SPEED_FIELDS)),
        (
            format!("{DELETED}/{key}/value"),
            "row_version=1&value=1".to_owned(),
        ),
        (format!("{DELETED}/{key}/limits"), row.clone()),
        (
            format!("{DELETED}/{key}/translations/pl"),
            "row_version=1&name=N&description=".to_owned(),
        ),
        (
            format!("{DELETED}/{key}/translations/pl/delete"),
            row.clone(),
        ),
        (format!("{DELETED}/{key}/delete"), row),
    ]
}

/// Send `GET path` as `session` as an HTMX request in English.
async fn get_htmx(world: &World, session: &TestSession, path: &str) -> crate::test_support::Reply {
    world.get(session, path, &[ENGLISH, HTMX]).await
}

#[tokio::test]
async fn parameters_of_a_deleted_recipe_give_e_rcp_012_to_the_engineer_and_the_operator() {
    let world = recipe_world();
    for path in deleted_data_routes() {
        for session in [&world.engineer, &world.operator] {
            let reply = get_htmx(&world, session, &path).await;
            assert_error(&reply, StatusCode::CONFLICT, "E-RCP-012", &path);
        }
    }
    let form_route = format!("{DELETED}/new");
    for session in [&world.engineer, &world.admin] {
        let reply = get_htmx(&world, session, &form_route).await;
        assert_error(&reply, StatusCode::CONFLICT, "E-RCP-012", &form_route);
    }
    let operator = get_htmx(&world, &world.operator, &form_route).await;
    assert_error(&operator, StatusCode::FORBIDDEN, "E-WEB-002", &form_route);
}

#[tokio::test]
async fn parameters_commands_of_a_deleted_recipe_give_e_rcp_012_and_write_nothing() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    for (path, body) in deleted_command_routes() {
        for session in [&world.engineer, &world.admin] {
            let reply = world.post_with(session, &path, &body, &[HTMX]).await;
            assert_error(&reply, StatusCode::CONFLICT, "E-RCP-012", &path);
        }
        let operator = world
            .post_with(&world.operator, &path, &body, &[HTMX])
            .await;
        assert_error(&operator, StatusCode::FORBIDDEN, "E-WEB-002", &path);
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_table_of_a_deleted_recipe_shows_the_rows_to_the_administrator() {
    let world = recipe_world();
    let reply = get_htmx(&world, &world.admin, DELETED).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

/// The command routes of the draft 3 of `R-100` with a valid body.
fn command_routes() -> Vec<(String, String)> {
    let row = "row_version=1".to_owned();
    vec![
        (ADD.to_owned(), form(&INT_FIELDS)),
        (ADD.to_owned(), format!("{}&fields=1", form(&INT_FIELDS))),
        (
            parameter_path(SPEED_KEY, "/definition"),
            form(&SPEED_FIELDS),
        ),
        (
            parameter_path(SPEED_KEY, "/value"),
            "row_version=1&value=1".to_owned(),
        ),
        (parameter_path(SPEED_KEY, "/limits"), row.clone()),
        (
            parameter_path(SPEED_KEY, "/translations/pl"),
            "row_version=1&name=N&description=".to_owned(),
        ),
        (
            parameter_path(LINE_KEY, "/translations/pl/delete"),
            row.clone(),
        ),
        (parameter_path(SPEED_KEY, "/delete"), row),
    ]
}

#[tokio::test]
async fn parameters_each_command_gives_e_web_006_in_read_only_mode() {
    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    let before = world.fixture.snapshot();
    for (path, body) in command_routes() {
        let reply = world
            .post_with(&world.engineer, &path, &body, &[HTMX])
            .await;
        assert_error(&reply, StatusCode::SERVICE_UNAVAILABLE, "E-WEB-006", &path);
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_each_get_route_works_in_read_only_mode() {
    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    let paths = [
        ADD.to_owned(),
        format!("{ADD}/new"),
        parameter_path(SPEED_KEY, ""),
        parameter_path(SPEED_KEY, "/limits"),
        parameter_path(SPEED_KEY, "/translations"),
    ];
    for path in paths {
        let reply = get_htmx(&world, &world.engineer, &path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    }
}
