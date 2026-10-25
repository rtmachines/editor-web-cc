//! Tests of the two panic layers (design D14 row T9, ADR-0057).
//!
//! The test routes live only in this file. Each panic comes from `expect` on `None`, which only
//! test code may call (`reliability.md` section 2.3). The JSON answer on `/export/v1/` has its
//! test in `error/export_json_tests.rs` (design D15).

use std::hint::black_box;

use axum::Router;
use axum::extract::Request;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::middleware::{Next, from_fn};
use axum::response::Response;
use axum::routing::{MethodRouter, get};

use super::INTERNAL_TEXT;
use crate::WebState;
use crate::i18n::Lang;
use crate::pipeline::wrap;
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{Fixture, PEER, REACHED, Reply, capture_logs, get_with, text};

/// The message of the test panic. No answer and no log line may hold it.
const PAYLOAD: &str = "the test value is missing";
/// The value of `HX-Reswap` for an HTMX error (ADR-0036 (p), `api.md` section 5.3).
const RESWAP: &str = "innerHTML show:#error-panel:top";
/// The header of an HTMX request.
const HTMX: (&str, &str) = ("hx-request", "true");
/// The cookie of the UI language `en`.
const ENGLISH: (&str, &str) = ("cookie", "recipe_lang=en");
/// The body of the backstop: the code and the English text of `reliability.md` section 1.2.
const BACKSTOP_BODY: &str = "E-WEB-010 An internal error occurred. Contact the administrator.";
/// The headers of `api.md` section 9 that the backstop answer does not have (ADR-0057 (j)).
const ABSENT_ON_BACKSTOP: [&str; 5] = [
    "content-security-policy",
    "x-content-type-options",
    "referrer-policy",
    "strict-transport-security",
    "cache-control",
];
/// The empty status region at the end of an HTMX error answer (ADR-0047 (b)).
const EMPTY_TOAST: &str = "<div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>";

/// A handler that panics. `black_box` hides the `None` from the optimizer and from clippy.
async fn explode() -> &'static str {
    black_box(None::<&'static str>).expect(PAYLOAD)
}

async fn reached() -> &'static str {
    REACHED
}

/// A layer that panics on `/boom` and passes every other request on.
async fn explode_in_layer(request: Request, next: Next) -> Response {
    if request.uri().path() == "/boom" {
        return black_box(None::<Response>).expect(PAYLOAD);
    }
    next.run(request).await
}

fn row(path: &'static str, handler: MethodRouter<WebState>) -> RouteEntry {
    RouteEntry {
        method: Method::GET,
        path,
        access: Access::Static,
        handler,
    }
}

/// Build the full router with a route that panics, a normal route, and a client at `PEER`.
fn router(fixture: &Fixture) -> Router {
    let rows = vec![
        row("/test/panic", get(explode)),
        row("/test/reached", get(reached)),
    ];
    assemble(fixture.state.clone(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

/// Build a router with the layers of `wrap` only.
///
/// Its layer panics on `/boom`. No `render_errors` and no inner panic layer sit around that layer.
/// Its panic takes the path of a panic in `render_errors`.
fn outer_router(fixture: &Fixture) -> Router {
    let inner = Router::new()
        .route("/boom", get(reached))
        .route("/ok", get(reached))
        .layer(from_fn(explode_in_layer));
    wrap(inner, fixture.state.settings())
}

/// Check that an answer holds no part of the panic: no payload, no location, no `Debug` text.
fn assert_no_panic_detail(reply: &Reply) {
    for detail in [PAYLOAD, ".rs:", "panicked", "Any {"] {
        assert!(!reply.body.contains(detail), "{detail}: {}", reply.body);
    }
}

/// Return the lines of `text` that the error mapping writes.
fn failure_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.contains("request failed"))
        .collect()
}

/// The backstop text must equal the English bundle text of `E-WEB-010`.
///
/// The bundle text equals the row `E-WEB-010` of `reliability.md` section 1.2. A change of that row
/// and of `en.ftl` would leave `INTERNAL_TEXT` stale without this test.
#[test]
fn backstop_text_equals_the_english_bundle_text_of_e_web_010() {
    assert_eq!(INTERNAL_TEXT, text(Lang::En, "error-E-WEB-010"));
}

#[tokio::test]
async fn panic_in_a_handler_gives_the_error_page_with_the_request_id() {
    let fixture = Fixture::new();
    let reply = get_with(&router(&fixture), "/test/panic", &[ENGLISH]).await;
    assert_eq!(
        reply.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        reply.body
    );
    let content_type = reply.header("content-type");
    assert!(content_type.starts_with("text/html"), "{content_type}");
    assert!(reply.body.contains("E-WEB-010"), "{}", reply.body);
    let request_id = reply.header("x-request-id");
    assert!(!request_id.is_empty());
    assert!(
        reply.body.contains(&format!("Reference: {request_id}")),
        "{}",
        reply.body
    );
    assert!(!reply.header("content-security-policy").is_empty());
    assert_eq!(reply.header("hx-retarget"), "");
    assert_no_panic_detail(&reply);
}

#[tokio::test]
async fn panic_in_a_handler_of_an_htmx_request_gives_the_error_fragment() {
    let fixture = Fixture::new();
    let reply = get_with(&router(&fixture), "/test/panic", &[HTMX, ENGLISH]).await;
    assert_eq!(
        reply.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        reply.body
    );
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.header("hx-reswap"), RESWAP);
    assert_eq!(reply.header("hx-push-url"), "false");
    assert!(reply.body.contains("E-WEB-010"), "{}", reply.body);
    assert!(
        reply.body.contains(&reply.header("x-request-id")),
        "{}",
        reply.body
    );
    assert!(reply.body.trim().ends_with(EMPTY_TOAST), "{}", reply.body);
    assert_no_panic_detail(&reply);
}

#[tokio::test]
async fn panic_in_a_handler_writes_one_line_with_the_code_and_the_request_id() {
    let fixture = Fixture::new();
    let (logs, _guard) = capture_logs();
    let reply = get_with(&router(&fixture), "/test/panic", &[]).await;
    let text = logs.text();
    let lines = failure_lines(&text);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = lines.first().unwrap();
    assert!(line.contains("error_code=E-WEB-010"), "{line}");
    let request_id = reply.header("x-request-id");
    assert!(line.contains(&format!("request_id={request_id}")), "{line}");
    assert!(!text.contains(PAYLOAD), "{text}");
}

#[tokio::test]
async fn panic_in_a_handler_keeps_the_router_serving_the_next_request() {
    let fixture = Fixture::new();
    let router = router(&fixture);
    let first = get_with(&router, "/test/panic", &[]).await;
    assert_eq!(first.status, StatusCode::INTERNAL_SERVER_ERROR);
    let next = get_with(&router, "/test/reached", &[]).await;
    assert_eq!(next.status, StatusCode::OK, "{}", next.body);
    assert_eq!(next.body, REACHED);
    let again = get_with(&router, "/test/panic", &[]).await;
    assert_eq!(again.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn panic_in_an_outer_layer_gives_the_plain_text_of_the_backstop() {
    let fixture = Fixture::new();
    let reply = get_with(&outer_router(&fixture), "/boom", &[HTMX]).await;
    assert_eq!(
        reply.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        reply.body
    );
    assert_eq!(reply.header("content-type"), "text/plain; charset=utf-8");
    assert_eq!(reply.body, BACKSTOP_BODY);
    assert!(!reply.header("x-request-id").is_empty());
    let names: Vec<&str> = reply.headers.keys().map(|name| name.as_str()).collect();
    assert!(
        names.iter().all(|name| !name.starts_with("hx-")),
        "{names:?}"
    );
    for absent in ABSENT_ON_BACKSTOP {
        assert!(!names.contains(&absent), "{absent}: {names:?}");
    }
    assert_no_panic_detail(&reply);
}

#[tokio::test]
async fn panic_in_an_outer_layer_writes_one_line_of_the_backstop_without_a_request_id() {
    let fixture = Fixture::new();
    let (logs, _guard) = capture_logs();
    let reply = get_with(&outer_router(&fixture), "/boom", &[]).await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    let text = logs.text();
    let lines = failure_lines(&text);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = lines.first().unwrap();
    assert!(line.contains("ERROR"), "{line}");
    assert!(line.contains("error_code=E-WEB-010"), "{line}");
    assert!(!line.contains("request_id"), "{line}");
    assert!(!text.contains(PAYLOAD), "{text}");
}

#[tokio::test]
async fn panic_in_an_outer_layer_keeps_the_router_serving_the_next_request() {
    let fixture = Fixture::new();
    let router = outer_router(&fixture);
    let first = get_with(&router, "/boom", &[]).await;
    assert_eq!(first.status, StatusCode::INTERNAL_SERVER_ERROR);
    let next = get_with(&router, "/ok", &[]).await;
    assert_eq!(next.status, StatusCode::OK, "{}", next.body);
    assert_eq!(next.body, REACHED);
}
