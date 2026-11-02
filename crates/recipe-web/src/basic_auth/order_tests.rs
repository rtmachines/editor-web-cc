//! Tests of the scheme of each export route and of the layer order (design D3, ADR-0109 (p)).
//!
//! The requirement "One scheme for each export route" says that a transfer route never reads the
//! session cookie and a session route never reads the `Authorization` header. The requirement
//! "Navigation guard before the Basic check" says that the guard of ADR-0055 runs before the
//! password check. The test row `/export/test/basic` is a Basic row under the export prefix. The
//! two real export routes are session rows. `Counts` shows which side effects a request had.

use axum::http::StatusCode;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_domain::audit::AuditAction;
use recipe_domain::identity::Role;

use crate::test_support::{Fixture, Reply, get_with};

/// The Basic row under the export prefix.
const BASIC_ROUTE: &str = "/export/test/basic";
/// The two session routes: the approved version of `R-500` and the retired version of `R-600`.
const SESSION_ROUTES: [&str; 2] = [
    "/export/v1/recipes/R-500/approved",
    "/export/v1/recipes/R-600/versions/1",
];
/// The password of the seeded `ExportClient` account `user1`.
const PASSWORD: &str = "tr4nsfer-s3cret";
/// A wrong password.
const WRONG_PASSWORD: &str = "wr0ng-s3cret";
/// The name of the header `Sec-Fetch-Site`.
const SITE: &str = "sec-fetch-site";
/// The values of `Sec-Fetch-Site` that the guard rejects (ADR-0055 (b)).
const FOREIGN_SITES: [&str; 2] = ["cross-site", "same-site"];

/// The side effects of the requests until now.
#[derive(Debug, PartialEq, Eq)]
struct Counts {
    attempts: usize,
    exported: usize,
    events: usize,
}

impl Counts {
    fn of(fixture: &Fixture) -> Self {
        let exported = fixture
            .store
            .audit_entries()
            .iter()
            .filter(|entry| entry.action() == AuditAction::VersionExported)
            .count();
        Self {
            attempts: fixture.store.state().login_attempt_count(),
            exported,
            events: fixture.sink.events().len(),
        }
    }
}

fn authorization(password: &str) -> String {
    format!("Basic {}", STANDARD.encode(format!("user1:{password}")))
}

/// Build a fixture with the account `user1` and the data of the two session routes.
async fn fixture_with_client_and_recipes() -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::ExportClient, PASSWORD, false);
    fixture.seed_approval_recipes().await;
    fixture
}

async fn send(fixture: &Fixture, path: &str, headers: &[(&str, &str)]) -> Reply {
    get_with(&fixture.router(), path, headers).await
}

#[tokio::test]
async fn basic_guard_runs_before_the_password_check_with_valid_credentials() {
    let fixture = fixture_with_client_and_recipes().await;
    let header = authorization(PASSWORD);
    for site in FOREIGN_SITES {
        let before = Counts::of(&fixture);

        let reply = send(
            &fixture,
            BASIC_ROUTE,
            &[("authorization", &header), (SITE, site)],
        )
        .await;

        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{site}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-007"), "{site}: {}", reply.body);
        assert_eq!(
            Counts::of(&fixture).attempts,
            before.attempts,
            "{site}: the guard let the password check run"
        );
    }
}

#[tokio::test]
async fn basic_guard_runs_before_the_password_check_with_wrong_credentials() {
    let fixture = fixture_with_client_and_recipes().await;
    let header = authorization(WRONG_PASSWORD);
    let before = Counts::of(&fixture);

    let reply = send(
        &fixture,
        BASIC_ROUTE,
        &[("authorization", &header), (SITE, "cross-site")],
    )
    .await;

    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"), "{}", reply.body);
    assert_eq!(Counts::of(&fixture).attempts, before.attempts);
}

#[tokio::test]
async fn basic_guard_runs_before_the_header_check_and_the_htmx_header_fails_too() {
    let fixture = fixture_with_client_and_recipes().await;
    let header = authorization(PASSWORD);

    let no_header = send(&fixture, BASIC_ROUTE, &[(SITE, "cross-site")]).await;
    let htmx = send(
        &fixture,
        BASIC_ROUTE,
        &[("authorization", &header), ("hx-request", "true")],
    )
    .await;

    assert_eq!(
        no_header.status,
        StatusCode::FORBIDDEN,
        "{}",
        no_header.body
    );
    assert!(no_header.body.contains("E-WEB-007"), "{}", no_header.body);
    assert_eq!(htmx.status, StatusCode::FORBIDDEN, "{}", htmx.body);
    assert!(htmx.body.contains("E-WEB-007"), "{}", htmx.body);
    assert_eq!(Counts::of(&fixture).attempts, 0);
}

#[tokio::test]
async fn basic_check_runs_when_the_guard_passes() {
    let fixture = fixture_with_client_and_recipes().await;
    let wrong = authorization(WRONG_PASSWORD);
    let right = authorization(PASSWORD);

    let refused = send(
        &fixture,
        BASIC_ROUTE,
        &[("authorization", &wrong), (SITE, "same-origin")],
    )
    .await;
    let after_refusal = Counts::of(&fixture);
    let accepted = send(
        &fixture,
        BASIC_ROUTE,
        &[("authorization", &right), (SITE, "none")],
    )
    .await;

    assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
    assert_eq!(after_refusal.attempts, 1, "the control must write a row");
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.body);
    assert!(
        accepted.body.starts_with("login=user1;"),
        "{}",
        accepted.body
    );
}

#[tokio::test]
async fn basic_transfer_route_ignores_the_session_cookie() {
    let fixture = fixture_with_client_and_recipes().await;
    let administrator = fixture.session_of(41);
    let before = Counts::of(&fixture);

    let reply = send(&fixture, BASIC_ROUTE, &[("cookie", &administrator.cookie)]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-012"), "{}", reply.body);
    assert!(!reply.body.contains("login="), "{}", reply.body);
    assert_eq!(Counts::of(&fixture), before);
}

#[tokio::test]
async fn basic_credentials_do_not_open_a_session_route() {
    let fixture = fixture_with_client_and_recipes().await;
    let header = authorization(PASSWORD);
    for route in SESSION_ROUTES {
        let before = Counts::of(&fixture);

        let reply = send(&fixture, route, &[("authorization", &header)]).await;

        assert_eq!(
            reply.status,
            StatusCode::SEE_OTHER,
            "{route}: {}",
            reply.body
        );
        assert_eq!(reply.header("location"), "/login", "{route}");
        assert_eq!(Counts::of(&fixture), before, "{route}");
    }
}

#[tokio::test]
async fn basic_header_is_not_read_on_a_session_route_with_a_session() {
    let fixture = fixture_with_client_and_recipes().await;
    let administrator = fixture.session_of(41);
    let wrong = authorization(WRONG_PASSWORD);
    for route in SESSION_ROUTES {
        let before = Counts::of(&fixture);

        let reply = send(
            &fixture,
            route,
            &[("authorization", &wrong), ("cookie", &administrator.cookie)],
        )
        .await;

        assert_eq!(reply.status, StatusCode::OK, "{route}: {}", reply.body);
        let after = Counts::of(&fixture);
        assert_eq!(after.attempts, before.attempts, "{route}: Basic check ran");
        assert_eq!(after.events, before.events, "{route}");
        assert_eq!(after.exported, before.exported + 1, "{route}");
    }
}
