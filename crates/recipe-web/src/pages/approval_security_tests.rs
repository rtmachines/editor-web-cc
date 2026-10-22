//! Read-only mode and a missing CSRF token on the five new `POST` routes (task 13.3, design D10).
//!
//! The five routes are the approve, the retire, the discard, the delete, and the restore. The tests
//! of `recipe_security_tests` send the matrix bodies of these routes. In the matrix world, the
//! approve, the retire, the delete, and the restore fail at a business rule. These tests use the
//! approval world, so each command would write if the gate did not stop it. A control test proves
//! that each body writes.
//!
//! The tests also read the new query pages in read-only mode. They check the HTMX headers and the
//! empty toast of each HTMX refusal (ADR-0049 (k)).

use std::collections::BTreeSet;

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;

use super::audit::test_world::{AuditWorld, audit_world};
use super::recipe_sweep_tests::TOKEN_ONLY_ROUTES;
use super::test_helpers::{assert_error, assert_refusal_headers};
use crate::test_support::plant::{HTMX, main_of};
use crate::test_support::{Reply, post_fields};

/// The count of characters of an answer that a failure message shows.
const BODY_EXCERPT: usize = 300;
/// The number of commands in `commands()`.
const COMMAND_COUNT: usize = 5;
/// The pages that a read-only server still answers: the preview, the diff, the version page, and
/// the history page.
const RECIPE_PAGES: [&str; 4] = [
    "/recipes/R-500/versions/2/approve",
    "/recipes/R-500/versions/2/diff/1",
    "/recipes/R-500/versions/2",
    "/recipes/R-500/history",
];
/// The other pages that a read-only server still answers: the audit list and the security events.
const LIST_PAGES: [&str; 2] = ["/audit", "/security-events"];

/// One of the five commands with a body that the real use case accepts.
pub(super) struct Command {
    /// The name that a failure message shows.
    pub(super) name: &'static str,
    /// The path of the `POST` route.
    pub(super) path: String,
    /// The form body without the CSRF token.
    pub(super) body: String,
}

/// Where a request carries its CSRF token.
#[derive(Clone, Copy)]
enum Token<'a> {
    /// No token in the header and no token in the body.
    Missing,
    /// The token is the value of the header `X-CSRF-Token`.
    Header(&'a str),
    /// The token is the form field `csrf_token`.
    Field(&'a str),
}

/// Return the five commands that the Administrator `admin` can run in the approval world.
///
/// The approve and the discard act on the draft version 2 of `R-500`. The retire acts on the
/// approved version 1. The delete acts on `R-200`, which has no approved version. The restore acts
/// on the deleted recipe `R-300`. The tests of `lifecycle_body_tests.rs` also read this table.
pub(super) async fn commands(world: &AuditWorld) -> Vec<Command> {
    let draft = format!("row_version={}", world.data.r500_v2_row_version);
    let approved = format!("row_version={}", world.row_version_of("R-500", 1).await);
    let command = |name: &'static str, path: &str, body: &str| Command {
        name,
        path: path.to_owned(),
        body: body.to_owned(),
    };
    vec![
        command("approve", "/recipes/R-500/versions/2/approve", &draft),
        command("retire", "/recipes/R-500/versions/1/retire", &approved),
        command("discard", "/recipes/R-500/versions/2/discard", &draft),
        command("delete", "/recipes/R-200/delete", ""),
        command("restore", "/recipes/R-300/restore", ""),
    ]
}

/// Return the command with the position `index` of `commands()`.
async fn command_of(world: &AuditWorld, index: usize) -> Command {
    let found = commands(world).await.into_iter().nth(index);
    found.unwrap()
}

/// Send the command as `admin` with a same-origin `POST` and the token as `token` says.
///
/// The function passes no session to `post_fields`, because that function would add the token
/// header. It sends the cookie of `admin` as a header instead.
async fn send(world: &AuditWorld, command: &Command, token: Token<'_>, htmx: bool) -> Reply {
    let mut headers = vec![("cookie", world.data.admin.cookie.as_str())];
    if htmx {
        headers.push(HTMX);
    }
    let mut body = command.body.clone();
    match token {
        Token::Missing => {}
        Token::Header(value) => headers.push(("x-csrf-token", value)),
        Token::Field(value) => {
            let separator = if body.is_empty() { "" } else { "&" };
            body = format!("{body}{separator}csrf_token={value}");
        }
    }
    let router = world.fixture.router();
    post_fields(&router, &command.path, None, &headers, &body).await
}

/// Assert `403 E-WEB-007`, the HTMX headers and toast, one new `csrf_rejected` event, and no
/// change of the data.
async fn assert_csrf_refused(world: &AuditWorld, command: &Command, how: (Token<'_>, bool)) {
    let (token, htmx) = how;
    let what = format!("{} htmx={htmx}", command.name);
    let before = world.fixture.snapshot();
    let events = world.fixture.sink.events().len();
    let reply = send(world, command, token, htmx).await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-007", &what);
    assert_refusal_headers(&reply, htmx, &what);
    assert_eq!(world.fixture.snapshot(), before, "{what}");
    let after = world.fixture.sink.events();
    assert_eq!(after.len(), events.checked_add(1).unwrap(), "{what}");
    let last = after.last().map(|event| event.kind());
    assert_eq!(last, Some(SecurityEventKind::CsrfRejected), "{what}");
}

/// Assert `503 E-WEB-006`, the HTMX headers and toast, no change of the data, and no security
/// event.
async fn assert_read_only_refused(world: &AuditWorld, command: &Command, htmx: bool) {
    let what = format!("{} htmx={htmx}", command.name);
    let before = world.fixture.snapshot();
    let events = world.fixture.sink.events().len();
    let token = Token::Header(&world.data.admin.csrf);
    let reply = send(world, command, token, htmx).await;
    assert_error(&reply, StatusCode::SERVICE_UNAVAILABLE, "E-WEB-006", &what);
    assert_refusal_headers(&reply, htmx, &what);
    assert_eq!(world.fixture.snapshot(), before, "{what}");
    assert_eq!(world.fixture.sink.events().len(), events, "{what}");
}

/// Assert that the command passes the gates and writes. The answer is `200` or `303`.
async fn assert_writes(index: usize, field: bool) {
    let world = audit_world().await;
    let command = command_of(&world, index).await;
    let value = world.data.admin.csrf.as_str();
    let token = if field {
        Token::Field(value)
    } else {
        Token::Header(value)
    };
    let before = world.fixture.snapshot();
    let reply = send(&world, &command, token, false).await;
    let what = format!("{} field={field}", command.name);
    let start: String = reply.body.chars().take(BODY_EXCERPT).collect();
    let success = [StatusCode::OK, StatusCode::SEE_OTHER];
    assert!(success.contains(&reply.status), "{what}: {start}");
    assert_ne!(world.fixture.snapshot(), before, "{what} wrote nothing");
}

#[tokio::test]
async fn approval_security_the_five_commands_write_with_the_token_in_the_header() {
    for index in 0..COMMAND_COUNT {
        assert_writes(index, false).await;
    }
}

#[tokio::test]
async fn approval_security_the_five_commands_write_with_the_token_in_the_form_field() {
    for index in 0..COMMAND_COUNT {
        assert_writes(index, true).await;
    }
}

#[tokio::test]
async fn approval_security_the_command_list_holds_every_token_only_route() {
    let world = audit_world().await;
    let last_segment = |path: &str| path.rsplit('/').next().unwrap_or_default().to_owned();
    let table = commands(&world).await;
    let segments: Vec<String> = table
        .iter()
        .map(|command| last_segment(&command.path))
        .collect();
    let listed: BTreeSet<String> = segments.iter().cloned().collect();
    let known: BTreeSet<String> = TOKEN_ONLY_ROUTES.into_iter().map(last_segment).collect();
    assert_eq!(listed, known);
    assert_eq!(segments.len(), COMMAND_COUNT, "a route stands twice");
    assert_eq!(TOKEN_ONLY_ROUTES.len(), COMMAND_COUNT);
    for command in &table {
        assert_eq!(
            last_segment(&command.path),
            command.name,
            "{}",
            command.path
        );
    }
}

#[tokio::test]
async fn approval_security_a_post_without_a_token_gives_403_and_writes_nothing() {
    let world = audit_world().await;
    for command in commands(&world).await {
        for htmx in [false, true] {
            assert_csrf_refused(&world, &command, (Token::Missing, htmx)).await;
        }
    }
}

#[tokio::test]
async fn approval_security_a_token_of_another_session_in_the_header_does_not_count() {
    let world = audit_world().await;
    let other = world.data.anowak.csrf.as_str();
    for command in commands(&world).await {
        for htmx in [false, true] {
            assert_csrf_refused(&world, &command, (Token::Header(other), htmx)).await;
        }
    }
}

#[tokio::test]
async fn approval_security_a_token_of_another_session_in_the_form_field_does_not_count() {
    let world = audit_world().await;
    let other = world.data.anowak.csrf.as_str();
    for command in commands(&world).await {
        for htmx in [false, true] {
            assert_csrf_refused(&world, &command, (Token::Field(other), htmx)).await;
        }
    }
}

#[tokio::test]
async fn approval_security_read_only_mode_gives_503_and_writes_nothing() {
    let world = audit_world().await;
    let list = commands(&world).await;
    world.fixture.state.read_only().set(true);
    for command in list {
        for htmx in [false, true] {
            assert_read_only_refused(&world, &command, htmx).await;
        }
    }
}

#[tokio::test]
async fn approval_security_read_only_mode_keeps_the_new_query_pages_available() {
    let world = audit_world().await;
    let entry = first_entry_path(&world).await;
    world.fixture.state.read_only().set(true);
    let pages = RECIPE_PAGES.map(str::to_owned).into_iter();
    let pages = pages.chain(LIST_PAGES.map(str::to_owned));
    for path in pages.chain([entry]) {
        let session = &world.data.admin;
        let page = world.get_en(session, &path).await;
        assert_eq!(page.status, StatusCode::OK, "GET {path}: {}", page.body);
        let content = world.get_content(session, &path).await;
        assert_eq!(content.status, StatusCode::OK, "GET {path} content");
    }
}

/// Return the path of the first entry that the audit list shows.
async fn first_entry_path(world: &AuditWorld) -> String {
    let list = world.get_en(&world.data.admin, "/audit").await;
    let main = main_of(&list.body);
    let id = main
        .split("href=\"/audit/")
        .nth(1)
        .and_then(|rest| rest.split('"').next());
    format!("/audit/{}", id.unwrap())
}
