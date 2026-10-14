//! Tests of the CSRF layer (task 6.6, design D12, ADR-0035 e, s, t).

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::{Method, StatusCode};
use recipe_app::memory::Fault;
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::{KeyRing, Role};

use super::{buffer_body, is_mutating, issue_pre_session};
use crate::error::WebError;
use crate::settings::WebSettings;
use crate::test_support::{
    CSRF_KEY_NEW, CSRF_KEY_OLD, Fixture, PEER, REACHED, Reply, TestSession, call, capture_logs,
    cookie_pair, get_with, request,
};

const SAME_ORIGIN: (&str, &str) = ("sec-fetch-site", "same-origin");
const HOST: &str = "recipes.plant.local";
const PRE_COOKIE: &str = "__Host-recipe_csrf";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

/// Send a urlencoded form with the method `POST` and the given headers.
async fn post(router: &Router, path: &str, headers: &[(&str, &str)], body: &str) -> Reply {
    send(router, Method::POST, path, headers, body).await
}

async fn send(
    router: &Router,
    method: Method,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> Reply {
    let mut builder =
        request(method, path).header("content-type", "application/x-www-form-urlencoded");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    call(router, builder.body(Body::from(body.to_owned())).unwrap()).await
}

/// Assert the answer of a rejected request: `403 E-WEB-007`, no handler, and one event.
fn assert_rejected(fixture: &Fixture, reply: &Reply, what: &str) {
    assert_eq!(
        reply.status,
        StatusCode::FORBIDDEN,
        "{what}: {}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-007"), "{what}: {}", reply.body);
    assert!(!reply.body.contains(REACHED), "{what}");
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{what}: {events:?}");
    let event = &events[0];
    assert_eq!(event.kind(), SecurityEventKind::CsrfRejected, "{what}");
    assert_eq!(event.client_ip(), Some(PEER.ip()), "{what}");
}

fn assert_accepted(fixture: &Fixture, reply: &Reply, what: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
    assert_eq!(reply.body, REACHED, "{what}");
    assert!(fixture.sink.events().is_empty(), "{what}");
}

fn session_headers<'a>(
    session: &'a TestSession,
    extra: &[(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    let mut headers = vec![("cookie", session.cookie.as_str())];
    headers.extend_from_slice(extra);
    headers
}

// --- The session token -------------------------------------------------------------------------

#[tokio::test]
async fn csrf_session_token_in_the_header_is_accepted() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let headers = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &session.csrf)]);
    let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
    assert_accepted(&fixture, &reply, "header");
}

#[tokio::test]
async fn csrf_session_token_in_the_form_field_is_accepted() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let body = format!("a=b&csrf_token={}&c=d", session.csrf);
    let headers = session_headers(&session, &[SAME_ORIGIN]);
    let reply = post(&fixture.router(), "/test/users", &headers, &body).await;
    assert_accepted(&fixture, &reply, "field");
}

#[tokio::test]
async fn csrf_put_and_delete_need_the_token_too() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let router = fixture.router();
    let good = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &session.csrf)]);
    for method in [Method::PUT, Method::DELETE] {
        let reply = send(&router, method.clone(), "/test/users", &good, "").await;
        assert_eq!(reply.status, StatusCode::OK, "{method}");
    }
    let bad = session_headers(&session, &[SAME_ORIGIN]);
    for method in [Method::PUT, Method::DELETE] {
        let reply = send(&router, method.clone(), "/test/users", &bad, "").await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method}");
    }
    assert_eq!(fixture.sink.events().len(), 2);
}

#[test]
fn csrf_every_method_except_get_head_and_options_is_mutating() {
    for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
        assert!(!is_mutating(&method), "{method}");
    }
    let unusual = Method::from_bytes(b"PROPFIND").unwrap();
    for method in [
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::TRACE,
        Method::CONNECT,
        unusual,
    ] {
        assert!(is_mutating(&method), "{method}");
    }
}

#[tokio::test]
async fn csrf_patch_needs_the_token_and_the_origin_too() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let router = fixture.router();
    let bare = session_headers(&session, &[]);
    let reply = send(&router, Method::PATCH, "/test/users", &bare, "").await;
    assert_rejected(&fixture, &reply, "patch without token and origin");
    let good = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &session.csrf)]);
    let reply = send(&router, Method::PATCH, "/test/users", &good, "").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, REACHED);
}

#[tokio::test]
async fn csrf_get_needs_no_token_and_no_origin() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let reply = get_with(
        &fixture.router(),
        "/test/users",
        &[("cookie", session.cookie.as_str())],
    )
    .await;
    assert_accepted(&fixture, &reply, "get");
}

#[tokio::test]
async fn csrf_token_of_another_session_is_rejected() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let other = fixture.seed_session(2, Role::Administrator, false);
    let headers = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &other.csrf)]);
    let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
    assert_rejected(&fixture, &reply, "token of another session");
}

#[tokio::test]
async fn csrf_missing_or_malformed_session_token_is_rejected() {
    for (token_header, body) in [
        (None, "a=b".to_owned()),
        (Some("abc"), "a=b".to_owned()),
        (Some(""), "a=b".to_owned()),
        (None, "csrf_token=&a=b".to_owned()),
        (None, "csrf_token=short".to_owned()),
    ] {
        let fixture = Fixture::new();
        let session = fixture.seed_session(1, Role::Administrator, false);
        let mut extra = vec![SAME_ORIGIN];
        if let Some(token) = token_header {
            extra.push(("x-csrf-token", token));
        }
        let headers = session_headers(&session, &extra);
        let reply = post(&fixture.router(), "/test/users", &headers, &body).await;
        assert_rejected(&fixture, &reply, &format!("{token_header:?} {body}"));
    }
}

#[tokio::test]
async fn csrf_a_wrong_header_is_not_rescued_by_a_right_field() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let body = format!("csrf_token={}", session.csrf);
    let headers = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", "wrong")]);
    let reply = post(&fixture.router(), "/test/users", &headers, &body).await;
    assert_rejected(&fixture, &reply, "wrong header");
}

#[tokio::test]
async fn csrf_field_in_a_body_that_is_not_a_form_does_not_count() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let body = format!(r#"{{"csrf_token":"{}"}}"#, session.csrf);
    let request = request(Method::POST, "/test/users")
        .header("content-type", "application/json")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str())
        .body(Body::from(body))
        .unwrap();
    let reply = call(&fixture.router(), request).await;
    assert_rejected(&fixture, &reply, "json body");
}

#[tokio::test]
async fn csrf_percent_encoded_field_is_decoded() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let encoded: String = session
        .csrf
        .chars()
        .map(|c| format!("%{:02X}", u32::from(c)))
        .collect();
    let body = format!("csrf%5Ftoken={encoded}");
    let headers = session_headers(&session, &[SAME_ORIGIN]);
    let reply = post(&fixture.router(), "/test/users", &headers, &body).await;
    assert_accepted(&fixture, &reply, "encoded");
}

#[tokio::test]
async fn csrf_layer_gives_the_body_back_to_the_handler() {
    let fixture = Fixture::new();
    let reply =
        crate::test_support::post_pre_session(&fixture, "/lang", "lang=en&next=%2Fusers", &[])
            .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/users");
}

#[tokio::test]
async fn csrf_buffer_above_the_limit_gives_e_web_003() {
    let error = buffer_body(Body::from(vec![b'a'; 11]), 10)
        .await
        .unwrap_err();
    assert!(
        matches!(error, WebError::PayloadTooLarge { .. }),
        "{error:?}"
    );
    let kept: Bytes = buffer_body(Body::from(vec![b'a'; 10]), 10).await.unwrap();
    assert_eq!(kept.len(), 10);
}

// --- The origin --------------------------------------------------------------------------------

#[tokio::test]
async fn csrf_origin_of_the_same_host_is_accepted() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    for (host, origin) in [
        (HOST, "https://recipes.plant.local"),
        (
            "recipes.plant.local:8443",
            "https://recipes.plant.local:8443",
        ),
    ] {
        let headers = session_headers(
            &session,
            &[
                ("host", host),
                ("origin", origin),
                ("x-csrf-token", &session.csrf),
            ],
        );
        let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
        assert_accepted(&fixture, &reply, origin);
    }
}

#[tokio::test]
async fn csrf_fetch_metadata_same_origin_needs_no_origin_header() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let headers = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &session.csrf)]);
    let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
    assert_accepted(&fixture, &reply, "sec-fetch-site");
}

#[tokio::test]
async fn csrf_bad_origins_are_rejected_even_with_a_valid_token() {
    let cases: [(&str, Vec<(&str, &str)>); 9] = [
        ("origin null", vec![("host", HOST), ("origin", "null")]),
        (
            "http scheme",
            vec![("host", HOST), ("origin", "http://recipes.plant.local")],
        ),
        (
            "other port",
            vec![
                ("host", HOST),
                ("origin", "https://recipes.plant.local:8443"),
            ],
        ),
        (
            "port only in host",
            vec![
                ("host", "recipes.plant.local:8443"),
                ("origin", "https://recipes.plant.local"),
            ],
        ),
        (
            "other site",
            vec![("host", HOST), ("origin", "https://evil.example")],
        ),
        (
            "missing host",
            vec![("origin", "https://recipes.plant.local")],
        ),
        ("no origin and no fetch metadata", vec![("host", HOST)]),
        (
            "cross-site metadata",
            vec![("host", HOST), ("sec-fetch-site", "cross-site")],
        ),
        (
            "same-site metadata",
            vec![("host", HOST), ("sec-fetch-site", "same-site")],
        ),
    ];
    for (what, extra) in cases {
        let fixture = Fixture::new();
        let session = fixture.seed_session(1, Role::Administrator, false);
        let mut all = extra;
        all.push(("x-csrf-token", &session.csrf));
        let headers = session_headers(&session, &all);
        let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
        assert_rejected(&fixture, &reply, what);
    }
}

// --- The pre-session token ---------------------------------------------------------------------

/// Build a cookie value and a nonce from fixed bytes and a signing key file.
fn forged(nonce_bytes: [u8; 16], key: &str) -> (String, String) {
    let ring = KeyRing::parse(key).unwrap();
    let nonce = hex(&nonce_bytes);
    let value = format!("{nonce}.{}", hex(&ring.sign(&nonce_bytes)));
    (format!("{PRE_COOKIE}={value}"), nonce)
}

async fn login_with(fixture: &Fixture, cookie: &str, field: &str, extra: &[(&str, &str)]) -> Reply {
    let mut headers = vec![SAME_ORIGIN, ("cookie", cookie)];
    headers.extend_from_slice(extra);
    post(
        &fixture.router(),
        "/test/login",
        &headers,
        &format!("login=anna&csrf_pre={field}"),
    )
    .await
}

#[tokio::test]
async fn csrf_issued_cookie_has_the_attributes_of_api_section_7_3() {
    let fixture = Fixture::new();
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let text = token.set_cookie().to_str().unwrap().to_owned();
    let mut parts = text.split("; ");
    let pair = parts.next().unwrap();
    let mut attributes: Vec<&str> = parts.collect();
    attributes.sort_unstable();
    assert_eq!(
        attributes,
        ["HttpOnly", "Path=/", "SameSite=Strict", "Secure"],
        "{text}"
    );
    let (name, value) = pair.split_once('=').unwrap();
    assert_eq!(name, PRE_COOKIE);
    let (nonce, tag) = value.split_once('.').unwrap();
    assert_eq!(nonce, token.nonce());
    assert_eq!(nonce.len(), 32);
    assert_eq!(tag.len(), 64);
    assert!(
        value
            .bytes()
            .all(|b| b == b'.' || b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    assert!(!text.contains("Max-Age") && !text.contains("Expires") && !text.contains("Domain"));
}

#[tokio::test]
async fn csrf_issued_nonce_is_32_hex_characters_from_a_fallible_source() {
    let fixture = Fixture::new();
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let text = token.set_cookie().to_str().unwrap().to_owned();
    let value = text.split(';').next().unwrap();
    let (_name, pair) = value.split_once('=').unwrap();
    let (nonce, _tag) = pair.split_once('.').unwrap();
    assert_eq!(nonce.len(), 32, "{text}");
    assert!(
        nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    assert_eq!(nonce, token.nonce());
}

#[tokio::test]
async fn csrf_issued_tokens_are_new_each_time_and_signed_under_the_first_key() {
    let fixture = Fixture::new();
    let first = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let second = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    assert_ne!(first.nonce(), second.nonce());
    let pair = cookie_pair(first.set_cookie().to_str().unwrap());
    let value = pair.strip_prefix("__Host-recipe_csrf=").unwrap();
    let (nonce, tag) = value.split_once('.').unwrap();
    let nonce_bytes = unhex(nonce);
    let expected = KeyRing::parse(CSRF_KEY_NEW).unwrap().sign(&nonce_bytes);
    assert_eq!(tag, hex(&expected));
}

#[tokio::test]
async fn csrf_debug_of_the_issued_token_hides_the_nonce() {
    let fixture = Fixture::new();
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let shown = format!("{token:?}");
    assert!(!shown.contains(token.nonce()), "{shown}");
    assert!(shown.contains("[redacted]"), "{shown}");
}

#[tokio::test]
async fn csrf_login_with_an_open_session_accepts_the_pre_session_token_alone() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let both = format!(
        "{}; {}",
        session.cookie,
        cookie_pair(token.set_cookie().to_str().unwrap())
    );
    let body = format!("login=anna&csrf_pre={}", token.nonce());
    let reply = post(
        &fixture.router(),
        "/test/login",
        &[SAME_ORIGIN, ("cookie", &both)],
        &body,
    )
    .await;
    assert_accepted(&fixture, &reply, "login with a session");
}

#[tokio::test]
async fn csrf_login_does_not_accept_the_session_token() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let body = format!("login=anna&csrf_token={}", session.csrf);
    let headers = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &session.csrf)]);
    let reply = post(&fixture.router(), "/test/login", &headers, &body).await;
    assert_rejected(&fixture, &reply, "login with the session token");
}

#[tokio::test]
async fn csrf_forged_cookie_is_rejected() {
    let fixture = Fixture::new();
    let (cookie, nonce) = forged([5; 16], CSRF_KEY_OLD);
    let reply = login_with(&fixture, &cookie, &nonce, &[]).await;
    assert_rejected(&fixture, &reply, "cookie signed by an unknown key");
}

#[tokio::test]
async fn csrf_cookie_signed_by_the_old_key_of_a_two_line_file_is_accepted() {
    let keys = format!("{CSRF_KEY_NEW}\n{CSRF_KEY_OLD}\n");
    let fixture = Fixture::with_settings(WebSettings::default(), &keys);
    let (cookie, nonce) = forged([6; 16], CSRF_KEY_OLD);
    let reply = login_with(&fixture, &cookie, &nonce, &[]).await;
    assert_accepted(&fixture, &reply, "old key");
    let (cookie, nonce) = forged([7; 16], CSRF_KEY_NEW);
    let reply = login_with(&fixture, &cookie, &nonce, &[]).await;
    assert_accepted(&fixture, &reply, "new key");
}

#[tokio::test]
async fn csrf_cookie_with_a_changed_nonce_is_rejected() {
    let fixture = Fixture::new();
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let pair = cookie_pair(token.set_cookie().to_str().unwrap());
    let value = pair.strip_prefix("__Host-recipe_csrf=").unwrap();
    let (nonce, tag) = value.split_once('.').unwrap();
    let flipped = if nonce.starts_with('0') { "1" } else { "0" };
    let changed = format!("{flipped}{}", &nonce[1..]);
    let cookie = format!("{PRE_COOKIE}={changed}.{tag}");
    let reply = login_with(&fixture, &cookie, &changed, &[]).await;
    assert_rejected(&fixture, &reply, "changed nonce");
}

#[tokio::test]
async fn csrf_field_that_differs_from_the_nonce_of_the_cookie_is_rejected() {
    let fixture = Fixture::new();
    let first = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let second = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let cookie = cookie_pair(first.set_cookie().to_str().unwrap());
    let reply = login_with(&fixture, &cookie, second.nonce(), &[]).await;
    assert_rejected(&fixture, &reply, "other nonce");
}

#[tokio::test]
async fn csrf_missing_malformed_or_uppercase_pre_session_parts_are_rejected() {
    let fixture = Fixture::new();
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let pair = cookie_pair(token.set_cookie().to_str().unwrap());
    let value = pair.strip_prefix("__Host-recipe_csrf=").unwrap().to_owned();
    let nonce = token.nonce().to_owned();
    let cases = [
        ("no cookie", String::new(), nonce.clone()),
        ("no field", pair.clone(), String::new()),
        (
            "no dot",
            format!("{PRE_COOKIE}={}", value.replace('.', "")),
            nonce.clone(),
        ),
        (
            "two dots",
            format!("{PRE_COOKIE}={value}.00"),
            nonce.clone(),
        ),
        ("odd hex", format!("{PRE_COOKIE}=zz.{value}"), nonce.clone()),
        (
            "upper case",
            format!("{PRE_COOKIE}={}", value.to_uppercase()),
            nonce.to_uppercase(),
        ),
        ("empty value", format!("{PRE_COOKIE}="), nonce.clone()),
    ];
    for (what, cookie, field) in cases {
        let fixture = Fixture::new();
        let reply = login_with(&fixture, &cookie, &field, &[]).await;
        assert_rejected(&fixture, &reply, what);
    }
}

// --- POST /lang --------------------------------------------------------------------------------

#[tokio::test]
async fn csrf_lang_accepts_the_pre_session_token_without_a_session() {
    let fixture = Fixture::new();
    let reply = crate::test_support::post_pre_session(&fixture, "/lang", "lang=en", &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn csrf_lang_accepts_the_session_token_of_the_open_session() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let router = fixture.router();
    let with_header = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &session.csrf)]);
    let reply = post(&router, "/lang", &with_header, "lang=en").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    let with_field = session_headers(&session, &[SAME_ORIGIN]);
    let body = format!("lang=pl&csrf_token={}", session.csrf);
    let reply = post(&router, "/lang", &with_field, &body).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn csrf_lang_with_a_session_also_accepts_the_pre_session_token() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let cookies = format!(
        "{}; {}",
        session.cookie,
        cookie_pair(token.set_cookie().to_str().unwrap())
    );
    let body = format!("lang=en&csrf_pre={}", token.nonce());
    let reply = post(
        &fixture.router(),
        "/lang",
        &[SAME_ORIGIN, ("cookie", &cookies)],
        &body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
}

#[tokio::test]
async fn csrf_lang_without_a_token_or_with_a_foreign_token_is_rejected() {
    let fixture = Fixture::new();
    let reply = post(&fixture.router(), "/lang", &[SAME_ORIGIN], "lang=en").await;
    assert_rejected(&fixture, &reply, "no token");
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, false);
    let other = fixture.seed_session(2, Role::Operator, false);
    let headers = session_headers(&session, &[SAME_ORIGIN, ("x-csrf-token", &other.csrf)]);
    let reply = post(&fixture.router(), "/lang", &headers, "lang=en").await;
    assert_rejected(&fixture, &reply, "token of another session");
}

#[tokio::test]
async fn csrf_lang_session_token_without_a_session_is_rejected() {
    let fixture = Fixture::new();
    let reply = post(
        &fixture.router(),
        "/lang",
        &[SAME_ORIGIN, ("x-csrf-token", &"ab".repeat(32))],
        &format!("lang=en&csrf_token={}", "ab".repeat(32)),
    )
    .await;
    assert_rejected(&fixture, &reply, "no session");
}

// --- Order, the event, and the log -------------------------------------------------------------

#[tokio::test]
async fn csrf_forged_request_in_read_only_mode_gets_403_and_writes_the_event() {
    let fixture = Fixture::new();
    fixture.state.read_only().set(true);
    let session = fixture.seed_session(1, Role::Administrator, false);
    let headers = session_headers(&session, &[("host", HOST)]);
    let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
    assert_rejected(&fixture, &reply, "read-only");
    assert!(!reply.body.contains("E-WEB-006"));
}

#[tokio::test]
async fn csrf_forged_request_of_a_user_who_must_change_the_password_gets_403() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, true);
    let headers = session_headers(&session, &[("host", HOST)]);
    let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
    assert_rejected(&fixture, &reply, "must change password");
    assert_eq!(reply.header("location"), "");
}

#[tokio::test]
async fn csrf_event_holds_the_request_id_and_the_user_agent() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let headers = session_headers(&session, &[("user-agent", "TestBrowser/1.0")]);
    let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].request_id().to_string(),
        reply.header("x-request-id")
    );
    assert_eq!(
        events[0].user_agent().map(|a| a.as_str()),
        Some("TestBrowser/1.0")
    );
    assert_eq!(events[0].user(), None);
}

#[tokio::test]
async fn csrf_failed_event_write_gives_the_storage_error() {
    let fixture = Fixture::new();
    fixture.sink.fail_with(Fault::Unavailable);
    let session = fixture.seed_session(1, Role::Administrator, false);
    let headers = session_headers(&session, &[("host", HOST)]);
    let reply = post(&fixture.router(), "/test/users", &headers, "a=b").await;
    assert_eq!(
        reply.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-DB-005"), "{}", reply.body);
    assert!(!reply.body.contains("E-WEB-007"), "{}", reply.body);
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn csrf_rejection_logs_one_warning_and_no_token() {
    let (logs, _guard) = capture_logs();
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let pre_cookie = cookie_pair(token.set_cookie().to_str().unwrap());
    let cookies = format!("{}; {pre_cookie}", session.cookie);
    let wrong_header = "f".repeat(64);
    let headers = [
        ("cookie", cookies.as_str()),
        ("x-csrf-token", wrong_header.as_str()),
    ];
    let body = format!("csrf_token={}&csrf_pre={}", "e".repeat(64), token.nonce());
    let reply = post(&fixture.router(), "/test/users", &headers, &body).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let text = logs.text();
    assert!(text.contains("WARN"), "{text}");
    assert!(text.contains("error_code=E-WEB-007"), "{text}");
    assert_eq!(text.matches("request failed").count(), 1, "{text}");
    for secret in [
        session.csrf.as_str(),
        session.cookie.as_str(),
        wrong_header.as_str(),
        "e".repeat(64).as_str(),
        token.nonce(),
        pre_cookie.as_str(),
    ] {
        assert!(!text.contains(secret), "{secret} in {text}");
    }
}
