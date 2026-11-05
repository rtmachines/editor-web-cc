use axum::Router;
use axum::extract::Path;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse as _, Response};
use axum::routing::get;
use recipe_app::auth::Operation;
use recipe_domain::identity::Role;

use super::PageRequest;
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{Fixture, PEER, get_with};

/// Answer `true` or `false` for the operation that the path names, or the code of the error.
async fn probe(request: PageRequest, Path(name): Path<String>) -> Response {
    let operation = match name.as_str() {
        "update-definition" => Operation::UpdateParameterDefinition,
        "export" => Operation::ExportApprovedVersion,
        "logout" => Operation::Logout,
        _ => Operation::GetVersion,
    };
    match request.may_run(operation) {
        Ok(allowed) => allowed.to_string().into_response(),
        Err(error) => error.code().to_string().into_response(),
    }
}

/// Build a router with one probe route. `Access::Lang` ignores the mode and the role.
fn router(fixture: &Fixture) -> Router {
    let rows = vec![RouteEntry {
        method: Method::GET,
        path: "/probe/{name}",
        access: Access::Lang,
        handler: get(probe),
    }];
    assemble(fixture.state.clone(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

/// Ask `may_run` for `name` as a user with `role` while the read-only mode is `read_only`.
async fn answer(role: Role, read_only: bool, name: &str) -> String {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, role, false);
    fixture.state.read_only().set(read_only);
    let reply = get_with(
        &router(&fixture),
        &format!("/probe/{name}"),
        &[("cookie", session.cookie.as_str())],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body
}

#[tokio::test]
async fn may_run_lets_a_process_engineer_edit_a_definition_in_normal_mode() {
    let body = answer(Role::ProcessEngineer, false, "update-definition").await;
    assert_eq!(body, "true");
}

#[tokio::test]
async fn may_run_stops_a_process_engineer_from_editing_a_definition_in_read_only_mode() {
    let body = answer(Role::ProcessEngineer, true, "update-definition").await;
    assert_eq!(body, "false");
}

#[tokio::test]
async fn may_run_refuses_an_operator_to_edit_a_definition_in_both_modes() {
    let normal = answer(Role::Operator, false, "update-definition").await;
    let read_only = answer(Role::Operator, true, "update-definition").await;
    assert_eq!((normal.as_str(), read_only.as_str()), ("false", "false"));
}

#[tokio::test]
async fn may_run_lets_an_operator_export_log_out_and_read_in_read_only_mode() {
    for name in ["export", "logout", "get-version"] {
        assert_eq!(answer(Role::Operator, true, name).await, "true", "{name}");
    }
}

#[tokio::test]
async fn may_run_without_a_session_gives_e_web_004() {
    let fixture = Fixture::new();
    let reply = get_with(&router(&fixture), "/probe/get-version", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, "E-WEB-004");
}
