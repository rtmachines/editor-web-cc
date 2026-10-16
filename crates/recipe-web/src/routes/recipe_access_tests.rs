//! The `forbidden` event of the recipe, version, and parameter routes: the permission matrix of
//! `api.md` section 3 (task 15.1).
//!
//! This module is test code. The area rows of the matrix in `routes::tests` already check each
//! route and each of the six principals (design D17). So this module keeps only what the matrix
//! lacks: a refused write route records one `forbidden` security event, and the body holds the
//! code `E-WEB-002`. It reads the rows of the three areas, so it holds no second list of routes.

use axum::http::{Method, StatusCode};
use recipe_app::auth::{Operation, Requirement};
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::Permission;

use super::matrix::{Endpoint, PRINCIPALS, Principal, Want, assert_error, fill, world};
use super::plant_access_tests::{
    assert_page_refused_by_its_command, assert_query_answer, assert_query_refused_by_its_command,
    assert_refused_by_its_command,
};
use crate::pages::{parameters, recipes, versions};
use crate::test_support::{get_with, post_fields};

/// The number of the routes of the recipe, version, and parameter pages that the Operator refuses.
const REFUSED_ROUTES: usize = 19;

/// Return the matrix rows of the three areas, each with the answer of the six principals.
fn area_rows() -> Vec<(Endpoint, [Want; 6])> {
    let mut rows = recipes::route_tests::route_rows();
    rows.extend(versions::route_tests::route_rows());
    rows.extend(parameters::route_tests::route_rows());
    rows
}

/// Return the rows whose answer for the Operator is `403`: the routes that need `RecipeEdit`,
/// `VersionEdit`, or `ParameterEdit`.
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

/// A draft body with a bad field: a `row_version` that is not a number.
const BAD_ROW: &str = "row_version=x";
/// A draft body with a field that no draft form knows.
const EXTRA: &str = "row_version=1&extra=1";

/// The write routes of the recipe, version, and parameter forms (ADR-0046 (c)): the path, a body
/// with a bad field, a body with a field that the form does not know, and the command that the
/// `forbidden` event names. `{key}` is the parameter `LINE` of the live draft.
const FORM_ROUTES: [(&str, &str, &str, &str); 11] = [
    (
        "/recipes",
        "machine_code=&reference_number=&name=&change_note=",
        "machine_code=M7&reference_number=R-200&name=A&change_note=B&extra=1",
        "CreateRecipe",
    ),
    (
        "/recipes/{ref}/rename",
        "name=",
        "name=A&extra=1",
        "RenameRecipe",
    ),
    (
        "/recipes/{ref}/versions",
        "based_on=x&change_note=",
        "change_note=A&extra=1",
        "CreateVersion",
    ),
    (
        "/recipes/{ref}/versions/{n}/rebind",
        BAD_ROW,
        EXTRA,
        "RebindTypeRevision",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters",
        BAD_ROW,
        EXTRA,
        "AddParameter",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/definition",
        BAD_ROW,
        EXTRA,
        "UpdateParameterDefinition",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/value",
        BAD_ROW,
        EXTRA,
        "SetParameterValue",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
        BAD_ROW,
        EXTRA,
        "ReplaceLimits",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}",
        BAD_ROW,
        "row_version=1&name=A&description=B&extra=1",
        "UpsertTranslation",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}/delete",
        BAD_ROW,
        EXTRA,
        "RemoveTranslation",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/delete",
        BAD_ROW,
        EXTRA,
        "RemoveParameter",
    ),
];

/// The form pages of the recipe area and the command of the form that each page shows.
const FORM_PAGES: [(&str, &str); 3] = [
    ("/recipes/new", "CreateRecipe"),
    ("/recipes/{ref}/versions/new", "CreateVersion"),
    ("/recipes/{ref}/versions/{n}/parameters/new", "AddParameter"),
];

/// The form pages of the recipe area with a bad query value or an unknown query key, and the
/// command of the form that each page shows (ADR-0046 (a)).
const FORM_PAGE_QUERIES: [(&str, &str); 5] = [
    ("/recipes/new?extra=1", "CreateRecipe"),
    ("/recipes/{ref}/versions/new?based_on=x", "CreateVersion"),
    ("/recipes/{ref}/versions/new?extra=1", "CreateVersion"),
    (
        "/recipes/{ref}/versions/{n}/parameters/new?unit=x%21",
        "AddParameter",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/new?extra=1",
        "AddParameter",
    ),
];

/// The form of a new parameter for a role with the permission: the status and the code of today.
const FORM_PAGE_QUERY_ANSWERS: [(&str, StatusCode, &str); 2] = [
    (
        "/recipes/{ref}/versions/{n}/parameters/new?unit=x%21",
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-017",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/new?extra=1",
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    ),
];

#[tokio::test]
async fn recipe_access_a_form_page_with_a_bad_query_refuses_the_operator_with_the_event() {
    for (route, command) in FORM_PAGE_QUERIES {
        assert_query_refused_by_its_command(route, command).await;
    }
}

#[tokio::test]
async fn recipe_access_a_form_page_with_a_bad_query_keeps_the_answer_of_a_process_engineer() {
    for (route, status, code) in FORM_PAGE_QUERY_ANSWERS {
        assert_query_answer(route, status, code).await;
    }
}

#[tokio::test]
async fn recipe_access_an_operator_with_a_bad_field_gets_403_with_the_event_of_the_command() {
    for (route, bad, _unknown, command) in FORM_ROUTES {
        assert_refused_by_its_command(route, bad, command).await;
    }
}

#[tokio::test]
async fn recipe_access_an_operator_with_an_unknown_field_gets_403_with_the_event_of_the_command() {
    for (route, _bad, unknown, command) in FORM_ROUTES {
        assert_refused_by_its_command(route, unknown, command).await;
    }
}

#[tokio::test]
async fn recipe_access_a_form_page_refuses_the_operator_with_the_event_of_its_command() {
    for (route, command) in FORM_PAGES {
        assert_page_refused_by_its_command(route, command).await;
    }
}

#[tokio::test]
async fn recipe_access_the_rename_of_a_recipe_checks_recipe_edit() {
    // ADR-0046 (c): the event names `RenameRecipe`, whose permission is `RecipeEdit`.
    assert_eq!(
        Operation::RenameRecipe.requirement(),
        Requirement::Permission(Permission::RecipeEdit)
    );
    assert_refused_by_its_command("/recipes/{ref}/rename", "name=", "RenameRecipe").await;
}

#[tokio::test]
async fn recipe_access_a_process_engineer_with_a_bad_field_still_gets_the_lines() {
    let routes = [
        (
            "/recipes",
            "machine_code=&reference_number=&name=&change_note=",
        ),
        ("/recipes/{ref}/rename", "name="),
        ("/recipes/{ref}/versions", "based_on=x&change_note="),
    ];
    for (route, bad) in routes {
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
async fn recipe_access_a_forbidden_answer_records_one_forbidden_event() {
    let rows = forbidden_rows();
    assert_eq!(
        rows.len(),
        REFUSED_ROUTES,
        "the write routes of the three areas"
    );
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
