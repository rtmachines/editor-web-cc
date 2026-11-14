//! Tests of the member rows in the error answers of `POST /machines/{code}/types` and
//! `POST .../revisions` (task 8.3, design D16, `api.md` sections 2.3 and 5).
//!
//! Each test sends an HTMX request with bad member rows and checks the lines of the answer. The
//! helpers come from `define_error_tests`.

use axum::http::StatusCode;

use super::define_error_tests::{NEW, OOB_NEW_TAG, has_line, refused};
use super::test_kit::form_body;
use crate::pages::test_helpers::summary_lines;
use crate::test_support::plant::{HTMX, POLISH, world};
use crate::test_support::post_fields;

#[tokio::test]
async fn types_define_error_lists_a_bad_member_name_and_a_bad_type_with_their_codes() {
    let world = world();
    let rows = [("9a", "INT"), ("ok", "FOO"), ("", "")];
    let body = form_body(Some("ST_Press"), &rows, &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    assert!(reply.body.contains("E-PLT-013"), "{}", reply.body);
    assert!(
        has_line(&reply.body, "members-1-type", "FOO"),
        "{}",
        reply.body
    );
    assert!(
        has_line(&reply.body, "members-0-name", "Name of member 1"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_web_008_a_malformed_member_name_names_its_row() {
    let world = world();
    let body = form_body(Some("ST_Zone"), &[("9speed", "INT"), ("ok", "INT")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(
        has_line(&reply.body, "members-0-name", "Name of member 1"),
        "{}",
        reply.body
    );
    let second = form_body(Some("ST_Press"), &[("ok", "INT"), ("bad name", "INT")], &[]);
    let reply = refused(
        &world,
        NEW,
        &second,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(
        has_line(&reply.body, "members-1-name", "Name of member 2"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_web_008_the_label_of_a_row_follows_the_language() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("9speed", "INT")], &[]);
    let headers = [HTMX, POLISH];
    let router = world.fixture.router();
    let reply = post_fields(&router, NEW, Some(&world.engineer), &headers, &body).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(
        has_line(&reply.body, "members-0-name", "Nazwa składnika 1"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_web_008_a_row_with_only_one_field_names_the_empty_one() {
    let world = world();
    let only_name = form_body(Some("ST_Press"), &[("speed", "INT"), ("zone", "")], &[]);
    let reply = refused(
        &world,
        NEW,
        &only_name,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(
        has_line(&reply.body, "members-1-type", "Type of member 2"),
        "{}",
        reply.body
    );
    let only_type = form_body(Some("ST_Press"), &[("", "INT")], &[]);
    let reply = refused(
        &world,
        NEW,
        &only_type,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(
        has_line(&reply.body, "members-0-name", "Name of member 1"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("Clear both fields"), "{}", reply.body);
}

/// Spec "Member row with a bad name and no type" (D4): both inputs of the row give a line.
#[tokio::test]
async fn a_member_row_with_a_bad_name_and_no_type_gives_two_lines() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("9bad", "")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert_eq!(summary_lines(&reply.body), 2, "{}", reply.body);
    assert!(
        has_line(&reply.body, "members-0-name", "Name of member 1"),
        "{}",
        reply.body
    );
    assert!(
        has_line(&reply.body, "members-0-type", "Type of member 1"),
        "{}",
        reply.body
    );
    let form = reply.body.split(OOB_NEW_TAG).nth(1).unwrap();
    for id in ["members-0-name", "members-0-type"] {
        let line = format!("<div class=\"field-error\" id=\"{id}-error\">");
        assert!(form.contains(&line), "{id}: {form}");
    }
}

/// D4: a row with no name and a bad type gives two lines.
#[tokio::test]
async fn a_member_row_with_no_name_and_a_bad_type_gives_two_lines() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("", "FOO")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert_eq!(summary_lines(&reply.body), 2, "{}", reply.body);
    assert!(
        has_line(&reply.body, "members-0-name", "Name of member 1"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("Clear both fields"), "{}", reply.body);
    assert!(reply.body.contains("E-PLT-013"), "{}", reply.body);
    assert!(
        has_line(&reply.body, "members-0-type", "FOO"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_web_008_a_gap_in_the_indices_keeps_the_numbers_of_the_rows() {
    let world = world();
    let body = "name=ST_Press&members[4].name=speed&members[4].type=INT&members[9].name=&members[9].type=INT";
    let reply = refused(
        &world,
        NEW,
        body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(
        has_line(&reply.body, "members-1-name", "Name of member 2"),
        "{}",
        reply.body
    );
}
