//! Tests of the two export routes for the six principals (design D11 T12, ADR-0053 (a), ADR-0013).
//!
//! The matrix world of `routes::matrix` seeds `seed_recipes` only, so an allowed cell there gives
//! `E-EXP-002` or `E-EXP-003`. These tests build a local world with `seed_approval_recipes`. Then
//! `R-500` has the approved version 1, and `R-600` has the retired version 1. Both have a real hash.

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::Role;

use super::tests::exports;
use crate::routes::matrix::{PRINCIPALS, Principal, Want, assert_want};
use crate::test_support::{Fixture, Reply, TestSession, deactivate_user, get_with};

/// The password of every account of the local world.
const PASSWORD: &str = "correct horse battery";
/// The approved export of `R-500`.
const R500_APPROVED: &str = "/export/v1/recipes/R-500/approved";
/// The export of the approved version 1 of `R-500` by its number.
const R500_V1: &str = "/export/v1/recipes/R-500/versions/1";
/// The export of the retired version 1 of `R-600` by its number.
const R600_V1: &str = "/export/v1/recipes/R-600/versions/1";
/// The file name of the approved version 1 of `R-500`.
const R500_FILE: &str = "recipe_R-500_v1.json";
/// The file name of the retired version 1 of `R-600`.
const R600_FILE: &str = "RETIRED_recipe_R-600_v1.json";

/// The expected answer of one cell.
#[derive(Clone, Copy, Debug)]
enum Cell {
    /// `200` with this file name and one new `VersionExported` entry.
    File(&'static str),
    /// `403 E-WEB-002` as JSON, one `forbidden` event, and no new entry.
    Forbidden,
    /// A `303` that `assert_want` names, and no new entry.
    Redirect(Want),
}

/// Build a fixture with the five accounts of `routes::matrix::world` and the recipes of D14.
///
/// Return the session of `principal`. `NoSession` has none. `Deactivated` has a session, and the
/// fixture deactivates its user after the session starts.
async fn local_world(principal: Principal) -> (Fixture, Option<TestSession>) {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Administrator, PASSWORD, false);
    fixture.seed_account(2, Role::ProcessEngineer, PASSWORD, false);
    fixture.seed_account(3, Role::Operator, PASSWORD, false);
    fixture.seed_account(4, Role::Administrator, PASSWORD, true);
    fixture.seed_account(5, Role::Administrator, PASSWORD, false);
    fixture.seed_approval_recipes().await;
    let session = match principal {
        Principal::NoSession => None,
        Principal::Administrator => Some(fixture.session_of(1)),
        Principal::ProcessEngineer => Some(fixture.session_of(2)),
        Principal::Operator => Some(fixture.session_of(3)),
        Principal::MustChange => Some(fixture.session_of(4)),
        Principal::Deactivated => {
            let session = fixture.session_of(5);
            deactivate_user(&fixture, "user5").await;
            Some(session)
        }
    };
    (fixture, session)
}

/// Assert a file answer with the name `file_name`.
fn assert_file(reply: &Reply, file_name: &str, what: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json", "{what}");
    let disposition = format!("attachment; filename=\"{file_name}\"");
    assert_eq!(reply.header("content-disposition"), disposition, "{what}");
}

/// Assert the JSON error body of ADR-0053 (a) with `403 E-WEB-002`.
fn assert_forbidden_json(reply: &Reply, what: &str) {
    assert_eq!(
        reply.status,
        StatusCode::FORBIDDEN,
        "{what}: {}",
        reply.body
    );
    assert_eq!(reply.header("content-type"), "application/json", "{what}");
    assert_eq!(reply.header("content-disposition"), "", "{what}");
    let start = "{\"code\":\"E-WEB-002\",\"message\":\"";
    assert!(reply.body.starts_with(start), "{what}: {}", reply.body);
    let request_id = format!(",\"request_id\":\"{}\"}}", reply.header("x-request-id"));
    assert!(reply.body.ends_with(&request_id), "{what}: {}", reply.body);
}

/// Assert that the sink holds one event more than `events_before`, and that it is `forbidden`.
fn assert_one_forbidden_event(fixture: &Fixture, events_before: usize, what: &str) {
    let events = fixture.sink.events();
    assert_eq!(
        events.len(),
        events_before.checked_add(1).unwrap(),
        "{what}"
    );
    assert!(
        events
            .last()
            .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden),
        "{what}"
    );
}

/// Send `GET path` as `principal` in a fresh local world and assert the answer of `cell`.
async fn check_cell(path: &str, principal: Principal, cell: Cell) {
    let (fixture, session) = local_world(principal).await;
    let cookies: Vec<(&str, &str)> = session
        .iter()
        .map(|session| ("cookie", session.cookie.as_str()))
        .collect();
    let what = format!("GET {path} as {principal:?}");
    let entries_before = exports(&fixture).len();
    let events_before = fixture.sink.events().len();
    let reply = get_with(&fixture.router(), path, &cookies).await;
    let new_entries = match cell {
        Cell::File(file_name) => {
            assert_file(&reply, file_name, &what);
            1
        }
        Cell::Forbidden => {
            assert_forbidden_json(&reply, &what);
            assert_one_forbidden_event(&fixture, events_before, &what);
            0
        }
        Cell::Redirect(want) => {
            assert_want(&reply, want, &what);
            0
        }
    };
    let expected = entries_before.checked_add(new_entries).unwrap();
    assert_eq!(exports(&fixture).len(), expected, "{what}: entries");
}

/// Check one row of the matrix: the cell of `cells` for each principal of `PRINCIPALS`.
async fn check_row(path: &str, cells: [Cell; 6]) {
    for (principal, cell) in PRINCIPALS.into_iter().zip(cells) {
        check_cell(path, principal, cell).await;
    }
}

/// The six cells of a route that needs `VersionExportHistory`.
///
/// Only the Operator gets `403 E-WEB-002`, because the role has `VersionExport` but not
/// `VersionExportHistory`.
fn operator_refused(file_name: &'static str) -> [Cell; 6] {
    use Cell::{File, Forbidden, Redirect};
    let (login, password) = (Redirect(Want::Login), Redirect(Want::Password));
    [
        login,
        File(file_name),
        File(file_name),
        Forbidden,
        password,
        login,
    ]
}

/// Catches a route that maps a principal to the wrong answer on `/approved`.
///
/// All three roles hold `VersionExport`, so each gets the file and one entry. The session layer
/// and the password gate stop the other three principals, so each gets a redirect and no entry.
#[tokio::test]
async fn export_approved_route_gives_the_file_to_every_role_and_a_redirect_to_the_rest() {
    use Cell::{File, Redirect};
    let (login, password) = (Redirect(Want::Login), Redirect(Want::Password));
    let file = File(R500_FILE);
    check_row(R500_APPROVED, [login, file, file, file, password, login]).await;
}

/// Catches a route that checks `VersionExport` where it needs `VersionExportHistory`.
///
/// The Operator lacks the history permission, so the answer is `403 E-WEB-002` as JSON with one
/// `forbidden` event. The test also fails when the file or the entry appears for the Operator.
#[tokio::test]
async fn export_version_route_refuses_the_operator_for_a_retired_version() {
    check_row(R600_V1, operator_refused(R600_FILE)).await;
}

/// Catches a refusal that depends on the state of the version and not on the permission.
///
/// The version 1 of `R-500` is `Approved`, and the Operator may export it by `/approved`. The
/// route by number still refuses the Operator, so the permission is the cause of the refusal.
#[tokio::test]
async fn export_version_route_refuses_the_operator_for_an_approved_version_too() {
    check_row(R500_V1, operator_refused(R500_FILE)).await;
}
