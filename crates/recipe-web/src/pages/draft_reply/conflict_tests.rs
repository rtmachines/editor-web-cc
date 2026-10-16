//! Tests of the success answer and the conflict answer of a draft (task 7.4, design D10).
//!
//! The routes run the real use cases over the memory store, so the conflict shows the actor and
//! the time of the audit entry of the first save.

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::response::{Html, Response};
use axum::routing::get;
use recipe_app::auth::Operation;
use recipe_app::error::AppError;
use recipe_app::recipe::{UpsertTranslation, UpsertTranslationInput};
use recipe_domain::identity::Role;
use recipe_domain::recipe::{LangCode, ParameterKey, RecipeError, ReferenceNumber, Translation};
use recipe_domain::text::Text;

use super::{conflict, success};
use crate::WebState;
use crate::error::WebError;
use crate::pages::PageRequest;
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::recipes::LINE_KEY;
use crate::test_support::{Fixture, PEER, Reply, TestSession, capture_logs, get_with};

/// The time of every write of the fixture clock, in the form of `design-system.md` section 5.
const FIXTURE_TIME: &str = "1970-01-12 13:46 UTC";
/// The end of every HTMX error answer: the empty status region out of band (design-system.md 10.7).
const TOAST_CLEAR: &str = "<div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>";

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-100").unwrap()
}

/// Save the Polish translation of `LINE` in the version 3 of `R-100` with the sent row version.
async fn save(
    State(state): State<WebState>,
    Path(row_version): Path<i64>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let input = UpsertTranslationInput {
        reference: reference(),
        number: 3,
        expected_row_version: row_version,
        key: ParameterKey::from(LINE_KEY),
        translation: Translation {
            lang: LangCode::parse("pl").unwrap(),
            name: Text::parse("linia").unwrap(),
            description: None,
        },
    };
    let user = request.auth()?.user();
    match UpsertTranslation::new(state.identity().app().clone())
        .execute(user, &request.meta(), input)
        .await
    {
        Ok(output) => success(
            &output,
            Html("<p>saved</p>".to_owned()),
            request.request_id(),
        ),
        Err(error) => Err(conflict(&state, &request, error).await),
    }
}

/// Raise a conflict on the version `number` of `R-100` without a save before it.
async fn stale(
    State(state): State<WebState>,
    Path(number): Path<u32>,
    request: PageRequest,
) -> Result<&'static str, WebError> {
    let error = AppError::ConcurrentEdit {
        reference: reference(),
        number,
        expected: 1,
        actual: 5,
    };
    Err(conflict(&state, &request, error).await)
}

/// Pass an error that is not a conflict.
async fn other(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<&'static str, WebError> {
    let error = AppError::Recipe(RecipeError::RecipeDeleted {
        reference: reference(),
    });
    Err(conflict(&state, &request, error).await)
}

/// Build the router with the test routes. Each route needs a session.
fn router(fixture: &Fixture) -> Router {
    let route = |path: &'static str, handler| RouteEntry {
        method: Method::GET,
        path,
        access: Access::Operation(Operation::GetVersion),
        handler,
    };
    let rows = vec![
        route("/test/save/{row_version}", get(save)),
        route("/test/stale/{number}", get(stale)),
        route("/test/other", get(other)),
    ];
    assemble(fixture.state.clone(), rows, Router::new())
        .layer(axum::extract::connect_info::MockConnectInfo(PEER))
}

/// A fixture with the recipe `R-100` and a ProcessEngineer `user2` with a session.
///
/// The Administrator `user1` created the recipe, so the read of the version needs that user.
fn world() -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    fixture.seed_plant();
    fixture.seed_session(1, Role::Administrator, false);
    let session = fixture.seed_session(2, Role::ProcessEngineer, false);
    (fixture, session)
}

async fn ask(fixture: &Fixture, session: &TestSession, path: &str, lang: &str) -> Reply {
    let cookie = format!("{}; recipe_lang={lang}", session.cookie);
    let headers = [("cookie", cookie.as_str()), ("hx-request", "true")];
    get_with(&router(fixture), path, &headers).await
}

#[tokio::test]
async fn conflict_a_saved_draft_sets_hx_trigger_with_the_new_row_version() {
    let (fixture, session) = world();
    let reply = ask(&fixture, &session, "/test/save/1", "en").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    assert_eq!(reply.body, "<p>saved</p>");
}

#[tokio::test]
async fn conflict_a_stale_save_shows_the_row_version_the_login_and_the_time_of_the_last_change() {
    let (fixture, session) = world();
    ask(&fixture, &session, "/test/save/1", "en").await;
    let reply = ask(&fixture, &session, "/test/save/1", "en").await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<span class=\"alert-code\">E-WEB-001</span>"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains(
        "Another user changed version 3 of recipe R-100. Reload the page and apply your change again."
    ), "{}", reply.body);
    assert!(
        reply.body.contains("<li>Current change number: 2</li>"),
        "{}",
        reply.body
    );
    let last = format!("<li>Last change: user2, {FIXTURE_TIME}</li>");
    assert!(reply.body.contains(&last), "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<li>Your changes are not saved. The reload discards them.</li>")
    );
    let reference = format!("Reference: {}.", reply.header("x-request-id"));
    assert!(reply.body.contains(&reference), "{}", reply.body);
}

#[tokio::test]
async fn conflict_the_answer_goes_into_the_error_panel_and_keeps_the_url_and_the_input() {
    let (fixture, session) = world();
    let reply = ask(&fixture, &session, "/test/stale/3", "en").await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.header("hx-reswap"), "innerHTML show:#error-panel:top");
    assert_eq!(reply.header("hx-push-url"), "false");
    assert!(reply.header("hx-trigger").is_empty());
    assert!(!reply.body.contains("<form"), "{}", reply.body);
    // The only element out of band is the empty status region (design-system.md 10.7).
    assert_eq!(
        reply.body.matches("hx-swap-oob").count(),
        1,
        "{}",
        reply.body
    );
    assert!(reply.body.ends_with(TOAST_CLEAR), "{}", reply.body);
}

#[tokio::test]
async fn conflict_the_reload_link_is_a_boosted_link_to_the_version_page() {
    let (fixture, session) = world();
    let reply = ask(&fixture, &session, "/test/stale/3", "en").await;
    assert!(
        reply.body.contains(
            "<a class=\"target-link\" href=\"/recipes/R-100/versions/3\" hx-boost=\"true\">Reload version</a>"
        ),
        "{}",
        reply.body
    );
    let fragment = reply.body.strip_suffix(TOAST_CLEAR).unwrap();
    assert_eq!(fragment.matches(" hx-").count(), 1, "{}", reply.body);
}

#[tokio::test]
async fn conflict_without_a_last_change_says_that_it_cannot_show_it() {
    let (fixture, session) = world();
    let reply = ask(&fixture, &session, "/test/stale/3", "en").await;
    assert!(
        reply.body.contains("<li>Current change number: 1</li>"),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("<li>The system cannot show the last change.</li>"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("Last change:"), "{}", reply.body);
}

#[tokio::test]
async fn conflict_after_a_failed_read_is_still_e_web_001_and_logs_the_failure_once() {
    let (fixture, session) = world();
    let (logs, _guard) = capture_logs();
    let reply = ask(&fixture, &session, "/test/stale/99", "en").await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-001"), "{}", reply.body);
    assert!(
        reply.body.contains("<li>Current change number: 5</li>"),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("<li>The system cannot show the last change.</li>")
    );
    assert!(!reply.body.contains("E-WEB-005"), "{}", reply.body);
    let text = logs.text();
    assert_eq!(text.matches("request failed").count(), 1, "{text}");
    assert!(text.contains("E-WEB-001"), "{text}");
    assert!(text.contains("does not exist"), "{text}");
}

#[tokio::test]
async fn conflict_follows_the_language_of_the_request() {
    let (fixture, session) = world();
    ask(&fixture, &session, "/test/save/1", "pl").await;
    let reply = ask(&fixture, &session, "/test/save/1", "pl").await;
    let last = format!("<li>Ostatnia zmiana: user2, {FIXTURE_TIME}</li>");
    assert!(reply.body.contains(&last), "{}", reply.body);
    assert!(
        reply.body.contains("<li>Obecny numer zmiany: 2</li>"),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains(">Od\u{15b}wie\u{17c} wersj\u{119}</a>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn conflict_passes_an_error_that_is_not_a_conflict_unchanged() {
    let (fixture, session) = world();
    let reply = ask(&fixture, &session, "/test/other", "en").await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body.contains("E-RCP-012"), "{}", reply.body);
    assert!(!reply.body.contains("Reload version"), "{}", reply.body);
}
