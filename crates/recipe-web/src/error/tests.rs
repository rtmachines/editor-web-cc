use std::io;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::Path;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use axum::routing::get;
use parking_lot::Mutex;
use recipe_app::auth::Operation;
use recipe_app::error::{AppError, LockFailure, Missing, StorageError, TaskStage, TimeoutStage};
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{CredentialFailure, IdentityError, Login, PasswordReason};
use tower::ServiceExt as _;
use tracing_subscriber::fmt::MakeWriter;
use uuid::Uuid;

use super::*;
use crate::routes::{Access, RouteEntry, assemble, table};
use crate::test_support::{PEER, state};

const BODY_LIMIT: usize = 1024 * 1024;
const HTMX_CONFIG: &str = r#"{"allowEval":false,"allowScriptTags":false,"selfRequestsOnly":true,"historyCacheSize":0,"methodsThatUseUrlParams":["get"],"includeIndicatorStyles":false,"responseHandling":[{"code":"204","swap":false},{"code":"[23]..","swap":true},{"code":"[45]..","swap":true,"error":true},{"code":"...","swap":false}]}"#;

fn login() -> Login {
    Login::parse("anna").unwrap()
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

/// Return the error that the test route raises for `case`.
fn error_for(case: &str) -> WebError {
    match case {
        "field" => WebError::BadRequest {
            field: "lang",
            reason: FieldReason::Lang,
        },
        "policy" => app(IdentityError::PasswordPolicy {
            reason: PasswordReason::TooShort,
        }),
        "last-admin" => app(IdentityError::LastAdministrator { login: login() }),
        "db-rule" => app(constraint("E-DB-003")),
        "trigger" => app(constraint("E-PLT-030")),
        "forbidden" => app(AppError::Forbidden {
            operation: Operation::CreateMachine,
        }),
        "forbidden-users" => app(AppError::Forbidden {
            operation: Operation::ListUsers,
        }),
        "csrf" => WebError::CsrfRejected,
        "unauthenticated" => app(AppError::Unauthenticated),
        "credentials" => app(IdentityError::InvalidCredentials {
            cause: CredentialFailure::UnknownLogin,
        }),
        "inactive" => app(IdentityError::UserInactive),
        "current-password" => app(IdentityError::InvalidCurrentPassword),
        "must-change" => app(IdentityError::MustChangePassword),
        "not-found" => app(AppError::NotFound {
            entity: Missing::User(login()),
        }),
        other => more_error_for(other),
    }
}

/// Return the error of the cases that `error_for` does not hold.
fn more_error_for(case: &str) -> WebError {
    match case {
        // A trigger code without a key in the bundles: the message falls back to English.
        "trigger-audit" => app(constraint("E-AUD-003")),
        "too-large" => WebError::PayloadTooLarge {
            limit: ByteLimit::from(256 * 1024),
        },
        "throttled" => app(IdentityError::LoginThrottled { retry_after: 12 }),
        "read-only" => WebError::ReadOnlyMode,
        "timeout" => app(AppError::Timeout {
            stage: TimeoutStage::Request,
        }),
        "deadlock" => app(StorageError::Deadlock {
            cause: LockFailure::Deadlock,
        }),
        "task-failed" => app(AppError::TaskFailed {
            stage: TaskStage::PasswordHash,
        }),
        "storage-other" => app(StorageError::Other {
            sqlstate: Some("XX000".to_owned()),
            source: Box::new(std::io::Error::other("secret internal text")),
        }),
        "internal" => WebError::Internal {
            request_id: RequestId::from(Uuid::nil()),
        },
        _unknown => WebError::BadRequest {
            field: "case",
            reason: FieldReason::Form,
        },
    }
}

async fn raise(Path(case): Path<String>) -> Result<&'static str, WebError> {
    Err(error_for(&case))
}

fn test_router() -> Router {
    let mut entries = table();
    entries.push(RouteEntry {
        method: Method::GET,
        path: "/test/{case}",
        access: Access::Static,
        handler: get(raise),
    });
    assemble(state(), entries, Router::new()).layer(MockConnectInfo(PEER))
}

struct Answer {
    response: Response,
    body: String,
}

impl Answer {
    fn status(&self) -> StatusCode {
        self.response.status()
    }

    fn header(&self, name: &str) -> String {
        self.response
            .headers()
            .get(name)
            .map(|value| value.to_str().unwrap().to_owned())
            .unwrap_or_default()
    }

    fn request_id(&self) -> String {
        self.header("x-request-id")
    }
}

async fn ask(case: &str, headers: &[(&str, &str)]) -> Answer {
    let mut builder = Request::builder().uri(format!("/test/{case}"));
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let request = builder.body(Body::empty()).unwrap();
    let response = test_router().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, BODY_LIMIT).await.unwrap();
    Answer {
        response: Response::from_parts(parts, Body::empty()),
        body: String::from_utf8(bytes.to_vec()).unwrap(),
    }
}

async fn ask_htmx(case: &str) -> Answer {
    ask(case, &[("hx-request", "true")]).await
}

async fn ask_english(case: &str) -> Answer {
    ask(
        case,
        &[("hx-request", "true"), ("cookie", "recipe_lang=en")],
    )
    .await
}

fn without_request_id(answer: &Answer) -> String {
    answer.body.replace(&answer.request_id(), "<id>")
}

#[test]
fn error_mapping_web_error_codes_follow_the_registry() {
    let id = RequestId::from(Uuid::nil());
    let cases = [
        (
            WebError::PayloadTooLarge {
                limit: ByteLimit::from(1),
            },
            ErrorCode::WEB_003,
        ),
        (
            WebError::BadRequest {
                field: "f",
                reason: FieldReason::Form,
            },
            ErrorCode::WEB_008,
        ),
        (WebError::CsrfRejected, ErrorCode::WEB_007),
        (WebError::ReadOnlyMode, ErrorCode::WEB_006),
        (
            app(AppError::NotFound {
                entity: Missing::MalformedLogin,
            }),
            ErrorCode::WEB_005,
        ),
        (WebError::Internal { request_id: id }, ErrorCode::WEB_010),
        (app(AppError::Unauthenticated), ErrorCode::WEB_004),
        (app(IdentityError::UserInactive), ErrorCode::IDN_004),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code, "{error}");
    }
}

#[test]
fn error_mapping_web_error_messages_are_the_registry_texts() {
    assert_eq!(
        WebError::PayloadTooLarge {
            limit: ByteLimit::from(256 * 1024)
        }
        .to_string(),
        "The request is larger than 256 KiB."
    );
    assert_eq!(
        WebError::BadRequest {
            field: "lang",
            reason: FieldReason::Lang
        }
        .to_string(),
        "Field lang is not valid: the language must be pl or en."
    );
    assert_eq!(
        WebError::CsrfRejected.to_string(),
        "The form is no longer valid. Reload the page and try again."
    );
    assert_eq!(
        app(AppError::NotFound {
            entity: Missing::MalformedLogin
        })
        .to_string(),
        "User does not exist."
    );
    assert_eq!(
        WebError::ReadOnlyMode.to_string(),
        "The system is in read-only mode because disk space is low. Contact the administrator."
    );
    assert_eq!(
        WebError::Internal {
            request_id: RequestId::from(Uuid::nil())
        }
        .to_string(),
        "An internal error occurred. Contact the administrator."
    );
}

#[test]
fn error_mapping_byte_limit_shows_binary_units() {
    assert_eq!(ByteLimit::from(256 * 1024).to_string(), "256 KiB");
    assert_eq!(ByteLimit::from(1024 * 1024).to_string(), "1 MiB");
    assert_eq!(ByteLimit::from(64 * 1024).to_string(), "64 KiB");
    assert_eq!(ByteLimit::from(1000).to_string(), "1000 B");
}

#[tokio::test]
async fn error_mapping_field_error_gives_422_and_a_list_line() {
    let answer = ask_htmx("field").await;
    assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        answer.body.contains(r#"<ul class="field-lines""#),
        "{}",
        answer.body
    );
    assert!(answer.body.contains("E-WEB-008"));
    assert!(answer.body.contains(&answer.request_id()));
    assert!(!answer.body.contains("<html"), "a fragment has no page");
}

#[tokio::test]
async fn error_mapping_htmx_error_retargets_the_error_panel() {
    for case in [
        "field",
        "policy",
        "last-admin",
        "db-rule",
        "trigger",
        "forbidden",
        "csrf",
        "credentials",
        "current-password",
        "not-found",
        "too-large",
        "throttled",
        "read-only",
        "timeout",
        "deadlock",
        "task-failed",
        "storage-other",
        "internal",
    ] {
        let answer = ask_htmx(case).await;
        assert!(answer.status().as_u16() >= 400, "{case}");
        assert_eq!(answer.header("hx-retarget"), "#error-panel", "{case}");
        assert_eq!(
            answer.header("hx-reswap"),
            "innerHTML show:#error-panel:top",
            "{case}"
        );
        assert_eq!(answer.header("hx-redirect"), "", "{case}");
        // Design D17 (U3): a refused form keeps the URL of its page.
        assert_eq!(answer.header("hx-push-url"), "false", "{case}");
    }
}

#[tokio::test]
async fn error_mapping_full_page_without_hx_request() {
    let answer = ask("field", &[]).await;
    assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(answer.header("hx-retarget"), "");
    assert_eq!(answer.header("hx-reswap"), "");
    assert_eq!(answer.header("hx-push-url"), "");
    assert!(
        answer.body.starts_with("<!DOCTYPE html>"),
        "{}",
        answer.body
    );
    assert!(
        answer
            .body
            .contains(r#"<script src="/static/htmx.min.js"></script>"#)
    );
    assert!(answer.body.contains(r#"<body hx-history="false""#));
    assert!(answer.body.contains(r#"<div id="error-panel">"#));
    assert!(answer.body.contains("E-WEB-008"));
    assert!(answer.body.contains(&answer.request_id()));
}

#[tokio::test]
async fn error_mapping_page_has_the_htmx_configuration_of_api_section_9() {
    let answer = ask("field", &[]).await;
    let meta = format!(r#"<meta name="htmx-config" content='{HTMX_CONFIG}'>"#);
    assert!(answer.body.contains(&meta), "{}", answer.body);
}

#[tokio::test]
async fn error_mapping_page_has_no_inline_script_style_or_handler() {
    for case in ["field", "last-admin", "internal"] {
        let body = ask(case, &[]).await.body;
        assert!(!body.contains("<style"), "{case}");
        assert!(!body.contains(" style="), "{case}");
        assert!(!body.contains("|safe"), "{case}");
        assert!(!body.contains(" onclick="), "{case}");
        assert!(!body.contains(" onload="), "{case}");
        assert_eq!(body.matches("<script").count(), 3, "{case}");
        assert_eq!(
            body.matches(r#"<script src="/static/"#).count(),
            3,
            "{case}"
        );
    }
}

#[tokio::test]
async fn error_mapping_page_without_session_has_no_hx_headers_attribute() {
    let body = ask("field", &[]).await.body;
    assert!(!body.contains("hx-headers"), "{body}");
}

#[tokio::test]
async fn error_mapping_domain_value_error_is_422_with_field_errors() {
    let answer = ask_htmx("policy").await;
    assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(answer.body.contains(r#"role="alert""#));
    assert!(answer.body.contains("E-IDN-006"));
}

#[tokio::test]
async fn error_mapping_state_conflict_is_409_with_error_html() {
    let answer = ask_htmx("last-admin").await;
    assert_eq!(answer.status(), StatusCode::CONFLICT);
    assert!(
        answer.body.contains(r#"class="alert alert-error""#),
        "{}",
        answer.body
    );
    assert!(!answer.body.contains("field-lines"));
    assert!(answer.body.contains("E-IDN-002"));
}

#[tokio::test]
async fn error_mapping_database_rules_are_409_with_error_html() {
    for (case, code) in [("db-rule", "E-DB-003"), ("trigger", "E-PLT-030")] {
        let answer = ask_htmx(case).await;
        assert_eq!(answer.status(), StatusCode::CONFLICT, "{case}");
        assert!(
            answer.body.contains(r#"class="alert alert-error""#),
            "{case}"
        );
        assert!(answer.body.contains(code), "{case}");
    }
}

#[tokio::test]
async fn error_mapping_forbidden_is_403_and_names_the_operation() {
    let answer = ask_english("forbidden").await;
    assert_eq!(answer.status(), StatusCode::FORBIDDEN);
    assert!(
        answer
            .body
            .contains("Your role does not permit the operation Create machine."),
        "{}",
        answer.body
    );
    assert!(answer.body.contains("E-WEB-002"));
}

#[tokio::test]
async fn error_mapping_csrf_rejection_is_403() {
    let answer = ask_english("csrf").await;
    assert_eq!(answer.status(), StatusCode::FORBIDDEN);
    assert!(answer.body.contains("E-WEB-007"));
    assert!(
        answer
            .body
            .contains("The form is no longer valid. Reload the page and try again.")
    );
}

#[tokio::test]
async fn error_mapping_no_session_redirects_htmx_with_a_header() {
    let answer = ask_htmx("unauthenticated").await;
    assert_eq!(answer.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(answer.header("hx-redirect"), "/login");
    assert_eq!(answer.header("hx-retarget"), "");
    assert_eq!(answer.header("hx-reswap"), "");
    assert_eq!(answer.header("hx-push-url"), "");
    assert_eq!(answer.header("location"), "");
}

#[tokio::test]
async fn error_mapping_no_session_redirects_a_page_with_303() {
    let answer = ask("unauthenticated", &[]).await;
    assert_eq!(answer.status(), StatusCode::SEE_OTHER);
    assert_eq!(answer.header("location"), "/login");
    assert_eq!(answer.header("hx-redirect"), "");
}

#[tokio::test]
async fn error_mapping_wrong_credentials_are_401_with_no_redirect() {
    let answer = ask_htmx("credentials").await;
    assert_eq!(answer.status(), StatusCode::UNAUTHORIZED);
    assert!(answer.body.contains(r#"role="alert""#), "{}", answer.body);
    assert!(answer.body.contains("E-IDN-003"));
    assert_eq!(answer.header("hx-redirect"), "");
    assert_eq!(answer.header("location"), "");
}

#[tokio::test]
async fn error_mapping_inactive_user_gets_the_answer_of_wrong_credentials() {
    for headers in [vec![("hx-request", "true")], vec![]] {
        let wrong = ask("credentials", &headers).await;
        let inactive = ask("inactive", &headers).await;
        assert_eq!(wrong.status(), inactive.status());
        assert_eq!(without_request_id(&wrong), without_request_id(&inactive));
        assert!(!inactive.body.contains("E-IDN-004"));
        assert!(inactive.body.contains("E-IDN-003"));
    }
}

#[tokio::test]
async fn error_mapping_wrong_current_password_is_422_with_field_errors() {
    let answer = ask_htmx("current-password").await;
    assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(answer.body.contains(r#"role="alert""#));
    assert!(answer.body.contains("E-IDN-008"));
}

#[tokio::test]
async fn error_mapping_missing_resource_is_404() {
    let answer = ask_htmx("not-found").await;
    assert_eq!(answer.status(), StatusCode::NOT_FOUND);
    assert!(answer.body.contains("E-WEB-005"));
    assert!(answer.body.contains(r#"class="alert alert-error""#));
}

#[tokio::test]
async fn error_mapping_large_request_is_413() {
    let answer = ask_english("too-large").await;
    assert_eq!(answer.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(answer.body.contains("The request is larger than 256 KiB."));
}

#[tokio::test]
async fn error_mapping_throttle_is_429_with_retry_after_of_the_error() {
    let answer = ask_english("throttled").await;
    assert_eq!(answer.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(answer.header("retry-after"), "12");
    assert!(
        answer
            .body
            .contains("Too many failed logins from this address. Try again in 12 seconds."),
        "{}",
        answer.body
    );
}

#[tokio::test]
async fn error_mapping_unavailable_answers_are_503_with_the_retry_after_setting() {
    for case in ["read-only", "timeout", "deadlock"] {
        let answer = ask_htmx(case).await;
        assert_eq!(answer.status(), StatusCode::SERVICE_UNAVAILABLE, "{case}");
        assert_eq!(answer.header("retry-after"), "5", "{case}");
    }
}

#[tokio::test]
async fn error_mapping_other_statuses_set_no_retry_after() {
    for case in [
        "field",
        "last-admin",
        "forbidden",
        "task-failed",
        "internal",
    ] {
        assert_eq!(ask_htmx(case).await.header("retry-after"), "", "{case}");
    }
}

#[tokio::test]
async fn error_mapping_unexpected_errors_are_500_with_code_and_request_id_only() {
    for (case, code) in [
        ("task-failed", "E-WEB-011"),
        ("storage-other", "E-DB-008"),
        ("internal", "E-WEB-010"),
    ] {
        let answer = ask_htmx(case).await;
        assert_eq!(answer.status(), StatusCode::INTERNAL_SERVER_ERROR, "{case}");
        assert!(answer.body.contains(code), "{case}");
        assert!(answer.body.contains(&answer.request_id()), "{case}");
        for forbidden in [
            "secret internal text",
            "XX000",
            "sqlstate",
            "Custom",
            "Debug",
        ] {
            assert!(!answer.body.contains(forbidden), "{case}: {forbidden}");
        }
    }
}

#[tokio::test]
async fn error_mapping_must_change_password_redirects_to_the_password_page() {
    for headers in [vec![("hx-request", "true")], vec![]] {
        let answer = ask("must-change", &headers).await;
        assert_eq!(answer.status(), StatusCode::SEE_OTHER);
        assert_eq!(answer.header("location"), "/me/password");
        assert_eq!(answer.header("hx-redirect"), "");
    }
}

#[tokio::test]
async fn error_mapping_text_follows_the_language_cookie() {
    let english = ask("forbidden", &[("cookie", "recipe_lang=en")]).await;
    let polish = ask("forbidden", &[("cookie", "recipe_lang=pl")]).await;
    let default = ask("forbidden", &[]).await;
    assert!(english.body.contains(r#"<html lang="en">"#));
    assert!(polish.body.contains(r#"<html lang="pl">"#));
    assert!(!polish.body.contains("Your role does not permit"));
    assert!(polish.body.contains("Utworzenie maszyny"));
    assert_eq!(
        polish.body.replace(&polish.request_id(), "<id>"),
        default.body.replace(&default.request_id(), "<id>")
    );
}

#[tokio::test]
async fn error_mapping_code_without_a_bundle_key_falls_back_to_english() {
    // The bundles hold the keys of the codes that the pages return. `E-AUD-003` has no key.
    let answer = ask(
        "trigger-audit",
        &[("hx-request", "true"), ("cookie", "recipe_lang=pl")],
    )
    .await;
    assert!(
        answer
            .body
            .contains("The rule of the database stopped the change."),
        "{}",
        answer.body
    );
}

#[tokio::test]
async fn error_mapping_request_id_of_the_page_is_the_header_value() {
    let answer = ask("internal", &[]).await;
    let id = answer.request_id();
    assert!(Uuid::parse_str(&id).is_ok());
    assert!(answer.body.contains(&id));
}

#[tokio::test]
async fn error_mapping_unknown_test_case_does_not_reach_a_success_page() {
    let answer = ask("no-such-case", &[]).await;
    assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// A writer that copies every line of the log into a shared buffer.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Captured {
    /// Return the lines of the error mapping. The line "request finished" of the `TraceLayer`
    /// (`reliability.md` section 3.3) is not a line of the mapping, so it does not count here.
    fn lines(&self) -> Vec<String> {
        String::from_utf8(self.0.lock().clone())
            .unwrap()
            .lines()
            .filter(|line| !line.contains("request finished"))
            .map(str::to_owned)
            .collect()
    }
}

/// Answer one request and return the answer and the lines that the log got.
async fn logged(case: &str, headers: &[(&str, &str)]) -> (Answer, Vec<String>) {
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let answer = ask(case, headers).await;
    (answer, captured.lines())
}

#[tokio::test]
async fn error_mapping_log_has_one_line_per_error_with_level_code_and_request_id() {
    // The level follows reliability.md section 3.3 and api.md section 5.1.
    let cases = [
        ("field", "INFO", "E-WEB-008"),
        ("policy", "INFO", "E-IDN-006"),
        ("last-admin", "INFO", "E-IDN-002"),
        ("not-found", "INFO", "E-WEB-005"),
        ("unauthenticated", "INFO", "E-WEB-004"),
        ("credentials", "INFO", "E-IDN-003"),
        ("inactive", "INFO", "E-IDN-004"),
        ("must-change", "INFO", "E-IDN-007"),
        ("forbidden", "INFO", "E-WEB-002"),
        ("forbidden-users", "WARN", "E-WEB-002"),
        ("csrf", "WARN", "E-WEB-007"),
        ("throttled", "WARN", "E-IDN-005"),
        ("db-rule", "ERROR", "E-DB-003"),
        ("trigger", "ERROR", "E-PLT-030"),
        ("read-only", "ERROR", "E-WEB-006"),
        ("timeout", "ERROR", "E-WEB-009"),
        ("deadlock", "ERROR", "E-DB-001"),
        ("task-failed", "ERROR", "E-WEB-011"),
        ("storage-other", "ERROR", "E-DB-008"),
        ("internal", "ERROR", "E-WEB-010"),
    ];
    for (case, level, code) in cases {
        let (answer, lines) = logged(case, &[]).await;
        assert_eq!(lines.len(), 1, "{case}: {lines:?}");
        let line = &lines[0];
        assert_eq!(
            line.split_whitespace().next(),
            Some(level),
            "{case}: {line}"
        );
        assert!(
            line.contains(&format!("error_code={code}")),
            "{case}: {line}"
        );
        assert!(
            line.contains(&format!("request_id={}", answer.request_id())),
            "{case}: {line}"
        );
    }
}

#[tokio::test]
async fn error_mapping_log_holds_the_display_chain_and_never_the_debug_text() {
    let (_answer, lines) = logged("storage-other", &[]).await;
    let line = &lines[0];
    assert!(
        line.contains("An unexpected database error occurred."),
        "{line}"
    );
    assert!(line.contains("secret internal text"), "{line}");
    for debug_text in ["sqlstate", "XX000", "Custom", "kind:", "Other {", "App("] {
        assert!(!line.contains(debug_text), "{debug_text} in {line}");
    }
}

#[tokio::test]
async fn error_mapping_log_of_a_domain_error_shows_its_message() {
    let (_answer, lines) = logged("last-admin", &[]).await;
    assert!(
        lines[0].contains("anna is the last active administrator."),
        "{}",
        lines[0]
    );
}

#[tokio::test]
async fn error_mapping_log_of_an_unknown_login_holds_no_typed_text() {
    let (_answer, lines) = logged("credentials", &[]).await;
    assert!(!lines[0].contains("UnknownLogin"), "{}", lines[0]);
    assert!(!lines[0].contains("unknown_login"), "{}", lines[0]);
}

#[tokio::test]
async fn error_mapping_a_response_without_an_error_writes_no_log_line() {
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let request = Request::builder()
        .uri("/static/htmx.min.js")
        .body(Body::empty())
        .unwrap();
    let response = test_router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(captured.lines().is_empty());
}
