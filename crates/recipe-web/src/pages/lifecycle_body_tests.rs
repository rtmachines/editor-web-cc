//! The five lifecycle `POST` routes read the body before the permission check (ADR-0049 (l),
//! finding RCW-2).
//!
//! The five routes are the approve, the retire, the discard, the delete, and the restore. A body
//! with an unknown field gives `422 E-WEB-008` for every role, and the route writes nothing. An
//! Operator gets no `forbidden` event and no data of the recipe in that answer. A well-formed body
//! of the Operator reaches the command and gives `403 E-WEB-002` with one `forbidden` event.
//!
//! The tests use the approval world. The Administrator writes with a well-formed body. Each case
//! runs as an HTMX request and as a plain request.
//!
//! The route table is `commands()` of `approval_security_tests.rs`. The guard test of that file
//! checks that the table holds the five routes once. The header check is
//! `assert_refusal_headers` of `test_helpers.rs`.

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;

use super::approval_security_tests::{Command, commands};
use super::audit::test_world::{AuditWorld, audit_world};
use super::test_helpers::{TOAST_CLEAR, assert_error, assert_refusal_headers};
use crate::test_support::plant::HTMX;
use crate::test_support::{Reply, TestSession, post_fields};

/// The field that no form of the five routes has.
const UNKNOWN_FIELD: &str = "x=1";

/// Return the body of `route` with the unknown field added.
fn malformed(route: &Command) -> String {
    let separator = if route.body.is_empty() { "" } else { "&" };
    format!("{}{separator}{UNKNOWN_FIELD}", route.body)
}

/// Send `body` to `route` as `session`. The session adds its cookie and its CSRF header.
///
/// When `htmx` is true, the request also sends the header `HX-Request`.
async fn send(
    world: &AuditWorld,
    session: &TestSession,
    route: &Command,
    body: &str,
    htmx: bool,
) -> Reply {
    let headers: &[(&str, &str)] = if htmx { &[HTMX] } else { &[] };
    let router = world.fixture.router();
    post_fields(&router, &route.path, Some(session), headers, body).await
}

/// Assert the headers, the toast, and the page shape of a refusal.
///
/// An HTMX request gets the fragment with one empty toast at the end. A plain request gets the
/// full error page (`api.md` section 5.3).
fn assert_refusal(reply: &Reply, htmx: bool, what: &str) {
    assert_refusal_headers(reply, htmx, what);
    let has_page = reply.body.contains("<!DOCTYPE html>");
    assert_eq!(has_page, !htmx, "{what}: {}", reply.body);
    if htmx {
        assert!(reply.body.ends_with(TOAST_CLEAR), "{what}: {}", reply.body);
    } else {
        assert!(
            reply.body.contains("<main id=\"content\">"),
            "{what}: {}",
            reply.body
        );
    }
}

/// Return the count of the `forbidden` events that the sink recorded.
fn forbidden_count(world: &AuditWorld) -> usize {
    let events = world.fixture.sink.events();
    let forbidden = events
        .iter()
        .filter(|event| event.kind() == SecurityEventKind::Forbidden);
    forbidden.count()
}

/// Return the recipe reference that each route of `table` names. The `422` answers must hide it.
///
/// The reference is the second segment of the path, for example `R-500`.
fn references(table: &[Command]) -> Vec<&str> {
    let routes = table.iter();
    routes
        .filter_map(|route| route.path.split('/').nth(2))
        .collect()
}

/// Assert that an answer shows no form and no data of the recipes `references` (ADR-0049 (l)).
fn assert_no_recipe_data(reply: &Reply, references: &[&str], what: &str) {
    let body = &reply.body;
    assert!(!body.contains("<form"), "{what}: {body}");
    assert!(!body.contains("name=\"row_version\""), "{what}: {body}");
    for reference in references {
        assert!(
            !body.contains(reference),
            "{what} shows {reference}: {body}"
        );
    }
}

/// Assert `422 E-WEB-008` for an unknown field. Assert also no form, no data of a recipe, no
/// write, and no security event. `hidden` holds the references that the answer must not show.
/// `case` names the case in each failure message.
async fn assert_malformed_refused(
    world: &AuditWorld,
    session: &TestSession,
    route: &Command,
    hidden: &[&str],
    case: &str,
) {
    for htmx in [true, false] {
        let what = format!("{case} {} unknown field htmx={htmx}", route.name);
        let (before, events) = (world.fixture.snapshot(), world.fixture.sink.events().len());
        let forbidden = forbidden_count(world);
        let body = malformed(route);
        let reply = send(world, session, route, &body, htmx).await;
        assert_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", &what);
        assert_refusal(&reply, htmx, &what);
        assert_no_recipe_data(&reply, hidden, &what);
        assert_eq!(world.fixture.snapshot(), before, "{what}");
        assert_eq!(world.fixture.sink.events().len(), events, "{what}");
        assert_eq!(forbidden_count(world), forbidden, "{what}");
    }
}

/// Assert `403 E-WEB-002` for a well-formed body and exactly one new `forbidden` event.
/// `case` names the case in each failure message.
async fn assert_forbidden(world: &AuditWorld, session: &TestSession, route: &Command, case: &str) {
    for htmx in [true, false] {
        let what = format!("{case} {} well-formed htmx={htmx}", route.name);
        let (before, forbidden) = (world.fixture.snapshot(), forbidden_count(world));
        let reply = send(world, session, route, &route.body, htmx).await;
        assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", &what);
        assert_refusal(&reply, htmx, &what);
        assert_eq!(world.fixture.snapshot(), before, "{what}");
        let expected = forbidden.checked_add(1).unwrap();
        assert_eq!(forbidden_count(world), expected, "{what}");
    }
}

#[tokio::test]
async fn lifecycle_body_an_unknown_field_gives_422_before_the_permission_check() {
    let world = audit_world().await;
    let table = commands(&world).await;
    let hidden = references(&table);
    for route in &table {
        // Case 1: the Administrator has the permission. The unknown field stops the request.
        let admin = &world.data.admin;
        assert_malformed_refused(&world, admin, route, &hidden, "case 1 Administrator").await;
        // Case 2: the Operator has no permission. The route reads the body first.
        let operator = &world.operator;
        assert_malformed_refused(&world, operator, route, &hidden, "case 2 Operator").await;
        // Case 3: a well-formed body reaches the command. The command refuses the Operator.
        assert_forbidden(&world, operator, route, "case 3 Operator").await;
    }
}
