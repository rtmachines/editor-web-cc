//! Tests of the parsers of the path values and the query values (task 6.1, design D5).
//!
//! The value tests call each parser with a table of good and bad texts. The route tests send a
//! request through a small router, because only axum can make the rejection of a path value.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::rejection::PathRejection;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use axum::routing::get;
use recipe_app::error::{AppError, Missing};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::PlantError;
use tower::ServiceExt as _;

use super::{
    archived_machine, flag, machine_code, machine_id, page, path_code, path_code_type,
    path_code_unit, path_machine_id, revision, type_name, unit_path,
};
use crate::error::{FieldReason, WebError};

/// A machine identifier in the form of the route `/machines/by-id/{uuid}`.
const ID: &str = "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10";

fn is_malformed_machine(error: &WebError) -> bool {
    matches!(
        error,
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedMachine
        })
    )
}

fn is_malformed_type(error: &WebError) -> bool {
    matches!(
        error,
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedType
        })
    )
}

fn is_bad_request(error: &WebError, name: &str, expected: FieldReason) -> bool {
    matches!(error, WebError::BadRequest { field, reason } if *field == name && *reason == expected)
}

// --- Values ---------------------------------------------------------------------------------------

#[test]
fn path_values_machine_code_accepts_a_code_and_refuses_anything_else() {
    assert_eq!(machine_code("M7").unwrap().to_string(), "M7");
    assert_eq!(machine_code("M9999999").unwrap().to_string(), "M9999999");
    let bad = [
        "",
        "X7",
        "m7",
        "M",
        "M0",
        "M07",
        "M10000000",
        "M7 ",
        " M7",
        "M7.1",
        "M-1",
        "M\u{661}",
    ];
    for text in bad {
        let error = machine_code(text).unwrap_err();
        assert!(is_malformed_machine(&error), "{text:?}");
        assert_eq!(error.code(), ErrorCode::WEB_005, "{text:?}");
    }
}

#[test]
fn path_values_machine_id_accepts_only_the_hyphenated_form() {
    assert_eq!(machine_id(ID).unwrap().to_string(), ID);
    assert_eq!(machine_id(&ID.to_uppercase()).unwrap().to_string(), ID);
    let bad = [
        "",
        "0b9d8a4e5f1c4c529c1e2f6f3c1a7e10",
        "{0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10}",
        "urn:uuid:0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e1",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e100",
        " 0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10",
        "M7",
    ];
    for text in bad {
        assert!(
            is_malformed_machine(&machine_id(text).unwrap_err()),
            "{text:?}"
        );
    }
}

#[test]
fn path_values_type_name_accepts_a_struct_name_and_refuses_anything_else() {
    for text in ["ST_Zone", "_a", "a1", "new"] {
        assert_eq!(type_name(text).unwrap().to_string(), text);
    }
    let long = "a".repeat(65);
    let bad = [
        "",
        "9a",
        "a-b",
        "a b",
        "INT",
        "Zone\n",
        "\u{141}",
        long.as_str(),
    ];
    for text in bad {
        let error = type_name(text).unwrap_err();
        assert!(is_malformed_type(&error), "{text:?}");
        assert_eq!(error.code(), ErrorCode::WEB_005, "{text:?}");
    }
}

#[test]
fn path_values_revision_is_a_whole_number_from_1_without_a_sign_or_a_leading_zero() {
    assert_eq!(revision("1").unwrap(), 1);
    assert_eq!(revision("12").unwrap(), 12);
    assert_eq!(revision("4294967295").unwrap(), u32::MAX);
    let bad = [
        "",
        "0",
        "01",
        "+1",
        "-1",
        " 1",
        "1 ",
        "1.0",
        "x",
        "4294967296",
        "\u{661}",
    ];
    for text in bad {
        assert!(is_malformed_type(&revision(text).unwrap_err()), "{text:?}");
    }
}

#[test]
fn path_values_unit_path_keeps_the_code_of_the_domain_error() {
    assert_eq!(unit_path("U1.2.4").unwrap().to_string(), "U1.2.4");
    assert_eq!(unit_path("U1.2.4.5.6").unwrap().to_string(), "U1.2.4.5.6");
    let deep = unit_path("U1.2.4.5.6.7").unwrap_err();
    assert_eq!(deep.code(), ErrorCode::PLT_002);
    assert!(matches!(deep, WebError::App(AppError::Plant(_))));
    for text in [
        "", "1", "U", "U0", "U65", "U1..2", "U1.", "u1.2", "U01", "U1.2 ",
    ] {
        let error = unit_path(text).unwrap_err();
        assert_eq!(error.code(), ErrorCode::PLT_017, "{text:?}");
    }
}

#[test]
fn path_values_archived_machine_names_its_field_and_reason() {
    assert_eq!(archived_machine("M12").unwrap().to_string(), "M12");
    for text in ["", "X1", "M0", "m1", "M1 "] {
        let error = archived_machine(text).unwrap_err();
        assert!(
            is_bad_request(&error, "archived", FieldReason::MachineCode),
            "{text:?}"
        );
        assert_eq!(error.code(), ErrorCode::WEB_008);
    }
}

#[test]
fn path_values_page_counts_from_1_in_the_url_and_from_0_in_the_use_case() {
    let first = page(None).unwrap();
    assert_eq!((first.number, first.index), (1, 0));
    let same = page(Some("1")).unwrap();
    assert_eq!((same.number, same.index), (1, 0));
    let third = page(Some("3")).unwrap();
    assert_eq!((third.number, third.index), (3, 2));
    let last = page(Some("4294967295")).unwrap();
    assert_eq!(
        (last.number, last.index),
        (u32::MAX, u32::MAX.checked_sub(1).unwrap())
    );
}

#[test]
fn path_values_page_refuses_zero_a_sign_a_leading_zero_and_text() {
    for text in [
        "",
        "0",
        "01",
        "+1",
        "-1",
        " 1",
        "1 ",
        "1.5",
        "x",
        "4294967296",
        "\u{661}",
    ] {
        let error = page(Some(text)).err().unwrap();
        assert!(
            is_bad_request(&error, "page", FieldReason::Page),
            "{text:?}"
        );
    }
}

#[test]
fn path_values_flag_is_false_without_a_value_and_true_for_1_only() {
    assert!(!flag("show_archived", None).unwrap());
    assert!(flag("show_archived", Some("1")).unwrap());
    assert!(flag("add", Some("1")).unwrap());
    for text in ["", "0", "01", "true", "on", "yes", " 1", "1 ", "11"] {
        for field in ["show_archived", "add"] {
            let error = flag(field, Some(text)).unwrap_err();
            assert!(
                is_bad_request(&error, field, FieldReason::Flag),
                "{field} {text:?}"
            );
        }
    }
}

// --- Routes ---------------------------------------------------------------------------------------

async fn by_code(path: Result<Path<String>, PathRejection>) -> Result<String, WebError> {
    path_code(path).map(|code| code.to_string())
}

async fn by_id(path: Result<Path<String>, PathRejection>) -> Result<String, WebError> {
    path_machine_id(path).map(|id| id.to_string())
}

async fn by_unit(path: Result<Path<(String, String)>, PathRejection>) -> Result<String, WebError> {
    path_code_unit(path).map(|(code, unit)| format!("{code} {unit}"))
}

async fn by_type(path: Result<Path<(String, String)>, PathRejection>) -> Result<String, WebError> {
    path_code_type(path).map(|(code, name)| format!("{code} {name}"))
}

/// Send `GET uri` to a router with one route for each shape of path values.
async fn ask(uri: &str) -> Response {
    Router::new()
        .route("/code/{code}", get(by_code))
        .route("/id/{uuid}", get(by_id))
        .route("/unit/{code}/{path}", get(by_unit))
        .route("/type/{code}/{name}", get(by_type))
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// Return the error that a handler stored in the response, or `None` for a good answer.
fn error_of(response: &Response) -> Option<&WebError> {
    response
        .extensions()
        .get::<Arc<WebError>>()
        .map(|error| &**error)
}

#[tokio::test]
async fn path_values_routes_accept_good_values() {
    let id_uri = format!("/id/{ID}");
    for uri in [
        "/code/M7",
        id_uri.as_str(),
        "/unit/M7/U1.2.4",
        "/type/M7/ST_Zone",
    ] {
        let response = ask(uri).await;
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        assert!(error_of(&response).is_none(), "{uri}");
    }
}

#[tokio::test]
async fn path_values_a_bad_code_or_id_gives_the_malformed_machine() {
    for uri in [
        "/code/X7",
        "/code/%FF",
        "/id/abc",
        "/id/%FF",
        "/unit/X7/U1.2",
        "/unit/%FF/U1.2",
        "/type/X7/ST_Zone",
        "/type/%FF/ST_Zone",
    ] {
        let response = ask(uri).await;
        let error = error_of(&response).expect(uri);
        assert!(is_malformed_machine(error), "{uri}");
    }
}

/// Design D20 (RR-5): a `{name}` that is not UTF-8 names the struct and not the machine.
#[tokio::test]
async fn path_values_a_type_name_that_is_not_utf8_gives_the_malformed_type() {
    for uri in ["/type/M7/%FF", "/type/M7/ST%C3%28"] {
        let response = ask(uri).await;
        let error = error_of(&response).expect(uri);
        assert!(is_malformed_type(error), "{uri}");
        assert_eq!(error.code(), ErrorCode::WEB_005, "{uri}");
    }
}

/// Limit of axum: the rejection of a `{path}` that is not UTF-8 holds no code (RR-4).
///
/// The handler cannot check the code first, so `E-PLT-017` wins over a malformed ASCII code.
#[tokio::test]
async fn path_values_a_unit_path_that_is_not_utf8_hides_a_malformed_ascii_code() {
    let response = ask("/unit/X7/%FF").await;
    assert_eq!(error_of(&response).unwrap().code(), ErrorCode::PLT_017);
}

/// Design D20 (U27), `api.md` section 1: a `{path}` that is not UTF-8 is a malformed unit path.
#[tokio::test]
async fn path_values_a_unit_path_that_is_not_utf8_gives_e_plt_017() {
    for uri in ["/unit/M7/%FF", "/unit/M7/U%FF", "/unit/M7/U1.%C3%28"] {
        let response = ask(uri).await;
        let error = error_of(&response).expect(uri);
        assert_eq!(error.code(), ErrorCode::PLT_017, "{uri}");
        assert!(
            matches!(
                error,
                WebError::App(AppError::Plant(PlantError::InvalidUnitPath { .. }))
            ),
            "{uri}"
        );
    }
}

#[tokio::test]
async fn path_values_a_code_that_is_not_utf8_keeps_the_malformed_machine_before_the_path() {
    let response = ask("/unit/%FF/%FF").await;
    assert!(is_malformed_machine(error_of(&response).unwrap()));
}

#[tokio::test]
async fn path_values_a_bad_unit_path_gives_the_code_of_the_domain_error() {
    let deep = ask("/unit/M7/U1.2.4.5.6.7").await;
    assert_eq!(error_of(&deep).unwrap().code(), ErrorCode::PLT_002);
    let malformed = ask("/unit/M7/x").await;
    assert_eq!(error_of(&malformed).unwrap().code(), ErrorCode::PLT_017);
}

#[tokio::test]
async fn path_values_a_bad_type_name_gives_the_malformed_type() {
    for uri in ["/type/M7/INT", "/type/M7/9a", "/type/M7/a-b"] {
        let response = ask(uri).await;
        let error = error_of(&response).expect(uri);
        assert!(is_malformed_type(error), "{uri}");
    }
}

#[tokio::test]
async fn path_values_the_code_is_checked_before_the_second_value() {
    let response = ask("/unit/X7/x").await;
    assert!(is_malformed_machine(error_of(&response).unwrap()));
    let response = ask("/type/X7/INT").await;
    assert!(is_malformed_machine(error_of(&response).unwrap()));
}
