//! Tests of the read-only banner (task 6.4, design D12 and D14 row T12, ADR-0071, ADR-0073 (h)).
//!
//! The banner stands directly after `<main id="content">` and before `#error-panel`. It stands in
//! the full page, in the content answer of `HX-Target: content`, on the login page, and on the
//! error page. It has no role, no button, and no link. The tests use the real router, so they
//! prove the path from the flag in `ReadOnlyMode` to the markup.

use askama::Template;
use axum::Router;
use axum::extract::State;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::routing::get;
use recipe_app::auth::Operation;
use recipe_app::ports::NonEmpty;
use recipe_domain::identity::Role;

use super::SessionFrame;
use crate::WebState;
use crate::csrf::issue_pre_session;
use crate::error::{FieldError, FieldReason, FormErrors, RenderedForm, WebError};
use crate::pages::test_helpers::{assert_clean_html, encode};
use crate::pages::{PageRequest, Texts};
use crate::routes::{Access, RouteEntry, assemble, table};
use crate::test_support::{
    Fixture, PEER, Reply, TestSession, call, cookie_pair, get_with, icon_tag, post_with, request,
    text,
};

/// The English text of the banner (design D12, ADR-0073 (h)).
const EN_BANNER: &str = "Read-only mode: the free disk space is low or unknown. Changes are blocked. You can view, export, log in, and log out.";
/// The Polish text of the banner (design D12, ADR-0073 (h)).
const PL_BANNER: &str = "Tryb tylko do odczytu: brakuje miejsca na dysku lub nie można go zmierzyć. Zmiany są zablokowane. Możesz przeglądać, eksportować, logować się i wylogować.";
/// The language codes and the banner text of each language.
const LANGS: [(&str, &str); 2] = [("pl", PL_BANNER), ("en", EN_BANNER)];
/// The opening of the full page content. The banner follows it at once.
const MAIN_OPEN: &str = "<main id=\"content\">";
/// The start of the error panel. The banner stands before it.
const ERROR_PANEL: &str = "<div id=\"error-panel\">";
/// The full GET pages that the sweep opens with an Administrator session.
const SWEEP_PAGES: [&str; 8] = [
    "/me",
    "/me/password",
    "/users",
    "/users/new",
    "/recipes",
    "/machines",
    "/audit",
    "/security-events",
];

/// Return the markup that opens the banner: the notice with the severity `warning` and its icon.
fn banner_open() -> String {
    format!(
        "<div class=\"alert alert-warning\">{}",
        icon_tag("alert-warning")
    )
}

/// Return the end of the banner for `message`: the text in its `span` and the close of the `div`.
fn banner_close(message: &str) -> String {
    format!("<span>{message}</span></div>")
}

/// Return the position of `needle` in `body`, and fail with the body when it is missing.
fn at(body: &str, needle: &str) -> usize {
    assert!(body.contains(needle), "{needle:?} missing in {body}");
    body.find(needle).unwrap()
}

/// Assert that `body` holds the banner `message` exactly once, before `#error-panel`.
///
/// The banner has no role, no button, and no link.
fn assert_banner(body: &str, message: &str, what: &str) {
    let (open, close) = (banner_open(), banner_close(message));
    assert_eq!(body.matches(&open).count(), 1, "{what}: {body}");
    assert_eq!(body.matches(&close).count(), 1, "{what}: {body}");
    let (start, end) = (at(body, &open), at(body, &close));
    assert!(start < end, "{what}: the text stands before the icon");
    assert!(
        end < at(body, ERROR_PANEL),
        "{what}: banner after the error panel"
    );
    let banner = body.get(start..end).unwrap();
    assert!(!banner.contains("role="), "{what}: {banner}");
    assert!(!banner.contains("<button"), "{what}: {banner}");
    assert!(!banner.contains("<a "), "{what}: {banner}");
    assert_clean_html(body);
}

/// Assert that `body` holds no banner in either language.
fn assert_no_banner(body: &str, what: &str) {
    for message in [EN_BANNER, PL_BANNER] {
        assert!(!body.contains(message), "{what}: {body}");
    }
    assert!(!body.contains(&banner_open()), "{what}: {body}");
}

/// Return the header pairs of a request in the language `code` with the session.
fn cookie_of(session: Option<&TestSession>, code: &str) -> String {
    match session {
        Some(session) => format!("{}; recipe_lang={code}", session.cookie),
        None => format!("recipe_lang={code}"),
    }
}

/// Send `GET path` as a full page request.
async fn page(fixture: &Fixture, session: Option<&TestSession>, code: &str, path: &str) -> Reply {
    let cookie = cookie_of(session, code);
    get_with(&fixture.router(), path, &[("cookie", cookie.as_str())]).await
}

/// Send `GET path` as the content request of HTMX: `HX-Request` and `HX-Target: content`.
async fn content(fixture: &Fixture, session: &TestSession, code: &str, path: &str) -> Reply {
    let cookie = cookie_of(Some(session), code);
    let headers = [
        ("cookie", cookie.as_str()),
        ("hx-request", "true"),
        ("hx-target", "content"),
    ];
    get_with(&fixture.router(), path, &headers).await
}

/// Send a failed login: a valid pre-session token and the login `typed`. The password is too short,
/// so the answer comes before any hash.
async fn failed_login(fixture: &Fixture, code: &str, typed: &str) -> Reply {
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let cookie = format!(
        "{}; recipe_lang={code}",
        cookie_pair(token.set_cookie().to_str().unwrap())
    );
    let body = format!(
        "login={}&password=x&csrf_pre={}",
        encode(typed),
        token.nonce()
    );
    let request = request(axum::http::Method::POST, "/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", cookie)
        .body(axum::body::Body::from(body))
        .unwrap();
    call(&fixture.router(), request).await
}

/// Build a fixture with an Administrator session and the mode `on`.
fn world(on: bool) -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    fixture.state.read_only().set(on);
    (fixture, session)
}

#[test]
fn banner_text_is_the_text_of_design_d12_in_both_bundles() {
    assert_eq!(text(crate::i18n::Lang::En, "readonly-banner"), EN_BANNER);
    assert_eq!(text(crate::i18n::Lang::Pl, "readonly-banner"), PL_BANNER);
}

#[tokio::test]
async fn banner_stands_in_the_full_page_in_both_languages() {
    let (fixture, session) = world(true);

    for (code, message) in LANGS {
        let reply = page(&fixture, Some(&session), code, "/me").await;

        assert_eq!(reply.status, StatusCode::OK, "{code}");
        assert_banner(&reply.body, message, code);
        let first = format!("{MAIN_OPEN}{}", banner_open());
        assert!(reply.body.contains(&first), "{code}: {}", reply.body);
    }
}

#[tokio::test]
async fn banner_is_absent_from_the_full_page_while_the_mode_is_off() {
    let (fixture, session) = world(false);

    for (code, _message) in LANGS {
        let reply = page(&fixture, Some(&session), code, "/me").await;

        assert_eq!(reply.status, StatusCode::OK, "{code}");
        assert_no_banner(&reply.body, code);
        assert!(reply.body.contains(MAIN_OPEN), "{code}");
    }
}

#[tokio::test]
async fn banner_stands_in_the_content_answer_before_the_error_panel() {
    let (fixture, session) = world(true);

    for (code, message) in LANGS {
        let reply = content(&fixture, &session, code, "/recipes").await;

        assert_eq!(reply.status, StatusCode::OK, "{code}");
        assert!(!reply.body.contains("<main"), "{code}: {}", reply.body);
        assert!(!reply.body.contains("<!DOCTYPE"), "{code}");
        assert_banner(&reply.body, message, code);
        assert!(
            at(&reply.body, "</title>") < at(&reply.body, &banner_open()),
            "{code}: banner before the title"
        );
    }
}

#[tokio::test]
async fn banner_is_absent_from_the_content_answer_while_the_mode_is_off() {
    let (fixture, session) = world(false);

    for (code, _message) in LANGS {
        let reply = content(&fixture, &session, code, "/recipes").await;

        assert_eq!(reply.status, StatusCode::OK, "{code}");
        assert_no_banner(&reply.body, code);
        assert!(reply.body.contains(ERROR_PANEL), "{code}");
    }
}

#[tokio::test]
async fn banner_follows_the_mode_between_two_requests() {
    let (fixture, session) = world(false);
    let off = page(&fixture, Some(&session), "en", "/me").await;
    fixture.state.read_only().set(true);
    let on = page(&fixture, Some(&session), "en", "/me").await;
    fixture.state.read_only().set(false);
    let off_again = page(&fixture, Some(&session), "en", "/me").await;

    assert_no_banner(&off.body, "first");
    assert_banner(&on.body, EN_BANNER, "second");
    assert_no_banner(&off_again.body, "third");
}

#[tokio::test]
async fn banner_stands_on_the_login_page_in_both_languages() {
    let (fixture, _session) = world(true);

    for (code, message) in LANGS {
        let reply = page(&fixture, None, code, "/login").await;

        assert_eq!(reply.status, StatusCode::OK, "{code}");
        assert!(reply.body.contains("name=\"password\""), "{code}");
        assert_banner(&reply.body, message, code);
    }
}

#[tokio::test]
async fn banner_is_absent_from_the_login_page_while_the_mode_is_off() {
    let (fixture, _session) = world(false);

    for (code, _message) in LANGS {
        let reply = page(&fixture, None, code, "/login").await;

        assert_eq!(reply.status, StatusCode::OK, "{code}");
        assert_no_banner(&reply.body, code);
    }
}

#[tokio::test]
async fn banner_stands_on_the_login_page_after_a_failed_login() {
    let (fixture, _session) = world(true);

    for (code, message) in LANGS {
        let reply = failed_login(&fixture, code, "nobody").await;

        assert!(reply.status.is_client_error(), "{code}: {}", reply.status);
        assert!(reply.body.contains("name=\"password\""), "{code}");
        assert_banner(&reply.body, message, code);
    }
}

#[tokio::test]
async fn banner_is_absent_from_the_login_page_after_a_failed_login_while_the_mode_is_off() {
    let (fixture, _session) = world(false);

    let reply = failed_login(&fixture, "pl", "nobody").await;

    assert!(reply.status.is_client_error(), "{}", reply.status);
    assert_no_banner(&reply.body, "failed login");
}

#[tokio::test]
async fn banner_stands_on_the_error_page_of_a_refused_command() {
    let (fixture, session) = world(true);

    for (code, message) in LANGS {
        let cookie = cookie_of(Some(&session), code);
        let reply = post_with(
            &fixture.router(),
            "/users",
            Some(&session),
            &[("cookie", cookie.as_str())],
        )
        .await;

        assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE, "{code}");
        assert!(reply.body.contains("E-WEB-006"), "{code}: {}", reply.body);
        assert_banner(&reply.body, message, code);
    }
}

/// Build a fixture with a session of `role` and the mode `on`.
fn world_as(role: Role, on: bool) -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, role, false);
    fixture.state.read_only().set(on);
    (fixture, session)
}

#[tokio::test]
async fn banner_stands_on_the_recipes_page_of_an_operator_while_the_mode_is_on() {
    let (fixture, session) = world_as(Role::Operator, true);

    for (code, message) in LANGS {
        let reply = page(&fixture, Some(&session), code, "/recipes").await;

        assert_eq!(reply.status, StatusCode::OK, "{code}: {}", reply.body);
        assert_banner(&reply.body, message, code);
    }
}

#[tokio::test]
async fn banner_is_absent_from_the_recipes_page_of_a_process_engineer_while_the_mode_is_off() {
    let (fixture, session) = world_as(Role::ProcessEngineer, false);

    for (code, _message) in LANGS {
        let reply = page(&fixture, Some(&session), code, "/recipes").await;

        assert_eq!(reply.status, StatusCode::OK, "{code}: {}", reply.body);
        assert_no_banner(&reply.body, code);
        assert!(reply.body.contains(MAIN_OPEN), "{code}");
    }
}

#[tokio::test]
async fn banner_stands_on_the_error_page_of_a_forbidden_page() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    fixture.state.read_only().set(true);

    for (code, message) in LANGS {
        let reply = page(&fixture, Some(&session), code, "/users").await;

        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{code}");
        assert_banner(&reply.body, message, code);
    }
}

#[tokio::test]
async fn banner_is_absent_from_the_error_page_while_the_mode_is_off() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);

    let reply = page(&fixture, Some(&session), "en", "/users").await;

    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert!(reply.body.contains("E-WEB-002"), "{}", reply.body);
    assert_no_banner(&reply.body, "forbidden page");
}

#[tokio::test]
async fn banner_is_absent_from_the_error_fragment_of_an_htmx_command() {
    let (fixture, session) = world(true);

    let reply = post_with(
        &fixture.router(),
        "/users",
        Some(&session),
        &[("hx-request", "true"), ("hx-target", "content")],
    )
    .await;

    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(reply.body.contains("E-WEB-006"), "{}", reply.body);
    assert_no_banner(&reply.body, "error fragment");
}

#[tokio::test]
async fn banner_is_absent_from_the_export_answer_and_the_static_file() {
    let (fixture, session) = world(true);
    let headers = [
        ("cookie", session.cookie.as_str()),
        ("sec-fetch-site", "same-origin"),
    ];

    let export = get_with(
        &fixture.router(),
        "/export/v1/recipes/R-100/approved",
        &headers,
    )
    .await;
    let style = get_with(&fixture.router(), "/static/app.css", &[]).await;

    assert!(
        export
            .header("content-type")
            .starts_with("application/json")
    );
    assert_no_banner(&export.body, "export answer");
    assert_eq!(style.status, StatusCode::OK);
    assert_no_banner(&style.body, "static file");
}

#[tokio::test]
async fn banner_sweep_every_full_page_and_content_answer_shows_it_only_while_on() {
    for on in [true, false] {
        let (fixture, session) = world(on);
        for path in SWEEP_PAGES {
            for (code, message) in LANGS {
                let full = page(&fixture, Some(&session), code, path).await;
                let part = content(&fixture, &session, code, path).await;

                for (reply, shape) in [(full, "page"), (part, "content")] {
                    let what = format!("{path} {code} {shape} on={on}");
                    assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
                    if on {
                        assert_banner(&reply.body, message, &what);
                    } else {
                        assert_no_banner(&reply.body, &what);
                    }
                }
            }
        }
    }
}

/// The form of the test route that refuses. It is the form of `draft_reply` tests.
#[derive(Template)]
#[template(path = "test/oob-form.html")]
struct RefusedForm<'a> {
    t: Texts<'a>,
    oob: bool,
    value: String,
    row_version: i64,
    csrf: String,
    errors: FormErrors,
}

/// The path of the test route whose handler returns `WebError::Fields` with a framed form.
const REFUSED_FORM_PATH: &str = "/test/refused-form";

/// Refuse the request with a form that holds the session parts of the layout (`api.md` 5.3).
async fn refuse_with_form(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<&'static str, WebError> {
    let lang = request.lang();
    let texts = Texts::new(state.bundles(), lang);
    let lines = NonEmpty::new(FieldError::bad_request("tag", FieldReason::Tag));
    let errors = FormErrors::new(lines.iter(), state.bundles(), lang);
    let form = RefusedForm {
        t: Texts::new(state.bundles(), lang),
        oob: true,
        value: "SENT".to_owned(),
        row_version: 7,
        csrf: request.csrf_token(),
        errors,
    };
    let rendered = RenderedForm::from_template(&form, request.request_id())?
        .with_frame(SessionFrame::of(texts, &request));
    Err(WebError::Fields {
        lines,
        form: Some(rendered),
        failure: None,
    })
}

/// Send `GET` to the test route in the language `code` and return the answer.
///
/// The route is a query of the operation `GetOwnProfile`, so it runs while the mode is on and
/// the gate does not answer `503`. The handler returns `WebError::Fields` with a framed form.
async fn refused_form(fixture: &Fixture, session: &TestSession, code: &str) -> Reply {
    let rows = vec![RouteEntry {
        method: Method::GET,
        path: REFUSED_FORM_PATH,
        access: Access::Operation(Operation::GetOwnProfile),
        handler: get(refuse_with_form),
    }];
    let mut entries = table();
    entries.extend(rows);
    let router =
        assemble(fixture.state.clone(), entries, Router::new()).layer(MockConnectInfo(PEER));
    let cookie = cookie_of(Some(session), code);
    get_with(&router, REFUSED_FORM_PATH, &[("cookie", cookie.as_str())]).await
}

/// Assert the full page of the refused form: its status, its session layout, and its form.
fn assert_refused_form_page(reply: &Reply, session: &TestSession, what: &str) {
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{what}");
    let headers = format!("hx-headers='{{\"X-CSRF-Token\":\"{}\"}}'", session.csrf);
    assert!(reply.body.contains(&headers), "{what}: {}", reply.body);
    assert!(
        reply
            .body
            .contains("<form id=\"test-form\" method=\"post\""),
        "{what}: {}",
        reply.body
    );
    assert!(!reply.body.contains("hx-swap-oob"), "{what}");
}

#[tokio::test]
async fn banner_stands_on_the_page_of_a_refused_form_in_both_languages() {
    let (fixture, session) = world(true);

    for (code, message) in LANGS {
        let reply = refused_form(&fixture, &session, code).await;

        assert_refused_form_page(&reply, &session, code);
        assert_banner(&reply.body, message, code);
        assert!(
            at(&reply.body, &banner_open()) < at(&reply.body, "<form id=\"test-form\""),
            "{code}: banner after the form"
        );
    }
}

#[tokio::test]
async fn banner_is_absent_from_the_page_of_a_refused_form_while_the_mode_is_off() {
    let (fixture, session) = world(false);

    for (code, _message) in LANGS {
        let reply = refused_form(&fixture, &session, code).await;

        assert_refused_form_page(&reply, &session, code);
        assert_no_banner(&reply.body, code);
        assert!(reply.body.contains(ERROR_PANEL), "{code}");
    }
}

mod xss;
