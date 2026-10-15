//! Tests of the header `HX-Push-Url` of the HTMX error answers (task 13.6, design D20 U3,
//! ADR-0037 (z), spec "HTMX error headers").

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use crate::test_support::{Fixture, get_with, post_fields};

/// The headers of an HTMX request in English.
const HTMX_ENGLISH: [(&str, &str); 2] = [("hx-request", "true"), ("cookie", "recipe_lang=en")];

#[tokio::test]
async fn error_push_url_a_refused_machine_form_keeps_the_url() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::ProcessEngineer, false);
    let body = "code=X1&name=Line&plc_platform=S7_300&main_unit_name=Infeed";
    let reply = post_fields(
        &fixture.router(),
        "/machines",
        Some(&session),
        &HTMX_ENGLISH,
        body,
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-008"), "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.header("hx-push-url"), "false");
}

#[tokio::test]
async fn error_push_url_a_not_found_page_request_keeps_the_url() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Operator, false);
    fixture.seed_plant();
    let headers = [
        ("cookie", session.cookie.as_str()),
        HTMX_ENGLISH[0],
        HTMX_ENGLISH[1],
    ];
    let reply = get_with(&fixture.router(), "/machines/M99", &headers).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert_eq!(reply.header("hx-push-url"), "false");
}

#[tokio::test]
async fn error_push_url_a_page_request_without_htmx_sets_no_header() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Operator, false);
    fixture.seed_plant();
    let headers = [("cookie", session.cookie.as_str())];
    let reply = get_with(&fixture.router(), "/machines/M99", &headers).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert_eq!(reply.header("hx-push-url"), "");
}
