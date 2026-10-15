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
use crate::test_support::{get_with, post_fields};

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
