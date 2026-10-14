//! Requirement "Security headers on every response" (`api.md` section 9, design D11 steps 1 and 3).

use axum::body::Body;
use axum::http::{Method, Request};
use tower::ServiceExt as _;
use uuid::Uuid;

use crate::support::{app, get, header, router_with};

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

fn assert_security_headers(response: &axum::response::Response) {
    assert_eq!(header(response, "content-security-policy"), CSP);
    assert_eq!(header(response, "x-content-type-options"), "nosniff");
    assert_eq!(header(response, "referrer-policy"), "same-origin");
    assert_eq!(
        header(response, "strict-transport-security"),
        "max-age=31536000; includeSubDomains"
    );
}

#[tokio::test]
async fn headers_of_an_unknown_page_are_the_security_headers() {
    let response = get(&app(), "/no/such/page").await;
    assert_security_headers(&response);
    assert_eq!(header(&response, "cache-control"), "no-store");
}

#[tokio::test]
async fn headers_of_a_static_file_keep_its_own_cache_control() {
    let response = get(&app(), "/static/htmx.min.js").await;
    assert_security_headers(&response);
    assert_eq!(header(&response, "cache-control"), "public, max-age=86400");
}

#[tokio::test]
async fn headers_of_the_fail_closed_answer_are_the_security_headers() {
    // A request with another method than the table lists never reaches a handler.
    let response = crate::support::send(&app(), Method::DELETE, "/static/htmx.min.js").await;
    assert_security_headers(&response);
    assert_eq!(header(&response, "cache-control"), "no-store");
}

#[tokio::test]
async fn headers_hsts_max_age_follows_the_setting() {
    let settings = recipe_web::WebSettings {
        hsts_max_age: std::time::Duration::from_secs(600),
        ..recipe_web::WebSettings::default()
    };
    let response = get(&router_with(settings), "/no/such/page").await;
    assert_eq!(
        header(&response, "strict-transport-security"),
        "max-age=600; includeSubDomains"
    );
}

#[tokio::test]
async fn headers_request_id_is_a_new_uuid_v7_for_each_request() {
    let router = app();
    let first = header(&get(&router, "/static/htmx.min.js").await, "x-request-id");
    let second = header(&get(&router, "/static/htmx.min.js").await, "x-request-id");
    let first = Uuid::parse_str(&first).unwrap();
    let second = Uuid::parse_str(&second).unwrap();
    assert_eq!(first.get_version_num(), 7);
    assert_eq!(second.get_version_num(), 7);
    assert_ne!(first, second);
}

#[tokio::test]
async fn headers_request_id_ignores_the_value_of_the_client() {
    let request = Request::builder()
        .uri("/static/htmx.min.js")
        .header("x-request-id", "attacker-text")
        .body(Body::empty())
        .unwrap();
    let response = app().oneshot(request).await.unwrap();
    let shown = header(&response, "x-request-id");
    assert_ne!(shown, "attacker-text");
    assert!(Uuid::parse_str(&shown).is_ok(), "{shown}");
}
