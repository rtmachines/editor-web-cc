//! Tests of the request timeout, the body deadline, and the default trusted proxy (design D14
//! row T10, ADR-0058 (g) to (j), ADR-0065 (j), ADR-0072).
//!
//! Every test with time runs on a paused clock. The clock jumps to the next timer when no task is
//! ready, so no test waits for a real second. The JSON answers on `/export/v1/` have their tests
//! in `error/export_json_tests.rs` (design D15). The body deadline on an export path has its test
//! here (ADR-0072 (e)). The unit test of the body mapping is in `csrf/tests.rs`.

use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::routing::{MethodRouter, get, post};
use http_body_util::channel::Channel;
use recipe_app::auth::Operation;
use tokio::time::Instant;

use crate::WebState;
use crate::routes::{Access, RouteEntry, assemble};
use crate::settings::WebSettings;
use crate::test_support::{CSRF_KEY_NEW, Fixture, PEER, REACHED, Reply, call, get_with, request};

/// One second.
const SECOND: Duration = Duration::from_secs(1);
/// The request timeout of the tests of the body deadline. Only the body deadline can end first.
const LONG_REQUEST_TIMEOUT: Duration = Duration::from_secs(3600);
/// The time between two bytes of a body that trickles.
const TRICKLE_STEP: Duration = Duration::from_secs(4);
/// The time after which a body that trickles ends.
const TRICKLE_SPAN: Duration = Duration::from_secs(20);
/// The bound of a test future. A request that never ends fails the test and does not hang it.
const TEST_BOUND: Duration = Duration::from_secs(7200);

/// A handler that outlasts the request timeout.
async fn slow(State(state): State<WebState>) -> &'static str {
    let wait = state.settings().request_timeout.saturating_mul(2);
    tokio::time::sleep(wait).await;
    REACHED
}

/// A handler that ends one second before the request timeout.
async fn just_in_time(State(state): State<WebState>) -> &'static str {
    let wait = state
        .settings()
        .request_timeout
        .checked_sub(SECOND)
        .unwrap();
    tokio::time::sleep(wait).await;
    REACHED
}

async fn reached() -> &'static str {
    REACHED
}

fn row(
    method: Method,
    path: &'static str,
    access: Access,
    handler: MethodRouter<WebState>,
) -> RouteEntry {
    RouteEntry {
        method,
        path,
        access,
        handler,
    }
}

/// Build the full router with the four test routes and a client at `PEER`.
fn router(fixture: &Fixture) -> Router {
    let rows = vec![
        row(Method::GET, "/test/slow", Access::Static, get(slow)),
        row(
            Method::GET,
            "/test/in-time",
            Access::Static,
            get(just_in_time),
        ),
        row(
            Method::POST,
            "/test/body",
            Access::Operation(Operation::Login),
            post(reached),
        ),
        row(
            Method::POST,
            "/export/v1/test/body",
            Access::Operation(Operation::Login),
            post(reached),
        ),
    ];
    assemble(fixture.state.clone(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

/// Wait for `future`, and fail the test when the paused clock passes the bound.
async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(TEST_BOUND, future)
        .await
        .expect("the request did not end")
}

/// Assert that `elapsed` is `limit` plus less than one second.
fn assert_ended_at(elapsed: Duration, limit: Duration, what: &str) {
    assert!(elapsed >= limit, "{what}: {elapsed:?} is before {limit:?}");
    let latest = limit.checked_add(SECOND).unwrap();
    assert!(elapsed < latest, "{what}: {elapsed:?} is after {latest:?}");
}

#[tokio::test(start_paused = true)]
async fn timeout_a_slow_handler_gives_503_e_web_009_with_retry_after_at_the_limit() {
    let fixture = Fixture::new();
    let settings = fixture.state.settings().clone();
    let started = Instant::now();
    let reply = bounded(get_with(&router(&fixture), "/test/slow", &[])).await;
    assert_ended_at(started.elapsed(), settings.request_timeout, "slow");
    assert_eq!(
        reply.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-009"), "{}", reply.body);
    assert!(!reply.body.contains(REACHED), "{}", reply.body);
    assert_eq!(
        reply.header("retry-after"),
        settings.retry_after.as_secs().to_string()
    );
}

#[tokio::test(start_paused = true)]
async fn timeout_a_handler_that_ends_before_the_limit_is_not_cut() {
    let fixture = Fixture::new();
    let reply = bounded(get_with(&router(&fixture), "/test/in-time", &[])).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, REACHED);
}

/// Send a form with `body` to `path`. Return the answer and the paused time of the request.
async fn post_body_to(fixture: &Fixture, path: &str, body: Channel<Bytes>) -> (Reply, Duration) {
    let request = request(Method::POST, path)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .body(Body::new(body))
        .unwrap();
    let started = Instant::now();
    let reply = bounded(call(&router(fixture), request)).await;
    (reply, started.elapsed())
}

/// Send a form with `body` to the page route. Return the answer and the paused time.
async fn post_body(fixture: &Fixture, body: Channel<Bytes>) -> (Reply, Duration) {
    post_body_to(fixture, "/test/body", body).await
}

/// Send a form whose body never ends. The sender stays open until the request has ended.
async fn post_stalled_body(fixture: &Fixture) -> (Reply, Duration) {
    let (_sender, body) = Channel::<Bytes>::new(1);
    post_body(fixture, body).await
}

#[tokio::test(start_paused = true)]
async fn timeout_a_body_that_stops_gives_e_web_009_at_the_body_deadline() {
    let web = WebSettings {
        request_timeout: LONG_REQUEST_TIMEOUT,
        ..WebSettings::default()
    };
    let fixture = Fixture::with_settings(web, CSRF_KEY_NEW);
    let (reply, elapsed) = post_stalled_body(&fixture).await;
    let deadline = fixture.state.settings().body_read_timeout;
    assert_ended_at(elapsed, deadline, "stalled body");
    assert_eq!(
        reply.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-009"), "{}", reply.body);
    assert!(!reply.body.contains("E-WEB-003"), "{}", reply.body);
    assert_eq!(
        reply.header("retry-after"),
        fixture.state.settings().retry_after.as_secs().to_string()
    );
    assert!(fixture.sink.events().is_empty());
}

/// A body that stops on an export path still ends at the body deadline with the JSON `E-WEB-009`.
///
/// The request timeout skips the export paths, so only the body deadline can end this request
/// (ADR-0072 (b)). A missing deadline makes the request outlast the bound of the test.
#[tokio::test(start_paused = true)]
async fn timeout_a_body_that_stops_on_an_export_path_gives_json_e_web_009_at_the_body_deadline() {
    let fixture = Fixture::new();
    let (_sender, body) = Channel::<Bytes>::new(1);
    let (reply, elapsed) = post_body_to(&fixture, "/export/v1/test/body", body).await;
    let deadline = fixture.state.settings().body_read_timeout;
    assert_ended_at(elapsed, deadline, "stalled export body");
    assert_eq!(
        reply.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        reply.body
    );
    assert_eq!(reply.header("content-type"), "application/json");
    assert!(
        reply.body.starts_with("{\"code\":\"E-WEB-009\""),
        "{}",
        reply.body
    );
}

/// Send one byte every `TRICKLE_STEP`. End the body after `TRICKLE_SPAN` of paused time.
///
/// The task stops at once when the receiver is gone, because the request has ended.
fn trickle_body() -> Channel<Bytes> {
    let (mut sender, body) = Channel::<Bytes>::new(1);
    tokio::spawn(async move {
        let started = Instant::now();
        while started.elapsed() < TRICKLE_SPAN {
            tokio::time::sleep(TRICKLE_STEP).await;
            if sender.send_data(Bytes::from_static(b"x")).await.is_err() {
                return;
            }
        }
    });
    body
}

/// A client that sends one byte every 4 s gets `E-WEB-009` at the body deadline.
///
/// The body ends at 20 s. A timer for each frame would never fire, because a frame arrives every
/// 4 s. Only one deadline for the whole body ends the request at 10 s (ADR-0058 (i)).
#[tokio::test(start_paused = true)]
async fn timeout_a_body_that_trickles_gives_e_web_009_at_the_body_deadline() {
    let web = WebSettings {
        request_timeout: LONG_REQUEST_TIMEOUT,
        ..WebSettings::default()
    };
    let fixture = Fixture::with_settings(web, CSRF_KEY_NEW);
    let deadline = fixture.state.settings().body_read_timeout;
    assert!(TRICKLE_STEP < deadline && deadline < TRICKLE_SPAN);
    let (reply, elapsed) = post_body(&fixture, trickle_body()).await;
    assert_ended_at(elapsed, deadline, "trickled body");
    assert_eq!(
        reply.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-009"), "{}", reply.body);
}

/// Ask `GET /test/client` from `peer` with one `X-Forwarded-For` header line.
async fn client_from(fixture: &Fixture, peer: &str, forwarded: &str) -> Reply {
    let peer: SocketAddr = peer.parse().unwrap();
    get_with(
        &fixture.router_at(peer),
        "/test/client",
        &[("x-forwarded-for", forwarded)],
    )
    .await
}

#[tokio::test]
async fn default_proxy_a_neighbor_container_cannot_set_the_client_address() {
    let fixture = Fixture::new();
    let reply = client_from(&fixture, "172.28.0.3:40000", "198.51.100.7").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, "172.28.0.3");
}

#[tokio::test]
async fn default_proxy_the_address_of_the_proxy_sets_the_client_address() {
    let fixture = Fixture::new();
    let reply = client_from(&fixture, "172.28.0.2:40000", "198.51.100.7").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, "198.51.100.7");
}
