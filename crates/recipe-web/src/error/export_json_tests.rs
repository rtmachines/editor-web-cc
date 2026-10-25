//! Tests of the JSON error answers on the export paths (design D11 row T10, ADR-0053).
//!
//! The test routes live only in this file. One route under `/export/v1/` raises an error of each
//! family. One route needs a session, so the session layer and the gates raise their errors. One
//! route panics, so the inner panic layer gives `E-WEB-010` (ADR-0057 (b)). One route outlasts the
//! request timeout and still gets the answer of its handler, because the timeout skips the export
//! paths (ADR-0072 (a)).

use axum::Router;
use axum::extract::connect_info::MockConnectInfo;
use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::routing::{MethodRouter, get};
use recipe_app::auth::Operation;
use recipe_app::error::{AppError, LockFailure, Missing, StorageError, TaskStage, TimeoutStage};
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::ExportError;
use recipe_domain::identity::{IdentityError, Role};
use recipe_domain::recipe::{RecipeError, ReferenceNumber};
use tokio::time::Instant;
use uuid::Uuid;

use super::{ByteLimit, FieldReason, WebError};
use crate::WebState;
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{Fixture, PEER, REACHED, Reply, call, capture_logs, get_with, request};

/// The `Content-Security-Policy` of `api.md` section 9.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";
/// The header of an HTMX request.
const HTMX: (&str, &str) = ("hx-request", "true");
/// The cookie of the UI language `en`.
const ENGLISH: (&str, &str) = ("cookie", "recipe_lang=en");

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-100").unwrap()
}

fn app(error: impl Into<AppError>) -> WebError {
    WebError::App(error.into())
}

fn constraint(code: &str) -> StorageError {
    StorageError::Constraint {
        code: ErrorCode::find(code).unwrap(),
        message: "The rule of the database stopped the change.".to_owned(),
    }
}

/// Return the error of an export route for `case`.
fn export_error_for(case: &str) -> WebError {
    match case {
        "exp-001" => app(ExportError::NoApprovedVersion {
            reference: reference(),
        }),
        "exp-002" => app(ExportError::ContentHashMismatch {
            reference: reference(),
            number: 2,
        }),
        "exp-003" => app(ExportError::VersionNotExportable {
            reference: reference(),
            number: 2,
        }),
        "exp-004" => app(ExportError::Serialize(serde::ser::Error::custom(
            "the serializer failed",
        ))),
        "deleted" => app(RecipeError::RecipeDeleted {
            reference: reference(),
        }),
        "forbidden" => app(AppError::Forbidden {
            operation: Operation::ExportVersionByNumber,
        }),
        "not-found" => app(AppError::NotFound {
            entity: Missing::Recipe(reference()),
        }),
        other => error_for(other),
    }
}

/// Return the error of the families that every route can return.
fn error_for(case: &str) -> WebError {
    match case {
        "csrf" => WebError::CsrfRejected,
        "read-only" => WebError::ReadOnlyMode,
        "timeout" => app(AppError::Timeout {
            stage: TimeoutStage::Request,
        }),
        "deadlock" => app(StorageError::Deadlock {
            cause: LockFailure::Deadlock,
        }),
        "task-failed" => app(AppError::TaskFailed {
            stage: TaskStage::Export,
        }),
        "storage-other" => app(StorageError::Other {
            sqlstate: Some("XX000".to_owned()),
            source: Box::new(std::io::Error::other("secret internal text")),
        }),
        "db-rule" => app(constraint("E-DB-003")),
        "throttled" => app(IdentityError::LoginThrottled { retry_after: 12 }),
        "inactive" => app(IdentityError::UserInactive),
        "too-large" => WebError::PayloadTooLarge {
            limit: ByteLimit::from(256 * 1024),
        },
        "field" => WebError::BadRequest {
            field: "lang",
            reason: FieldReason::Lang,
        },
        _internal => WebError::Internal {
            request_id: RequestId::from(Uuid::nil()),
        },
    }
}

async fn raise(Path(case): Path<String>) -> Result<&'static str, WebError> {
    Err(export_error_for(&case))
}

async fn reached() -> &'static str {
    REACHED
}

/// The message of the test panic. No answer may hold it.
const PANIC_TEXT: &str = "the test value is missing";

/// A handler that panics. Only test code may call `expect` (`reliability.md` section 2.3).
///
/// `black_box` hides the `None` from the optimizer and from clippy.
async fn explode() -> &'static str {
    std::hint::black_box(None::<&'static str>).expect(PANIC_TEXT)
}

/// A handler that outlasts the request timeout. The test clock is paused, so it costs no real time.
async fn outlast_the_timeout(State(state): State<WebState>) -> &'static str {
    let wait = state.settings().request_timeout.saturating_mul(2);
    tokio::time::sleep(wait).await;
    REACHED
}

fn row(path: &'static str, access: Access, handler: MethodRouter<WebState>) -> RouteEntry {
    RouteEntry {
        method: Method::GET,
        path,
        access,
        handler,
    }
}

/// Build the router with the five test routes and a client at `PEER`.
///
/// `/export/v10/{case}` is outside the prefix `/export/v1/`. It proves that the prefix ends with `/`.
fn router(fixture: &Fixture) -> Router {
    let rows = vec![
        row("/export/v1/test/{case}", Access::Static, get(raise)),
        row("/export/v1/panic", Access::Static, get(explode)),
        row("/export/v1/slow", Access::Static, get(outlast_the_timeout)),
        row(
            "/export/v1/session/{case}",
            Access::Operation(Operation::ExportApprovedVersion),
            get(reached),
        ),
        row("/export/v10/{case}", Access::Static, get(raise)),
    ];
    assemble(fixture.state.clone(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

async fn ask(fixture: &Fixture, path: &str, headers: &[(&str, &str)]) -> Reply {
    get_with(&router(fixture), path, headers).await
}

/// The three fields of a JSON error body.
struct Body {
    code: String,
    message: String,
    request_id: String,
}

/// Read the body `{"code":…,"message":…,"request_id":…}` with the fields in this order and no other field.
fn json_of(reply: &Reply) -> Body {
    let body = &reply.body;
    assert!(
        body.starts_with("{\"code\":\""),
        "not a JSON error body: {body}"
    );
    let rest = body.strip_prefix("{\"code\":\"").unwrap();
    let (code, rest) = rest.split_once("\",\"message\":\"").unwrap();
    let (message, rest) = rest.split_once("\",\"request_id\":\"").unwrap();
    let request_id = rest.strip_suffix("\"}").unwrap();
    Body {
        code: code.to_owned(),
        message: message.to_owned(),
        request_id: request_id.to_owned(),
    }
}

/// Check the headers of `api.md` section 9 that an export answer needs (design D11 row T10).
fn assert_secure(reply: &Reply, case: &str) {
    assert_eq!(reply.header("cache-control"), "no-store", "{case}");
    assert_eq!(reply.header("x-content-type-options"), "nosniff", "{case}");
    assert_eq!(reply.header("content-security-policy"), CSP, "{case}");
}

/// Check that the answer has no `HX-*` header (ADR-0053 (c)).
fn assert_no_htmx_header(reply: &Reply, case: &str) {
    let names: Vec<&str> = reply.headers.keys().map(|name| name.as_str()).collect();
    assert!(
        names.iter().all(|name| !name.starts_with("hx-")),
        "{case}: {names:?}"
    );
}

/// Check one JSON error answer and return its body.
fn assert_json(reply: &Reply, status: StatusCode, code: &str, case: &str) -> Body {
    assert_eq!(reply.status, status, "{case}: {}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json", "{case}");
    assert_secure(reply, case);
    assert_no_htmx_header(reply, case);
    let body = json_of(reply);
    assert_eq!(body.code, code, "{case}");
    assert_eq!(body.request_id, reply.header("x-request-id"), "{case}");
    assert!(!body.message.is_empty(), "{case}");
    body
}

/// The cases of the families: the case, the status, and the code of the body.
const FAMILIES: [(&str, StatusCode, &str); 19] = [
    ("exp-001", StatusCode::NOT_FOUND, "E-EXP-001"),
    ("exp-002", StatusCode::INTERNAL_SERVER_ERROR, "E-EXP-002"),
    ("exp-003", StatusCode::CONFLICT, "E-EXP-003"),
    ("exp-004", StatusCode::INTERNAL_SERVER_ERROR, "E-EXP-004"),
    ("deleted", StatusCode::CONFLICT, "E-RCP-012"),
    ("forbidden", StatusCode::FORBIDDEN, "E-WEB-002"),
    ("not-found", StatusCode::NOT_FOUND, "E-WEB-005"),
    ("csrf", StatusCode::FORBIDDEN, "E-WEB-007"),
    ("read-only", StatusCode::SERVICE_UNAVAILABLE, "E-WEB-006"),
    ("timeout", StatusCode::SERVICE_UNAVAILABLE, "E-WEB-009"),
    ("deadlock", StatusCode::SERVICE_UNAVAILABLE, "E-DB-001"),
    (
        "task-failed",
        StatusCode::INTERNAL_SERVER_ERROR,
        "E-WEB-011",
    ),
    (
        "storage-other",
        StatusCode::INTERNAL_SERVER_ERROR,
        "E-DB-008",
    ),
    ("db-rule", StatusCode::CONFLICT, "E-DB-003"),
    ("throttled", StatusCode::TOO_MANY_REQUESTS, "E-IDN-005"),
    ("inactive", StatusCode::UNAUTHORIZED, "E-IDN-003"),
    ("too-large", StatusCode::PAYLOAD_TOO_LARGE, "E-WEB-003"),
    ("field", StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008"),
    ("internal", StatusCode::INTERNAL_SERVER_ERROR, "E-WEB-010"),
];

#[tokio::test]
async fn export_json_each_error_family_is_a_json_body_in_field_order() {
    let fixture = Fixture::new();
    for (case, status, code) in FAMILIES {
        let path = format!("/export/v1/test/{case}");
        let reply = ask(&fixture, &path, &[]).await;
        assert_json(&reply, status, code, case);
    }
}

/// A panic in an export handler gives the JSON body of `E-WEB-010` (ADR-0057 (b), ADR-0053 (a)).
#[tokio::test]
async fn export_json_a_panic_in_the_handler_is_the_json_body_of_e_web_010() {
    let fixture = Fixture::new();
    let reply = ask(&fixture, "/export/v1/panic", &[ENGLISH]).await;
    let body = assert_json(
        &reply,
        StatusCode::INTERNAL_SERVER_ERROR,
        "E-WEB-010",
        "panic",
    );
    assert_eq!(
        body.message,
        "An internal error occurred. Contact the administrator."
    );
    assert!(!reply.body.contains(PANIC_TEXT), "{}", reply.body);
}

/// A handler that outlasts the request timeout still gets its own answer on an export path.
///
/// The request timeout skips the export paths (ADR-0072 (a)). The test clock is paused, so the
/// answer comes after twice the limit and costs no real time (ADR-0072 (e)).
#[tokio::test(start_paused = true)]
async fn export_json_a_slow_handler_outlasts_the_request_timeout() {
    let fixture = Fixture::new();
    let limit = fixture.state.settings().request_timeout;
    let started = Instant::now();
    let reply = ask(&fixture, "/export/v1/slow", &[ENGLISH]).await;
    let waited = limit.saturating_mul(2);
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body, REACHED);
    assert!(started.elapsed() >= waited, "{:?}", started.elapsed());
}

/// An HTMX request gets the `403 E-WEB-007` of the guard before the handler (ADR-0053 (c),
/// ADR-0055 (c)). Each request writes one security event.
#[tokio::test]
async fn export_json_an_htmx_request_gets_the_guard_answer_as_json_and_no_htmx_header() {
    let fixture = Fixture::new();
    for (count, (case, _status, _code)) in FAMILIES.into_iter().enumerate() {
        let path = format!("/export/v1/test/{case}");
        let reply = ask(&fixture, &path, &[HTMX]).await;
        assert_json(&reply, StatusCode::FORBIDDEN, "E-WEB-007", case);
        let events = count.checked_add(1).unwrap();
        assert_eq!(fixture.sink.events().len(), events, "{case}");
    }
}

/// The sentences of the codes with arguments, in `pl` and `en` (design D9, `i18n/*.ftl`).
const SENTENCES: [(&str, &str, &str); 5] = [
    (
        "exp-001",
        "Receptura R-100 nie ma wersji zatwierdzonej.",
        "Recipe R-100 has no approved version.",
    ),
    (
        "exp-002",
        "Wersja 2 receptury R-100 nie zgadza się z zapisanym skrótem. Eksport jest odrzucony. Skontaktuj się z administratorem.",
        "Version 2 of recipe R-100 does not match its stored hash. The export is refused. Contact the administrator.",
    ),
    (
        "exp-003",
        "Wersja 2 receptury R-100 jest szkicem albo została odrzucona. Eksportować można tylko wersje zatwierdzone i wycofane.",
        "Version 2 of recipe R-100 is a draft or was discarded. Only approved and retired versions can be exported.",
    ),
    (
        "deleted",
        "Receptura R-100 jest usunięta. Administrator może ją przywrócić.",
        "Recipe R-100 is deleted. An administrator can restore it.",
    ),
    (
        "internal",
        "Wystąpił błąd wewnętrzny. Skontaktuj się z administratorem.",
        "An internal error occurred. Contact the administrator.",
    ),
];

#[tokio::test]
async fn export_json_the_message_is_the_sentence_of_the_code_in_the_ui_language() {
    let fixture = Fixture::new();
    for (case, polish, english) in SENTENCES {
        let path = format!("/export/v1/test/{case}");
        let pl = json_of(&ask(&fixture, &path, &[]).await);
        assert_eq!(pl.message, polish, "{case}");
        let en = json_of(&ask(&fixture, &path, &[ENGLISH]).await);
        assert_eq!(en.message, english, "{case}");
    }
}

#[tokio::test]
async fn export_json_the_message_holds_no_sentence_with_the_request_id() {
    let fixture = Fixture::new();
    for (case, _status, _code) in FAMILIES {
        let path = format!("/export/v1/test/{case}");
        for headers in [vec![], vec![ENGLISH]] {
            let reply = ask(&fixture, &path, &headers).await;
            let body = json_of(&reply);
            assert!(!body.message.contains(&body.request_id), "{case}");
            assert!(!body.message.contains("Reference:"), "{case}");
            assert!(!body.message.contains("Numer referencyjny"), "{case}");
        }
    }
}

#[tokio::test]
async fn export_json_retry_after_stays_on_503_and_429() {
    let fixture = Fixture::new();
    let seconds = fixture.state.settings().retry_after.as_secs().to_string();
    for (case, expected) in [
        ("read-only", seconds.as_str()),
        ("timeout", seconds.as_str()),
        ("throttled", "12"),
    ] {
        let reply = ask(&fixture, &format!("/export/v1/test/{case}"), &[]).await;
        assert_eq!(reply.header("retry-after"), expected, "{case}");
    }
}

#[tokio::test]
async fn export_json_no_session_redirects_to_login_and_ends_the_cookie() {
    let fixture = Fixture::new();
    for headers in [vec![], vec![HTMX]] {
        let reply = ask(&fixture, "/export/v1/session/x", &headers).await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{headers:?}");
        assert_eq!(reply.header("location"), "/login", "{headers:?}");
        assert!(reply.body.is_empty(), "{}", reply.body);
        let cookies = reply.set_cookies();
        assert!(
            cookies
                .iter()
                .any(|cookie| cookie.starts_with("__Host-recipe_session=")
                    && cookie.contains("Max-Age=0")),
            "{cookies:?}"
        );
        assert_secure(&reply, "E-WEB-004");
        assert_no_htmx_header(&reply, "E-WEB-004");
    }
}

#[tokio::test]
async fn export_json_forced_password_change_redirects_to_the_password_page() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Operator, true);
    for extra in [vec![], vec![HTMX]] {
        let mut headers = vec![("cookie", session.cookie.as_str())];
        headers.extend(extra);
        let reply = ask(&fixture, "/export/v1/session/x", &headers).await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{headers:?}");
        assert_eq!(reply.header("location"), "/me/password", "{headers:?}");
        assert!(reply.body.is_empty(), "{}", reply.body);
        assert_secure(&reply, "E-IDN-007");
        assert_no_htmx_header(&reply, "E-IDN-007");
    }
}

#[tokio::test]
async fn export_json_the_router_404_text_stays_for_an_unknown_path_and_an_unlisted_method() {
    let fixture = Fixture::new();
    let unknown = ask(&fixture, "/export/v1/unknown/path/here", &[]).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.body, "Not found");
    let request = request(Method::PUT, "/export/v1/test/exp-001")
        .body(axum::body::Body::empty())
        .unwrap();
    let unlisted = call(&router(&fixture), request).await;
    assert_eq!(unlisted.status, StatusCode::NOT_FOUND);
    assert_eq!(unlisted.body, "Not found");
}

#[tokio::test]
async fn export_json_a_path_outside_the_prefix_keeps_the_html_answer() {
    let fixture = Fixture::new();
    let reply = ask(&fixture, "/export/v10/exp-001", &[]).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert!(
        reply.header("content-type").starts_with("text/html"),
        "{}",
        reply.header("content-type")
    );
    assert!(reply.body.contains("E-EXP-001"), "{}", reply.body);
}

#[tokio::test]
async fn export_json_the_error_has_one_log_line_with_its_code_and_request_id() {
    let fixture = Fixture::new();
    for (case, logged_code) in [("exp-002", "E-EXP-002"), ("inactive", "E-IDN-004")] {
        let (logs, _guard) = capture_logs();
        let reply = ask(&fixture, &format!("/export/v1/test/{case}"), &[]).await;
        let text = logs.text();
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| line.contains("request failed"))
            .collect();
        assert_eq!(lines.len(), 1, "{case}: {lines:?}");
        let line = lines.first().unwrap();
        assert!(
            line.contains(&format!("error_code={logged_code}")),
            "{line}"
        );
        assert!(
            line.contains(&format!("request_id={}", reply.header("x-request-id"))),
            "{line}"
        );
    }
}
