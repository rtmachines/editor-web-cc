//! Tests of the request pipeline (task 6.5, design D11 steps 2 and 4 to 9, D14).

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, StatusCode};
use axum::routing::get;
use parking_lot::Mutex;
use recipe_domain::identity::Role;
use tracing_subscriber::fmt::MakeWriter;
use uuid::Uuid;

use super::{build_request_id, wrap};
use crate::settings::{CidrList, WebSettings};
use crate::test_support::{
    Fixture, REACHED, call, capture_logs, get_with, install, post_pre_session, post_with, request,
};

/// The key file of the pre-session CSRF cookie in these tests.
const KEYS: &str = crate::test_support::CSRF_KEY_NEW;

fn trusting(cidr: &str) -> Fixture {
    let web = WebSettings {
        trusted_proxies: CidrList::parse_all([cidr]).unwrap(),
        ..WebSettings::default()
    };
    Fixture::with_settings(web, KEYS)
}

/// Ask `GET /test/client` from `peer` with the given `X-Forwarded-For` header lines.
async fn client_from(
    fixture: &Fixture,
    peer: &str,
    forwarded: &[&str],
) -> crate::test_support::Reply {
    let peer: SocketAddr = peer.parse().unwrap();
    let mut headers = Vec::new();
    for line in forwarded {
        headers.push(("x-forwarded-for", *line));
    }
    get_with(&fixture.router_at(peer), "/test/client", &headers).await
}

// --- Step 1: the request id --------------------------------------------------------------------

fn id_of(reply: &crate::test_support::Reply) -> Uuid {
    Uuid::parse_str(&reply.header("x-request-id")).unwrap()
}

#[tokio::test]
async fn pipeline_request_ids_are_version_7_and_differ() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let first = id_of(&get_with(&router, "/test/login-form", &[]).await);
    let second = id_of(&get_with(&router, "/test/login-form", &[]).await);
    assert_eq!(first.get_version_num(), 7);
    assert_eq!(second.get_version_num(), 7);
    assert_ne!(first, second);
}

#[test]
fn request_id_with_random_bytes_is_version_7_and_keeps_the_millisecond_prefix() {
    let id = build_request_id(0x0123_4567_89ab, Some([0xAB; 16]), 5, 1);
    assert_eq!(id.get_version_num(), 7);
    assert_eq!(id.get_variant(), uuid::Variant::RFC4122);
    assert_eq!(
        id.as_bytes().get(..6),
        Some(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab][..])
    );
}

#[test]
fn request_id_without_random_bytes_is_version_7() {
    let id = build_request_id(1_700_000_000_000, None, 123_456, 9);
    assert_eq!(id.get_version_num(), 7);
    assert_eq!(id.get_variant(), uuid::Variant::RFC4122);
}

#[test]
fn request_ids_without_random_bytes_differ_for_the_same_clock_value() {
    let first = build_request_id(1_700_000_000_000, None, 123_456, 1);
    let second = build_request_id(1_700_000_000_000, None, 123_456, 2);
    assert_eq!(first.get_version_num(), 7);
    assert_eq!(second.get_version_num(), 7);
    assert_ne!(first, second);
}

#[test]
fn a_clock_at_zero_milliseconds_still_gives_a_version_7_id() {
    let first = build_request_id(0, None, 0, 0);
    let second = build_request_id(0, None, 0, 1);
    assert_eq!(first.get_version_num(), 7);
    assert_ne!(first, second);
}

// --- Step 4: the client address ----------------------------------------------------------------

#[tokio::test]
async fn pipeline_forwarded_header_of_an_untrusted_peer_does_not_count() {
    let fixture = Fixture::new();
    let reply = client_from(&fixture, "10.0.0.9:5000", &["10.0.0.5"]).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, "10.0.0.9");
}

#[tokio::test]
async fn pipeline_mapped_ipv6_peer_matches_the_ipv4_trusted_network() {
    let fixture = trusting("172.20.0.0/16");
    let reply = client_from(&fixture, "[::ffff:172.20.0.2]:5000", &["10.0.0.5"]).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, "10.0.0.5");
}

#[tokio::test]
async fn pipeline_mapped_ipv6_peer_is_stored_in_its_ipv4_form() {
    let fixture = Fixture::new();
    let reply = client_from(&fixture, "[::ffff:10.0.0.9]:5000", &[]).await;
    assert_eq!(reply.body, "10.0.0.9");
}

#[tokio::test]
async fn pipeline_mapped_ipv6_forwarded_entry_is_stored_in_its_ipv4_form() {
    let fixture = trusting("172.20.0.0/16");
    let reply = client_from(&fixture, "172.20.0.2:5000", &["::ffff:10.0.0.5"]).await;
    assert_eq!(reply.body, "10.0.0.5");
}

#[tokio::test]
async fn pipeline_rightmost_entry_that_is_not_a_trusted_proxy_is_the_client() {
    let fixture = trusting("172.20.0.0/16");
    let reply = client_from(
        &fixture,
        "172.20.0.2:5000",
        &["1.1.1.1, 10.0.0.5, 172.20.0.3"],
    )
    .await;
    assert_eq!(reply.body, "10.0.0.5");
}

#[tokio::test]
async fn pipeline_several_forwarded_header_lines_count_in_order() {
    let fixture = trusting("172.20.0.0/16");
    let reply = client_from(&fixture, "172.20.0.2:5000", &["1.1.1.1", "10.0.0.5"]).await;
    assert_eq!(reply.body, "10.0.0.5");
}

#[tokio::test]
async fn pipeline_trusted_peer_without_forwarded_header_gives_e_web_010() {
    let fixture = trusting("172.20.0.0/16");
    let reply = client_from(&fixture, "172.20.0.2:5000", &[]).await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(reply.body.contains("E-WEB-010"), "{}", reply.body);
    // The handler never runs, so the use case inserts no login attempt row.
    assert!(!reply.body.contains(REACHED), "{}", reply.body);
    let peer: SocketAddr = "172.20.0.2:5000".parse().unwrap();
    let login = post_with(&fixture.router_at(peer), "/test/login", None, &[]).await;
    assert_eq!(login.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(login.body.contains("E-WEB-010"), "{}", login.body);
    assert!(!login.body.contains(REACHED), "{}", login.body);
}

#[tokio::test]
async fn pipeline_chain_of_trusted_proxies_only_gives_the_first_entry() {
    let fixture = trusting("172.20.0.0/16");
    let reply = client_from(&fixture, "172.20.0.2:5000", &["172.20.0.9, 172.20.0.3"]).await;
    assert_eq!(reply.body, "172.20.0.9");
}

#[tokio::test]
async fn pipeline_forwarded_entry_that_is_not_an_address_gives_e_web_008() {
    let fixture = trusting("172.20.0.0/16");
    for bad in [
        "not-an-address",
        "garbage, 10.0.0.5",
        "10.0.0.5, garbage",
        "10.0.0.5,",
        "10.0.0.5:80",
        "",
    ] {
        let reply = client_from(&fixture, "172.20.0.2:5000", &[bad]).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad:?}");
        assert!(reply.body.contains("E-WEB-008"), "{bad:?}: {}", reply.body);
        assert!(!reply.body.contains(REACHED), "{bad:?}");
    }
}

#[tokio::test]
async fn pipeline_bad_forwarded_header_of_an_untrusted_peer_is_ignored() {
    let fixture = Fixture::new();
    let reply = client_from(&fixture, "10.0.0.9:5000", &["not-an-address"]).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, "10.0.0.9");
}

#[tokio::test]
async fn pipeline_missing_connect_info_gives_e_web_010() {
    let fixture = Fixture::new();
    let router = fixture.router_without_peer();
    let reply = get_with(&router, "/test/client", &[]).await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(reply.body.contains("E-WEB-010"), "{}", reply.body);
    assert!(!reply.header("x-request-id").is_empty());
}

// --- Step 5: the body limit --------------------------------------------------------------------

#[tokio::test]
async fn pipeline_body_above_the_form_limit_gives_e_web_003() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let limit = 256 * 1024;
    let body = "a".repeat(limit + 1);
    let request = request(Method::POST, "/test/users")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str())
        .header("x-csrf-token", session.csrf.as_str())
        .body(Body::from(body))
        .unwrap();
    let reply = call(&fixture.router(), request).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(reply.body.contains("E-WEB-003"), "{}", reply.body);
    assert!(reply.body.contains("256 KiB"), "{}", reply.body);
}

#[tokio::test]
async fn pipeline_declared_length_above_the_form_limit_gives_e_web_003() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let request = request(Method::POST, "/test/users")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("content-length", "262145")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str())
        .header("x-csrf-token", session.csrf.as_str())
        .body(Body::empty())
        .unwrap();
    let reply = call(&fixture.router(), request).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(reply.body.contains("E-WEB-003"), "{}", reply.body);
}

#[tokio::test]
async fn pipeline_body_of_exactly_the_form_limit_passes() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let request = request(Method::POST, "/test/users")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str())
        .header("x-csrf-token", session.csrf.as_str())
        .body(Body::from("a".repeat(256 * 1024)))
        .unwrap();
    let reply = call(&fixture.router(), request).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, REACHED);
}

// --- Step 7: the session layer -----------------------------------------------------------------

fn cleared(reply: &crate::test_support::Reply) -> bool {
    reply.set_cookies().iter().any(|cookie| {
        cookie.starts_with("__Host-recipe_session=;")
            && cookie.contains("Max-Age=0")
            && cookie.contains("Path=/")
            && cookie.contains("Secure")
            && cookie.contains("HttpOnly")
    })
}

#[tokio::test]
async fn pipeline_valid_session_reaches_the_handler_with_its_user() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(7, Role::Operator, false);
    let reply = get_with(
        &fixture.router(),
        "/test/profile",
        &[("cookie", session.cookie.as_str())],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, "user7");
    assert!(!cleared(&reply));
}

#[tokio::test]
async fn pipeline_route_with_a_session_requirement_and_no_cookie_redirects_to_login() {
    let fixture = Fixture::new();
    let reply = get_with(&fixture.router(), "/test/profile", &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), "/login");
    assert!(cleared(&reply), "{:?}", reply.set_cookies());
}

#[tokio::test]
async fn pipeline_htmx_request_without_a_session_gets_401_and_hx_redirect() {
    let fixture = Fixture::new();
    let reply = get_with(
        &fixture.router(),
        "/test/profile",
        &[("hx-request", "true")],
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.header("hx-redirect"), "/login");
    assert!(reply.body.contains("E-WEB-004"), "{}", reply.body);
    assert!(cleared(&reply));
}

#[tokio::test]
async fn pipeline_invalid_cookie_values_give_e_web_004_and_clear_the_cookie() {
    let fixture = Fixture::new();
    let unknown = format!("__Host-recipe_session={}", "ab".repeat(16));
    for cookie in [
        "__Host-recipe_session=not-a-token",
        "__Host-recipe_session=",
        "__Host-recipe_session=ABABABABABABABABABABABABABABABAB",
        unknown.as_str(),
    ] {
        let reply = get_with(
            &fixture.router(),
            "/test/profile",
            &[("cookie", cookie), ("hx-request", "true")],
        )
        .await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{cookie}");
        assert!(cleared(&reply), "{cookie}");
    }
}

#[tokio::test]
async fn pipeline_no_session_route_ignores_an_invalid_cookie() {
    let fixture = Fixture::new();
    let reply = get_with(
        &fixture.router(),
        "/test/login-form",
        &[("cookie", "__Host-recipe_session=not-a-token")],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, "none");
    assert!(!cleared(&reply));
}

#[tokio::test]
async fn pipeline_no_session_route_ignores_a_valid_session() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(7, Role::Operator, false);
    let reply = get_with(
        &fixture.router(),
        "/test/login-form",
        &[("cookie", session.cookie.as_str())],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, "none");
}

#[tokio::test]
async fn pipeline_static_files_need_no_session_and_ignore_a_bad_cookie() {
    let fixture = Fixture::new();
    let reply = get_with(
        &fixture.router(),
        "/static/htmx.min.js",
        &[("cookie", "__Host-recipe_session=not-a-token")],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!cleared(&reply));
}

#[tokio::test]
async fn pipeline_session_store_failure_gives_the_storage_error() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(7, Role::Operator, false);
    fixture
        .store
        .fail_begin(recipe_app::memory::Fault::Unavailable);
    let reply = get_with(
        &fixture.router(),
        "/test/profile",
        &[("cookie", session.cookie.as_str())],
    )
    .await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(reply.body.contains("E-DB-005"), "{}", reply.body);
}

// --- Step 9: the password gate ----------------------------------------------------------------

#[tokio::test]
async fn pipeline_must_change_password_redirects_every_other_route() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Administrator, true);
    let router = fixture.router();
    let cookie = [("cookie", session.cookie.as_str())];
    for path in ["/test/profile", "/test/users"] {
        let reply = get_with(&router, path, &cookie).await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{path}");
        assert_eq!(reply.header("location"), "/me/password", "{path}");
    }
    let reply = post_with(&router, "/test/users", Some(&session), &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), "/me/password");
}

#[tokio::test]
async fn pipeline_must_change_password_still_reaches_the_five_exceptions() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Administrator, true);
    let router = fixture.router();
    let cookie = [("cookie", session.cookie.as_str())];
    let reply = get_with(&router, "/test/me-password", &cookie).await;
    assert_eq!(
        (reply.status, reply.body.as_str()),
        (StatusCode::OK, REACHED)
    );
    let reply = get_with(&router, "/static/htmx.min.js", &cookie).await;
    assert_eq!(reply.status, StatusCode::OK);
    let reply = post_with(&router, "/test/me-password", Some(&session), &[]).await;
    assert_eq!(
        (reply.status, reply.body.as_str()),
        (StatusCode::OK, REACHED)
    );
    let reply = post_with(&router, "/test/logout", Some(&session), &[]).await;
    assert_eq!(
        (reply.status, reply.body.as_str()),
        (StatusCode::OK, REACHED)
    );
}

#[tokio::test]
async fn pipeline_session_without_the_flag_passes_the_password_gate() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Administrator, false);
    let reply = get_with(
        &fixture.router(),
        "/test/users",
        &[("cookie", session.cookie.as_str())],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
}

// --- Step 9: the read-only gate ----------------------------------------------------------------

#[tokio::test]
async fn pipeline_read_only_mode_stops_a_command_with_e_web_006() {
    let fixture = Fixture::new();
    fixture.state.read_only().set(true);
    let session = fixture.seed_session(1, Role::Administrator, false);
    let reply = post_with(&fixture.router(), "/test/users", Some(&session), &[]).await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(reply.body.contains("E-WEB-006"), "{}", reply.body);
    assert_eq!(reply.header("retry-after"), "5");
    assert!(!reply.body.contains(REACHED));
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn pipeline_read_only_mode_lets_login_logout_queries_and_lang_pass() {
    let fixture = Fixture::new();
    fixture.state.read_only().set(true);
    let session = fixture.seed_session(1, Role::Administrator, false);
    let router = fixture.router();
    let reply = post_pre_session(&fixture, "/test/login", "login=anna", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = post_with(&router, "/test/logout", Some(&session), &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = get_with(
        &router,
        "/test/users",
        &[("cookie", session.cookie.as_str())],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
}

#[tokio::test]
async fn pipeline_command_passes_when_read_only_mode_is_off() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let reply = post_with(&fixture.router(), "/test/users", Some(&session), &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

// --- Step 2: the trace layer -------------------------------------------------------------------

#[tokio::test]
async fn pipeline_trace_line_holds_the_span_fields_and_no_header_or_body() {
    let (logs, _guard) = capture_logs();
    let fixture = Fixture::new();
    let session = fixture.seed_session(7, Role::Operator, false);
    let agent = "a".repeat(300);
    let reply = get_with(
        &fixture.router(),
        "/test/profile",
        &[
            ("cookie", session.cookie.as_str()),
            ("user-agent", agent.as_str()),
            ("authorization", "Bearer secret-bearer-value"),
            ("x-csrf-token", "secret-csrf-header"),
            ("x-custom-header", "secret-custom-header"),
        ],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    let text = logs.text();
    for field in [
        "request_id=",
        "method=GET",
        "route=\"/test/profile\"",
        "client_ip=192.0.2.10",
        "session_ref=",
        "user_id=",
        "role=Operator",
        "status=200",
        "latency_ms=",
    ] {
        assert!(text.contains(field), "missing {field}: {text}");
    }
    assert!(text.contains(&"a".repeat(256)), "{text}");
    assert!(!text.contains(&"a".repeat(257)), "{text}");
    for secret in [
        session.cookie.as_str(),
        session.csrf.as_str(),
        "secret-bearer-value",
        "secret-csrf-header",
        "secret-custom-header",
    ] {
        assert!(!text.contains(secret), "{secret} in {text}");
    }
}

#[tokio::test]
async fn pipeline_trace_line_holds_no_request_body() {
    let (logs, _guard) = capture_logs();
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let request = request(Method::POST, "/test/users")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str())
        .header("x-csrf-token", session.csrf.as_str())
        .body(Body::from("password=hunter2-secret-body"))
        .unwrap();
    let reply = call(&fixture.router(), request).await;
    assert_eq!(reply.status, StatusCode::OK);
    let text = logs.text();
    assert!(text.contains("status=200"), "{text}");
    assert!(!text.contains("hunter2-secret-body"), "{text}");
}

/// A writer that copies every line of the log into a shared buffer.
#[derive(Clone, Default)]
struct Lines(Arc<Mutex<Vec<u8>>>);

impl io::Write for Lines {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Lines {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// A handler that writes one `ERROR` line without a request id of its own.
async fn fails_with_a_line() -> &'static str {
    tracing::error!("inner fault");
    REACHED
}

#[tokio::test]
async fn pipeline_error_line_holds_the_request_id_under_a_warn_filter() {
    // The global subscriber of the tests keeps the callsites enabled. The subscriber of this
    // thread then decides, and it lets only `WARN` and `ERROR` pass, like `RUST_LOG=warn`.
    let _global = capture_logs();
    let lines = Lines::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(lines.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .finish();
    let _scoped = install(subscriber);
    let fixture = Fixture::new();
    let inner = Router::new().route("/inner-fault", get(fails_with_a_line));
    let router = wrap(inner, fixture.state.settings());
    let reply = get_with(&router, "/inner-fault", &[]).await;
    assert_eq!(reply.status, StatusCode::OK);
    let text = String::from_utf8_lossy(&lines.0.lock()).into_owned();
    let line = text
        .lines()
        .find(|line| line.contains("inner fault"))
        .expect(&text);
    let id = reply.header("x-request-id");
    assert!(line.contains("ERROR"), "{line}");
    assert!(
        line.contains(&format!("request_id={id}")),
        "no request_id in {line}"
    );
}
