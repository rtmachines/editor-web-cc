//! Tests of the requirement "No password dialog of a browser" (ADR-0109 (o), R39, A45).
//!
//! The test router has a Basic row under `/export/v1/`, so the error layer gives the JSON answer
//! of ADR-0053. A `401` answer carries `WWW-Authenticate: RecipeExport realm="export"`. No answer
//! carries a challenge of the scheme `Basic`, and no Basic route answers with a `303`.

use axum::Router;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::routing::get;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_app::auth::Operation;
use recipe_domain::identity::Role;

use super::BasicUser;
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{Fixture, PEER, Reply, get_with};

/// The Basic route under the export prefix.
const EXPORT_ROUTE: &str = "/export/v1/test/basic";
/// The Basic route outside the export prefix. It gives the HTML answer.
const HTML_ROUTE: &str = "/test/basic";
/// The session route under the export prefix.
const SESSION_ROUTE: &str = "/export/v1/test/session";
/// The password of the seeded account.
const PASSWORD: &str = "tr4nsfer-s3cret";
/// The challenge of ADR-0109 (o).
const CHALLENGE: &str = "RecipeExport realm=\"export\"";
/// The header that carries the challenge.
const CHALLENGE_HEADER: &str = "www-authenticate";

async fn account(user: BasicUser) -> String {
    user.user().login().as_str().to_owned()
}

async fn reached() -> &'static str {
    "session route reached"
}

fn router(fixture: &Fixture) -> Router {
    let rows = vec![
        RouteEntry {
            method: Method::GET,
            path: EXPORT_ROUTE,
            access: Access::Basic(Operation::ExportApprovedVersion),
            handler: get(account),
        },
        RouteEntry {
            method: Method::GET,
            path: SESSION_ROUTE,
            access: Access::Operation(Operation::ExportApprovedVersion),
            handler: get(reached),
        },
    ];
    assemble(fixture.state.clone(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

fn fixture_with_client(must_change_password: bool) -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::ExportClient, PASSWORD, must_change_password);
    fixture
}

fn authorization(login: &str, password: &str) -> String {
    format!("Basic {}", STANDARD.encode(format!("{login}:{password}")))
}

async fn ask(fixture: &Fixture, path: &str, headers: &[(&str, &str)]) -> Reply {
    get_with(&router(fixture), path, headers).await
}

/// Check that the answer carries exactly the challenge of ADR-0109 (o) and no other.
fn assert_recipe_export_challenge(reply: &Reply, case: &str) {
    let values: Vec<_> = reply.headers.get_all(CHALLENGE_HEADER).iter().collect();
    assert_eq!(values.len(), 1, "{case}: {values:?}");
    assert_eq!(
        values.first().unwrap().to_str().unwrap(),
        CHALLENGE,
        "{case}"
    );
    assert_no_basic_challenge(reply, case);
}

/// Check that no `WWW-Authenticate` value of the answer starts with the scheme `Basic`.
fn assert_no_basic_challenge(reply: &Reply, case: &str) {
    for value in reply.headers.get_all(CHALLENGE_HEADER) {
        let text = value.to_str().unwrap().to_ascii_lowercase();
        assert!(!text.starts_with("basic"), "{case}: {text}");
        assert!(!text.contains("basic realm"), "{case}: {text}");
    }
}

/// Check the JSON error body `{"code":"<code>", ...}` of an export path.
fn assert_json_error(reply: &Reply, code: &str, case: &str) {
    assert_eq!(reply.header("content-type"), "application/json", "{case}");
    let prefix = format!("{{\"code\":\"{code}\",\"message\":\"");
    assert!(reply.body.starts_with(&prefix), "{case}: {}", reply.body);
    assert!(
        reply.body.contains("\"request_id\":\""),
        "{case}: {}",
        reply.body
    );
}

#[tokio::test]
async fn basic_request_without_header_gives_401_json_and_the_recipe_export_challenge() {
    let fixture = fixture_with_client(false);

    let reply = ask(&fixture, EXPORT_ROUTE, &[]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_json_error(&reply, "E-WEB-012", "missing header");
    assert_recipe_export_challenge(&reply, "missing header");
}

#[tokio::test]
async fn basic_request_with_wrong_password_gives_401_json_and_the_recipe_export_challenge() {
    let fixture = fixture_with_client(false);
    let header = authorization("user1", "wr0ng-s3cret");

    let reply = ask(&fixture, EXPORT_ROUTE, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_json_error(&reply, "E-IDN-003", "wrong password");
    assert_recipe_export_challenge(&reply, "wrong password");
}

#[tokio::test]
async fn basic_request_of_a_person_account_gives_401_and_the_recipe_export_challenge() {
    let fixture = Fixture::new();
    fixture.seed_account(3, Role::Administrator, PASSWORD, false);
    let header = authorization("user3", PASSWORD);

    let reply = ask(&fixture, EXPORT_ROUTE, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_json_error(&reply, "E-IDN-003", "person account");
    assert_recipe_export_challenge(&reply, "person account");
}

#[tokio::test]
async fn basic_request_with_a_malformed_header_gives_401_and_the_recipe_export_challenge() {
    let fixture = fixture_with_client(false);

    let reply = ask(&fixture, EXPORT_ROUTE, &[("authorization", "Bearer abc")]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_json_error(&reply, "E-WEB-012", "bearer scheme");
    assert_recipe_export_challenge(&reply, "bearer scheme");
}

#[tokio::test]
async fn basic_401_message_is_the_sentence_of_the_ui_language() {
    let fixture = fixture_with_client(false);

    let english = ask(&fixture, EXPORT_ROUTE, &[("cookie", "recipe_lang=en")]).await;
    let polish = ask(&fixture, EXPORT_ROUTE, &[("cookie", "recipe_lang=pl")]).await;

    assert!(
        english.body.contains(
            "\"message\":\"The request has no valid Basic credentials. \
             Send the login and the password of a transfer account.\""
        ),
        "{}",
        english.body
    );
    assert!(
        !polish
            .body
            .contains("The request has no valid Basic credentials"),
        "{}",
        polish.body
    );
    assert_recipe_export_challenge(&polish, "polish");
}

#[tokio::test]
async fn basic_success_carries_no_challenge() {
    let fixture = fixture_with_client(false);
    let header = authorization("user1", PASSWORD);

    let reply = ask(&fixture, EXPORT_ROUTE, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, "user1");
    assert!(reply.headers.get(CHALLENGE_HEADER).is_none());
}

#[tokio::test]
async fn basic_account_with_must_change_password_gets_no_303() {
    let fixture = fixture_with_client(true);
    let header = authorization("user1", PASSWORD);

    let reply = ask(&fixture, EXPORT_ROUTE, &[("authorization", &header)]).await;

    assert_ne!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert!(reply.headers.get("location").is_none());
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, "user1");
}

#[tokio::test]
async fn basic_refusal_of_a_must_change_password_account_gives_401_and_no_303() {
    let fixture = fixture_with_client(true);
    let header = authorization("user1", "wr0ng-s3cret");

    let reply = ask(&fixture, EXPORT_ROUTE, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert!(reply.headers.get("location").is_none());
    assert_recipe_export_challenge(&reply, "wrong password, must change");
}

#[tokio::test]
async fn basic_route_never_answers_303_for_a_refused_request() {
    let fixture = fixture_with_client(false);
    let wrong = authorization("user1", "wr0ng-s3cret");
    let cases: [(&str, Vec<(&str, &str)>); 2] = [
        ("no header", vec![]),
        ("wrong password", vec![("authorization", wrong.as_str())]),
    ];

    for (case, headers) in cases {
        let reply = ask(&fixture, EXPORT_ROUTE, &headers).await;

        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{case}");
        assert!(reply.headers.get("location").is_none(), "{case}");
        assert!(reply.headers.get("hx-redirect").is_none(), "{case}");
    }
}

#[tokio::test]
async fn basic_session_route_keeps_its_303_and_carries_no_basic_challenge() {
    let fixture = fixture_with_client(false);
    let header = authorization("user1", PASSWORD);

    let reply = ask(&fixture, SESSION_ROUTE, &[("authorization", &header)]).await;

    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), "/login");
    assert_no_basic_challenge(&reply, "session route");
}

#[tokio::test]
async fn basic_route_outside_the_export_prefix_carries_no_basic_challenge() {
    let fixture = fixture_with_client(false);

    let reply = get_with(&fixture.router(), HTML_ROUTE, &[]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_no_basic_challenge(&reply, "html route");
}
