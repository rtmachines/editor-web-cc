//! Tests of the field names of design D5, `WebError::BadRow`, and the error line of a member row
//! (task 6.4, spec "Field error lines").
//!
//! The answers come from a router with three test routes, so the tests read the real markup.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::Path;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, Request, StatusCode};
use axum::routing::{MethodRouter, get};
use recipe_domain::error_codes::ErrorCode;
use tower::ServiceExt as _;

use super::view::{field_key, has_input, input_id};
use super::{FieldReason, RowField, WebError};
use crate::WebState;
use crate::i18n::{Bundles, Lang};
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{PEER, state, text};

/// The field names that design D5 adds to the fields that have an input with the same `id`.
const D5_FIELDS: [&str; 10] = [
    "code",
    "machine_name",
    "plc_platform",
    "main_unit_name",
    "parent_path",
    "segment",
    "unit_name",
    "type_name",
    "machine_q",
    "show_archived",
];

/// The fields of the pages of step P3.1. The users page keeps `q`.
const OLD_FIELDS: [&str; 5] = ["login", "password", "current_password", "role", "q"];

// --- The fields -----------------------------------------------------------------------------------

#[test]
fn row_field_input_fields_hold_the_names_of_design_d5_and_keep_the_old_names() {
    for name in D5_FIELDS.into_iter().chain(OLD_FIELDS) {
        assert!(has_input(name), "{name}");
        assert_eq!(input_id(name), name, "{name}");
    }
}

#[test]
fn row_field_a_name_without_an_input_has_no_link() {
    for name in [
        "",
        "name",
        "page",
        "add",
        "archived",
        "form",
        "lang",
        "members-x",
        "Code",
    ] {
        assert!(!has_input(name), "{name:?}");
    }
}

#[test]
fn row_field_the_list_of_member_rows_links_to_its_first_input() {
    assert!(has_input("members"));
    assert_eq!(input_id("members"), "members-0-name");
}

#[test]
fn row_field_every_name_of_design_d5_has_a_label_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    for name in D5_FIELDS.into_iter().chain(["members", "archived", "add"]) {
        for lang in [Lang::Pl, Lang::En] {
            let key = field_key(name);
            assert!(
                bundles.message(lang, &key, None).is_some(),
                "{key} in {lang}"
            );
        }
    }
}

// --- The row field --------------------------------------------------------------------------------

#[test]
fn row_field_names_its_label_key_and_the_suffix_of_the_input_id() {
    assert_eq!(RowField::Name.label_key(), "udt-member-name");
    assert_eq!(RowField::Type.label_key(), "udt-member-type");
    assert_eq!(RowField::Name.id_suffix(), "name");
    assert_eq!(RowField::Type.id_suffix(), "type");
}

#[test]
fn row_field_a_bad_row_has_the_code_e_web_008_and_a_message_without_user_text() {
    let error = WebError::BadRow {
        row: 3,
        field: RowField::Type,
        reason: FieldReason::MemberRows,
    };
    assert_eq!(error.code(), ErrorCode::WEB_008);
    let message = error.to_string();
    assert!(message.contains("member row 3"), "{message}");
    assert!(message.contains("type"), "{message}");
    assert!(message.contains("Clear both fields"), "{message}");
}

// --- The answers ----------------------------------------------------------------------------------

async fn field_error(Path(name): Path<String>) -> Result<&'static str, WebError> {
    let field = D5_FIELDS
        .iter()
        .chain(OLD_FIELDS.iter())
        .find(|known| **known == name)
        .copied()
        .unwrap_or("form");
    Err(WebError::BadRequest {
        field,
        reason: FieldReason::Form,
    })
}

async fn row_error(Path((row, field)): Path<(u16, String)>) -> Result<&'static str, WebError> {
    let (field, reason) = if field == "type" {
        (RowField::Type, FieldReason::MemberRows)
    } else {
        (RowField::Name, FieldReason::MemberName)
    };
    Err(WebError::BadRow { row, field, reason })
}

async fn members_error() -> Result<&'static str, WebError> {
    Err(WebError::BadRequest {
        field: "members",
        reason: FieldReason::MemberCount,
    })
}

/// Build the router with the three test routes. None of them needs a session.
fn router() -> Router {
    let route = |path: &'static str, handler: MethodRouter<WebState>| RouteEntry {
        method: Method::GET,
        path,
        access: Access::Static,
        handler,
    };
    let rows = vec![
        route("/test/field/{name}", get(field_error)),
        route("/test/row/{row}/{field}", get(row_error)),
        route("/test/members", get(members_error)),
    ];
    assemble(state(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

/// The status, the headers that matter, and the body of one answer.
struct Answer {
    status: StatusCode,
    retarget: String,
    body: String,
}

/// Send `GET path` in `lang`, as an HTMX request or as a request for a full page.
async fn ask(path: &str, lang: Lang, htmx: bool) -> Answer {
    let mut builder = Request::builder()
        .uri(path)
        .header("cookie", format!("recipe_lang={lang}"));
    if htmx {
        builder = builder.header("hx-request", "true");
    }
    let response = router()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, 1024 * 1024).await.unwrap();
    Answer {
        status: parts.status,
        retarget: parts
            .headers
            .get("hx-retarget")
            .map(|value| value.to_str().unwrap().to_owned())
            .unwrap_or_default(),
        body: String::from_utf8(bytes.to_vec()).unwrap(),
    }
}

/// The list of field lines with one linked label.
fn linked(id: &str, label: &str) -> String {
    format!(
        "<ul class=\"field-lines\"><li><a class=\"target-link\" href=\"#{id}\">{label}</a></li></ul>"
    )
}

#[tokio::test]
async fn row_field_a_bad_name_of_a_row_links_to_the_input_of_that_row() {
    let answer = ask("/test/row/0/name", Lang::En, true).await;
    assert_eq!(
        answer.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        answer.body
    );
    assert_eq!(answer.retarget, "#error-panel");
    assert!(answer.body.contains("E-WEB-008"));
    let line = linked("members-0-name", "Name of member 1");
    assert!(answer.body.contains(&line), "{}", answer.body);
}

#[tokio::test]
async fn row_field_the_number_of_the_label_is_the_index_plus_one_and_the_id_keeps_the_index() {
    let answer = ask("/test/row/2/type", Lang::En, true).await;
    let line = linked("members-2-type", "Type of member 3");
    assert!(answer.body.contains(&line), "{}", answer.body);
    let high = ask("/test/row/1023/name", Lang::En, true).await;
    let line = linked("members-1023-name", "Name of member 1024");
    assert!(high.body.contains(&line), "{}", high.body);
}

#[tokio::test]
async fn row_field_the_label_and_the_reason_follow_the_language() {
    let answer = ask("/test/row/2/type", Lang::Pl, true).await;
    let line = linked("members-2-type", "Typ sk\u{142}adnika 3");
    assert!(answer.body.contains(&line), "{}", answer.body);
    let reason = text(Lang::Pl, "reason-member-rows");
    assert!(answer.body.contains(&reason), "{}", answer.body);
    assert!(
        answer
            .body
            .contains("Pole Typ sk\u{142}adnika 3 jest niepoprawne")
    );
}

#[tokio::test]
async fn row_field_the_sentence_names_the_row_and_the_reason() {
    let answer = ask("/test/row/0/name", Lang::En, true).await;
    let reason = text(Lang::En, "reason-member-name");
    let sentence = format!("Field Name of member 1 is not valid: {reason}.");
    assert!(answer.body.contains(&sentence), "{}", answer.body);
}

#[tokio::test]
async fn row_field_the_full_error_page_shows_the_label_of_the_row_without_a_link() {
    let answer = ask("/test/row/0/name", Lang::En, false).await;
    assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        answer
            .body
            .contains("<ul class=\"field-lines\"><li>Name of member 1</li></ul>"),
        "{}",
        answer.body
    );
    assert!(!answer.body.contains("href=\"#members-0-name\""));
}

#[tokio::test]
async fn row_field_the_error_of_the_whole_list_links_to_the_first_input() {
    let answer = ask("/test/members", Lang::En, true).await;
    assert_eq!(
        answer.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        answer.body
    );
    let line = linked("members-0-name", "Members");
    assert!(answer.body.contains(&line), "{}", answer.body);
    let full = ask("/test/members", Lang::En, false).await;
    assert!(full.body.contains("<li>Members</li>"), "{}", full.body);
    assert!(!full.body.contains("href=\"#members-0-name\""));
}

#[tokio::test]
async fn row_field_each_name_of_design_d5_links_to_its_input_with_its_label() {
    for name in D5_FIELDS {
        for lang in [Lang::En, Lang::Pl] {
            let answer = ask(&format!("/test/field/{name}"), lang, true).await;
            let line = linked(name, &text(lang, &field_key(name)));
            assert!(
                answer.body.contains(&line),
                "{name} {lang}: {}",
                answer.body
            );
        }
    }
}

#[tokio::test]
async fn row_field_a_name_without_an_input_shows_the_label_without_a_link() {
    let answer = ask("/test/field/unknown", Lang::En, true).await;
    assert!(
        !answer.body.contains("<a class=\"target-link\" href=\"#"),
        "{}",
        answer.body
    );
}
