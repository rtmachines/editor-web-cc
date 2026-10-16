//! The `forbidden` event of the plant routes: `api.md` sections 2.2 and 2.3 (task 11.4, D20 U19).
//!
//! This module is test code. The area rows of the matrix in `routes::tests` already check each
//! route and each of the six principals (design D17). So this module keeps only what the matrix
//! lacks: a refused write route records one `forbidden` security event, and the body holds the
//! code `E-WEB-002`. It reads the rows of the three areas, so it holds no second list of routes.

use std::collections::BTreeSet;

use axum::http::{Method, StatusCode};
use recipe_domain::audit::SecurityEventKind;

use super::matrix::{Endpoint, PRINCIPALS, Principal, Want, assert_error, fill, world};
use crate::pages::{machines, types, units};
use crate::test_support::{Fixture, get_with, post_fields};

/// The number of the routes of the machine, unit, and type pages (`api.md` sections 2.2 and 2.3).
const PLANT_ROUTES: usize = 19;

/// Return the matrix rows of the three areas, each with the answer of the six principals.
fn area_rows() -> Vec<(Endpoint, [Want; 6])> {
    let mut rows = machines::route_tests::route_rows();
    rows.extend(units::route_tests::route_rows());
    rows.extend(types::route_tests::route_rows());
    rows
}

/// Return the rows whose answer for the Operator is `403`: the routes that need `MachineManage`,
/// `UnitManage`, or `UdtManage`.
fn forbidden_rows() -> Vec<Endpoint> {
    let operator = PRINCIPALS
        .iter()
        .position(|principal| matches!(principal, Principal::Operator))
        .unwrap();
    area_rows()
        .into_iter()
        .filter(|(_endpoint, wants)| matches!(wants.get(operator), Some(Want::Forbidden)))
        .map(|(endpoint, _wants)| endpoint)
        .collect()
}

/// Return the method and the path of each route that the three areas register.
fn registered_routes() -> BTreeSet<(String, &'static str)> {
    let mut rows = machines::rows();
    rows.extend(units::rows());
    rows.extend(types::rows());
    rows.into_iter()
        .map(|row| (row.method.to_string(), row.path))
        .collect()
}

#[test]
fn plant_access_the_area_rows_of_the_matrix_hold_every_route_of_the_three_areas() {
    let matrix: BTreeSet<(String, &'static str)> = area_rows()
        .iter()
        .map(|(endpoint, _wants)| (endpoint.method.to_string(), endpoint.path))
        .collect();
    assert_eq!(matrix, registered_routes());
    assert_eq!(matrix.len(), PLANT_ROUTES);
}

/// The write routes of the plant forms (ADR-0046 (c)): the path, a body with a bad field, a body
/// with a field that the form does not know, and the command that the `forbidden` event names.
const FORM_ROUTES: [(&str, &str, &str, &str); 6] = [
    (
        "/machines",
        "code=&name=&plc_platform=&main_unit_name=",
        "code=M50&name=A&plc_platform=S7-1500&main_unit_name=B&extra=1",
        "CreateMachine",
    ),
    (
        "/machines/{code}/rename",
        "name=",
        "name=A&extra=1",
        "RenameMachine",
    ),
    (
        "/machines/{code}/units",
        "parent_path=&segment=x&name=",
        "parent_path=&segment=3&name=A&extra=1",
        "AddUnit",
    ),
    (
        "/machines/{code}/units/{path}/rename",
        "name=",
        "name=A&extra=1",
        "RenameUnit",
    ),
    (
        "/machines/{code}/types",
        "name=",
        "name=ST_New&extra=1",
        "DefineUdt",
    ),
    (
        "/machines/{code}/types/{name}/revisions",
        "members%5B0%5D.name=&members%5B0%5D.type=",
        "extra=1",
        "DefineUdt",
    ),
];

/// The form pages of the plant and the command of the form that each page shows (ADR-0046 (b)).
const FORM_PAGES: [(&str, &str); 3] = [
    ("/machines/new", "CreateMachine"),
    ("/machines/{code}/units/new", "AddUnit"),
    ("/machines/{code}/types/new", "DefineUdt"),
];

/// Return the security events of `fixture` as `(kind, detail)` texts.
fn event_texts(fixture: &Fixture) -> Vec<String> {
    fixture
        .sink
        .events()
        .iter()
        .map(|event| {
            format!(
                "{:?} {:?}",
                event.kind(),
                event.detail().map(ToString::to_string)
            )
        })
        .collect()
}

/// Return the text of the one `forbidden` event that names `command`.
fn forbidden_text(command: &str) -> String {
    let detail = format!("{{\"operation\":\"{command}\"}}");
    format!("{:?} {:?}", SecurityEventKind::Forbidden, Some(detail))
}

/// Return the state of the store and the count of the audit entries.
fn store_state(fixture: &Fixture) -> (String, usize) {
    (
        format!("{:?}", fixture.store.state()),
        fixture.store.audit_entries().len(),
    )
}

/// Send `body` as an HTMX `POST` to `route` as the Operator (ADR-0046 (a)).
///
/// Assert `403 E-WEB-002`, one `forbidden` event that names `command`, no form, and no write.
pub(super) async fn assert_refused_by_its_command(route: &str, body: &str, command: &str) {
    let (fixture, session) = world(Principal::Operator).await;
    let path = fill(route);
    let what = format!("POST {path} with {body}");
    let before = store_state(&fixture);
    let events = event_texts(&fixture);
    let headers = [("hx-request", "true")];
    let reply = post_fields(
        &fixture.router(),
        &path,
        session.as_ref(),
        &headers,
        &fill(body),
    )
    .await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", &what);
    let mut expected = events;
    expected.push(forbidden_text(command));
    assert_eq!(event_texts(&fixture), expected, "{what}");
    assert!(!reply.body.contains("<form"), "{what}: {}", reply.body);
    assert_eq!(store_state(&fixture), before, "{what}");
}

/// Open the form page `route` as the Operator and assert one `forbidden` event that names `command`.
pub(super) async fn assert_page_refused_by_its_command(route: &str, command: &str) {
    let (fixture, session) = world(Principal::Operator).await;
    let session = session.unwrap();
    let path = fill(route);
    let reply = get_with(
        &fixture.router(),
        &path,
        &[("cookie", session.cookie.as_str())],
    )
    .await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", &path);
    let events = event_texts(&fixture);
    assert_eq!(events.last(), Some(&forbidden_text(command)), "{path}");
    assert!(!reply.body.contains("<form"), "{path}: {}", reply.body);
}

/// The form pages of the plant with a bad query value or an unknown query key, and the command
/// of the form that each page shows (ADR-0046 (a)).
const FORM_PAGE_QUERIES: [(&str, &str); 5] = [
    ("/machines/new?extra=1", "CreateMachine"),
    ("/machines/{code}/units/new?parent_path=x%21", "AddUnit"),
    ("/machines/{code}/units/new?extra=1", "AddUnit"),
    ("/machines/{code}/types/new?from=1bad", "DefineUdt"),
    ("/machines/{code}/types/new?extra=1", "DefineUdt"),
];

/// The same pages for a role with the permission: the status and the code of today.
const FORM_PAGE_QUERY_ANSWERS: [(&str, StatusCode, &str); 4] = [
    (
        "/machines/{code}/units/new?parent_path=x%21",
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-017",
    ),
    (
        "/machines/{code}/units/new?extra=1",
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    ),
    (
        "/machines/{code}/types/new?from=1bad",
        StatusCode::NOT_FOUND,
        "E-WEB-005",
    ),
    (
        "/machines/{code}/types/new?extra=1",
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    ),
];

/// Open the form page `route` with its query string as the Operator (ADR-0046 (a)).
///
/// Assert `403 E-WEB-002`, exactly one new `forbidden` event that names `command`, and no form.
pub(super) async fn assert_query_refused_by_its_command(route: &str, command: &str) {
    let (fixture, session) = world(Principal::Operator).await;
    let session = session.unwrap();
    let path = fill(route);
    let mut expected = event_texts(&fixture);
    expected.push(forbidden_text(command));
    let cookie = [("cookie", session.cookie.as_str())];
    let reply = get_with(&fixture.router(), &path, &cookie).await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", &path);
    assert_eq!(event_texts(&fixture), expected, "{path}");
    assert!(!reply.body.contains("<form"), "{path}: {}", reply.body);
}

/// Open the form page `route` as the ProcessEngineer and assert `status` and `code`.
pub(super) async fn assert_query_answer(route: &str, status: StatusCode, code: &str) {
    let (fixture, session) = world(Principal::ProcessEngineer).await;
    let session = session.unwrap();
    let path = fill(route);
    let cookie = [("cookie", session.cookie.as_str())];
    let reply = get_with(&fixture.router(), &path, &cookie).await;
    assert_error(&reply, status, code, &path);
    let forbidden = event_texts(&fixture)
        .iter()
        .any(|text| text.starts_with(&format!("{:?}", SecurityEventKind::Forbidden)));
    assert!(!forbidden, "{path}");
}

#[tokio::test]
async fn plant_access_a_form_page_with_a_bad_query_refuses_the_operator_with_the_event() {
    for (route, command) in FORM_PAGE_QUERIES {
        assert_query_refused_by_its_command(route, command).await;
    }
}

#[tokio::test]
async fn plant_access_a_form_page_with_a_bad_query_keeps_the_answer_of_a_process_engineer() {
    for (route, status, code) in FORM_PAGE_QUERY_ANSWERS {
        assert_query_answer(route, status, code).await;
    }
}

#[tokio::test]
async fn plant_access_an_operator_with_a_bad_field_gets_403_with_the_event_of_the_command() {
    for (route, bad, _unknown, command) in FORM_ROUTES {
        assert_refused_by_its_command(route, bad, command).await;
    }
}

#[tokio::test]
async fn plant_access_an_operator_with_an_unknown_field_gets_403_with_the_event_of_the_command() {
    for (route, _bad, unknown, command) in FORM_ROUTES {
        assert_refused_by_its_command(route, unknown, command).await;
    }
}

#[tokio::test]
async fn plant_access_a_form_page_refuses_the_operator_with_the_event_of_its_command() {
    for (route, command) in FORM_PAGES {
        assert_page_refused_by_its_command(route, command).await;
    }
}

#[tokio::test]
async fn plant_access_a_process_engineer_with_a_bad_field_still_gets_the_lines() {
    for (route, bad, _unknown, _command) in FORM_ROUTES {
        let (fixture, session) = world(Principal::ProcessEngineer).await;
        let path = fill(route);
        let headers = [("hx-request", "true")];
        let reply = post_fields(&fixture.router(), &path, session.as_ref(), &headers, bad).await;
        let what = format!("POST {path} with {bad}");
        assert_error(
            &reply,
            StatusCode::UNPROCESSABLE_ENTITY,
            "field-lines",
            &what,
        );
        assert!(reply.body.contains("<form"), "{what}: {}", reply.body);
    }
}

#[tokio::test]
async fn plant_access_a_forbidden_answer_records_one_forbidden_event() {
    let rows = forbidden_rows();
    assert_eq!(rows.len(), 11, "the write routes of the three areas");
    for endpoint in rows {
        let (fixture, session) = world(Principal::Operator).await;
        let session = session.unwrap();
        let router = fixture.router();
        let path = fill(endpoint.path);
        let body = fill(endpoint.body);
        let what = format!("{} {path} as Operator", endpoint.method);
        let events_before = fixture.sink.events().len();
        let reply = if endpoint.method == Method::GET {
            get_with(&router, &path, &[("cookie", session.cookie.as_str())]).await
        } else {
            post_fields(&router, &path, Some(&session), &[], &body).await
        };
        assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", &what);
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
}
