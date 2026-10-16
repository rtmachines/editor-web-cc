//! Tests of the body limit of each route group (task 8.2, design D11, `api.md` section 7.6).
//!
//! The router of these tests holds stub handlers on the real path patterns of the recipe routes.
//! So the tests check the limit layer and the CSRF buffer without the pages of waves 4 and 5.

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::routing::post;
use recipe_app::auth::Operation;
use recipe_domain::identity::Role;

use super::{largest_limit, route_limit};
use crate::routes::{RouteEntry, assemble, operation_row, table};
use crate::settings::WebSettings;
use crate::test_support::{Fixture, PEER, Reply, TestSession, call, request};

/// The path patterns of the four routes that carry a parameter value.
pub(crate) const VALUE_PATTERNS: [&str; 4] = [
    "/recipes/{ref}/versions/{n}/parameters",
    "/recipes/{ref}/versions/{n}/parameters/{key}/definition",
    "/recipes/{ref}/versions/{n}/parameters/{key}/value",
    "/recipes/{ref}/versions/{n}/rebind",
];
/// The request paths that match [`VALUE_PATTERNS`], in the same order.
pub(crate) const VALUE_PATHS: [&str; 4] = [
    "/recipes/R-100/versions/3/parameters",
    "/recipes/R-100/versions/3/parameters/0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10/definition",
    "/recipes/R-100/versions/3/parameters/0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10/value",
    "/recipes/R-100/versions/3/rebind",
];
/// The path pattern of the route that writes a translation.
pub(crate) const TRANSLATION_PATTERN: &str =
    "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}";
/// The request path that matches [`TRANSLATION_PATTERN`].
pub(crate) const TRANSLATION_PATH: &str =
    "/recipes/R-100/versions/3/parameters/0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10/translations/pl";
/// The path patterns of routes with the limit of an HTML form, also next to the routes above.
const FORM_PATTERNS: [&str; 4] = [
    "/recipes",
    "/recipes/{ref}/rename",
    "/recipes/{ref}/versions/{n}/parameters/{key}/delete",
    "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}/delete",
];
/// The request paths that match [`FORM_PATTERNS`], in the same order.
pub(crate) const FORM_PATHS: [&str; 4] = [
    "/recipes",
    "/recipes/R-100/rename",
    "/recipes/R-100/versions/3/parameters/0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10/delete",
    "/recipes/R-100/versions/3/parameters/0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10/translations/pl/delete",
];

const KIB: usize = 1024;

/// The stub handler. It reads the whole body, so the limit of the extractor applies too.
async fn length_of(body: Bytes) -> String {
    body.len().to_string()
}

/// Return the stub rows: one `POST` row for each value route, the translation route, and four forms.
fn stub_rows() -> Vec<RouteEntry> {
    let row = |path: &'static str, operation: Operation| {
        operation_row(Method::POST, path, operation, post(length_of))
    };
    let mut rows: Vec<RouteEntry> = VALUE_PATTERNS
        .iter()
        .copied()
        .map(|path| row(path, Operation::AddParameter))
        .collect();
    rows.push(row(TRANSLATION_PATTERN, Operation::UpsertTranslation));
    rows.extend(
        FORM_PATTERNS
            .iter()
            .copied()
            .map(|path| row(path, Operation::RenameRecipe)),
    );
    rows
}

/// A router over the stub rows, and the session of an Administrator.
pub(crate) fn limit_router() -> (Router, TestSession) {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let router =
        assemble(fixture.state.clone(), stub_rows(), Router::new()).layer(MockConnectInfo(PEER));
    (router, session)
}

/// Send a same-origin form `POST` with exactly the given headers and the given body.
pub(crate) async fn send_with(
    router: &Router,
    path: &str,
    (headers, body): (&[(&str, &str)], Body),
) -> Reply {
    let mut builder = request(Method::POST, path)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    call(router, builder.body(body).unwrap()).await
}

/// Send a `POST` with the session cookie, the session token in the header, the extra headers, and
/// the given body.
pub(crate) async fn send_body(
    router: &Router,
    session: &TestSession,
    path: &str,
    (extra, body): (&[(&str, &str)], Body),
) -> Reply {
    let mut headers = vec![
        ("cookie", session.cookie.as_str()),
        ("x-csrf-token", session.csrf.as_str()),
    ];
    headers.extend_from_slice(extra);
    send_with(router, path, (&headers, body)).await
}

/// Build a body of `length` bytes whose size the request does not announce, as a chunked body.
pub(crate) fn chunked(length: usize) -> Body {
    Body::from_stream(Body::from(vec![b'a'; length]).into_data_stream())
}

fn assert_too_large(reply: &Reply, limit_text: &str, what: &str) {
    assert_eq!(
        reply.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "{what}: {}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-003"), "{what}: {}", reply.body);
    assert!(reply.body.contains(limit_text), "{what}: {}", reply.body);
}

fn assert_accepted(reply: &Reply, length: usize, what: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
    assert_eq!(reply.body, length.to_string(), "{what}");
}

// --- The choice of the limit ---------------------------------------------------------------------

#[test]
fn limits_route_limit_gives_the_value_limit_to_the_four_value_routes() {
    let settings = WebSettings::default();
    for pattern in VALUE_PATTERNS {
        assert_eq!(
            route_limit(&settings, &Method::POST, pattern),
            settings.body_limit_value,
            "{pattern}"
        );
    }
}

#[test]
fn limits_route_limit_gives_the_translation_limit_to_the_translation_route_only() {
    let settings = WebSettings::default();
    assert_eq!(
        route_limit(&settings, &Method::POST, TRANSLATION_PATTERN),
        settings.body_limit_translation
    );
    let delete = "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}/delete";
    assert_eq!(
        route_limit(&settings, &Method::POST, delete),
        settings.body_limit_form
    );
}

#[test]
fn limits_route_limit_gives_the_form_limit_to_every_other_route() {
    let settings = WebSettings::default();
    for pattern in FORM_PATTERNS {
        assert_eq!(
            route_limit(&settings, &Method::POST, pattern),
            settings.body_limit_form,
            "{pattern}"
        );
    }
    for pattern in ["/login", "/users/{login}/role", "/machines/{code}/types"] {
        assert_eq!(
            route_limit(&settings, &Method::POST, pattern),
            settings.body_limit_form,
            "{pattern}"
        );
    }
}

#[test]
fn limits_route_limit_gives_the_form_limit_to_a_method_that_carries_no_value() {
    let settings = WebSettings::default();
    for pattern in VALUE_PATTERNS {
        assert_eq!(
            route_limit(&settings, &Method::GET, pattern),
            settings.body_limit_form,
            "{pattern}"
        );
    }
}

#[test]
fn limits_largest_limit_is_the_value_limit_of_the_table() {
    let settings = WebSettings::default();
    assert_eq!(largest_limit(&settings), 4 * KIB * KIB);
    let small = WebSettings {
        body_limit_form: 10,
        body_limit_value: 5,
        body_limit_translation: 7,
        ..WebSettings::default()
    };
    assert_eq!(largest_limit(&small), 10);
}

// --- The link to the real route table -------------------------------------------------------------

/// Return the patterns of `paths` whose limit does not follow the shape of the path.
///
/// A `POST` path under `/recipes/{ref}/versions/{n}` that ends with `/parameters`, `/definition`,
/// `/value`, or `/rebind` must get the value limit. A path that ends with `/translations/{lang}`
/// must get the translation limit. No other path gets one of the two.
fn limit_mismatches(paths: &[&str]) -> Vec<String> {
    let settings = WebSettings::default();
    let mut found = Vec::new();
    for path in paths {
        let under_version = path.starts_with("/recipes/") && path.contains("/versions/");
        let value = under_version
            && ["/parameters", "/definition", "/value", "/rebind"]
                .iter()
                .any(|end| path.ends_with(end));
        let translation = path.ends_with("/translations/{lang}");
        let limit = route_limit(&settings, &Method::POST, path);
        if (limit == settings.body_limit_value) != value {
            found.push(format!("{path}: value limit"));
        }
        if (limit == settings.body_limit_translation) != translation {
            found.push(format!("{path}: translation limit"));
        }
    }
    found
}

/// This test guards waves 4 and 5. They add the real `POST` rows of the recipe routes to the
/// table. A route that carries a value or a translation under another pattern than the five
/// patterns of `limits.rs` would fall back to the form limit. This test then fails.
#[test]
fn limits_every_post_row_of_the_route_table_has_the_limit_of_its_path_shape() {
    let rows = table();
    let paths: Vec<&str> = rows
        .iter()
        .filter(|row| row.method == Method::POST)
        .map(|row| row.path)
        .collect();
    assert_eq!(limit_mismatches(&paths), Vec::<String>::new());
}

#[test]
fn limits_the_check_of_the_table_accepts_the_real_patterns_and_finds_a_wrong_one() {
    assert!(limit_mismatches(&VALUE_PATTERNS).is_empty());
    assert!(limit_mismatches(&[TRANSLATION_PATTERN]).is_empty());
    assert!(limit_mismatches(&FORM_PATTERNS).is_empty());
    let wrong = [
        "/recipes/{id}/versions/{n}/parameters",
        "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{code}/value",
        "/recipes/{ref}/versions/{n}/parameters/{name}/translations/{lang}",
    ];
    for path in wrong {
        assert!(!limit_mismatches(&[path]).is_empty(), "{path}");
    }
}

// --- The value routes ----------------------------------------------------------------------------

#[tokio::test]
async fn limits_a_value_route_accepts_a_body_above_the_form_limit_up_to_4_mib() {
    let (router, session) = limit_router();
    for (limit, path) in [(300 * KIB, VALUE_PATHS[0]), (4 * KIB * KIB, VALUE_PATHS[2])] {
        let body = Body::from("a".repeat(limit));
        let reply = send_body(&router, &session, path, (&[], body)).await;
        assert_accepted(&reply, limit, path);
    }
}

#[tokio::test]
async fn limits_each_value_route_refuses_a_body_above_4_mib() {
    let (router, session) = limit_router();
    for path in VALUE_PATHS {
        let body = Body::from("a".repeat(4 * KIB * KIB + 1));
        let reply = send_body(&router, &session, path, (&[], body)).await;
        assert_too_large(&reply, "4 MiB", path);
    }
}

#[tokio::test]
async fn limits_a_declared_length_above_the_value_limit_gives_e_web_003() {
    let (router, session) = limit_router();
    let declared = (4 * KIB * KIB + 1).to_string();
    let headers = [("content-length", declared.as_str())];
    let reply = send_body(&router, &session, VALUE_PATHS[1], (&headers, Body::empty())).await;
    assert_too_large(&reply, "4 MiB", "declared");
}

#[tokio::test]
async fn limits_a_declared_length_above_the_form_limit_passes_on_a_value_route() {
    let (router, session) = limit_router();
    let declared = (300 * KIB).to_string();
    let headers = [("content-length", declared.as_str())];
    let reply = send_body(&router, &session, VALUE_PATHS[3], (&headers, Body::empty())).await;
    assert_ne!(
        reply.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "{}",
        reply.body
    );
}

// --- The translation route -----------------------------------------------------------------------

#[tokio::test]
async fn limits_the_translation_route_accepts_64_kib_and_refuses_one_byte_more() {
    let (router, session) = limit_router();
    let body = Body::from("a".repeat(64 * KIB));
    let reply = send_body(&router, &session, TRANSLATION_PATH, (&[], body)).await;
    assert_accepted(&reply, 64 * KIB, "64 KiB");
    let body = Body::from("a".repeat(64 * KIB + 1));
    let reply = send_body(&router, &session, TRANSLATION_PATH, (&[], body)).await;
    assert_too_large(&reply, "64 KiB", "64 KiB + 1");
}

// --- The other routes ----------------------------------------------------------------------------

#[tokio::test]
async fn limits_a_form_route_accepts_256_kib_and_refuses_one_byte_more() {
    let (router, session) = limit_router();
    for path in FORM_PATHS {
        let body = Body::from("a".repeat(256 * KIB));
        let reply = send_body(&router, &session, path, (&[], body)).await;
        assert_accepted(&reply, 256 * KIB, path);
        let body = Body::from("a".repeat(256 * KIB + 1));
        let reply = send_body(&router, &session, path, (&[], body)).await;
        assert_too_large(&reply, "256 KiB", path);
    }
}

// --- A chunked body ------------------------------------------------------------------------------

#[test]
fn limits_a_chunked_body_announces_no_length() {
    use axum::body::HttpBody as _;
    let body = chunked(10);
    assert_eq!(body.size_hint().upper(), None);
}

#[tokio::test]
async fn limits_a_chunked_body_above_the_value_limit_gives_e_web_003() {
    let (router, session) = limit_router();
    let headers = [("transfer-encoding", "chunked")];
    for path in VALUE_PATHS {
        let body = chunked(4 * KIB * KIB + 1);
        let reply = send_body(&router, &session, path, (&headers, body)).await;
        assert_too_large(&reply, "4 MiB", path);
    }
}

#[tokio::test]
async fn limits_a_chunked_body_up_to_the_limit_of_its_route_passes() {
    let (router, session) = limit_router();
    let headers = [("transfer-encoding", "chunked")];
    let cases = [
        (VALUE_PATHS[0], 300 * KIB),
        (VALUE_PATHS[2], 4 * KIB * KIB),
        (TRANSLATION_PATH, 64 * KIB),
        (FORM_PATHS[1], 256 * KIB),
    ];
    for (path, length) in cases {
        let reply = send_body(&router, &session, path, (&headers, chunked(length))).await;
        assert_accepted(&reply, length, path);
    }
}

#[tokio::test]
async fn limits_a_chunked_body_above_the_translation_or_form_limit_gives_e_web_003() {
    let (router, session) = limit_router();
    let headers = [("transfer-encoding", "chunked")];
    let cases = [
        (TRANSLATION_PATH, 64 * KIB + 1, "64 KiB"),
        (FORM_PATHS[0], 256 * KIB + 1, "256 KiB"),
        (FORM_PATHS[3], 256 * KIB + 1, "256 KiB"),
    ];
    for (path, length, text) in cases {
        let reply = send_body(&router, &session, path, (&headers, chunked(length))).await;
        assert_too_large(&reply, text, path);
    }
}

#[tokio::test]
async fn limits_a_chunked_body_above_the_limit_never_reaches_the_handler() {
    let (router, session) = limit_router();
    let reply = send_body(
        &router,
        &session,
        VALUE_PATHS[0],
        (&[], chunked(4 * KIB * KIB + 1)),
    )
    .await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(
        reply.body.parse::<usize>().is_err(),
        "the handler answered: {}",
        reply.body
    );
}
