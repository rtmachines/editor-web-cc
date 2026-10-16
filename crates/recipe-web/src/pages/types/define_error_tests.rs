//! Tests of the errors of `POST /machines/{code}/types` and `POST .../revisions` (task 8.3, design
//! D16, `api.md` sections 2.3 and 5).
//!
//! Each test sends an HTMX request and checks the status, the code in the fragment, the retarget
//! header, and that the system wrote nothing.

use axum::body::Body;
use axum::http::{Method, StatusCode};

use super::test_kit::{form_body, post_as, post_htmx, seed_cycle};
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{HTMX, POLISH, World, world};
use crate::test_support::{Reply, call, post_fields, request};

const NEW: &str = "/machines/M7/types";
const REVISIONS: &str = "/machines/M7/types/ST_Zone/revisions";
/// The header that an error answer sets for the swap.
const RESWAP: &str = "innerHTML show:#error-panel:top";
/// The opening tag of the form of a new struct when an error answer swaps it (ADR-0041 (e)).
const OOB_NEW_TAG: &str = "<form id=\"udt-form\" method=\"post\" action=\"/machines/M7/types\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";

/// Send an HTMX request as the ProcessEngineer and assert the error answer.
async fn refused(world: &World, path: &str, body: &str, status: StatusCode, code: &str) -> Reply {
    let before = world.fixture.snapshot();
    let reply = post_htmx(world, &world.engineer, path, body).await;
    assert_eq!(reply.status, status, "{code}: {}", reply.body);
    assert!(reply.body.contains(code), "{code}: {}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel", "{code}");
    assert_eq!(reply.header("hx-reswap"), RESWAP, "{code}");
    assert_eq!(
        world.fixture.snapshot(),
        before,
        "{code} must write nothing"
    );
    reply
}

/// Return true when the body has a line that links to the input `id` and names `label`.
///
/// A line of a list of errors (ADR-0041 (d)) links with its code and its sentence. The sentence of
/// `E-WEB-008` holds the label of the field.
fn has_line(body: &str, id: &str, label: &str) -> bool {
    let link = format!("<a class=\"target-link\" href=\"#{id}\">");
    body.split(&link)
        .skip(1)
        .any(|rest| rest.split("</a>").next().unwrap().contains(label))
}

// --- The codes of the domain (api.md section 2.3) ------------------------------------------------

#[tokio::test]
async fn types_define_error_e_plt_005_a_cycle_that_only_a_seeded_store_holds() {
    let world = world();
    seed_cycle(&world);
    let body = form_body(Some("ST_Gamma"), &[("alpha", "ST_Alpha@1")], &[]);
    refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-005",
    )
    .await;
}

#[tokio::test]
async fn types_define_error_e_plt_006_a_member_type_that_the_platform_lacks() {
    let world = world();
    let body = form_body(
        Some("ST_Press"),
        &[("speed", "INT"), ("label", "WSTRING(10)")],
        &[],
    );
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-006",
    )
    .await;
    assert!(
        reply.body.contains("label"),
        "the member is named: {}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_plt_013_a_type_that_is_not_valid() {
    let world = world();
    for type_text in [
        "ARRAY[1..8] INT",
        "ST_Nope@1",
        "ST_Zone@9",
        "int",
        "STRING(0)",
        "FOO",
    ] {
        let body = form_body(Some("ST_Press"), &[("speed", type_text)], &[]);
        let reply = refused(
            &world,
            NEW,
            &body,
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-013",
        )
        .await;
        assert!(
            reply.body.contains(type_text),
            "{type_text}: {}",
            reply.body
        );
    }
}

#[tokio::test]
async fn types_define_error_e_plt_015_the_name_of_an_existing_struct() {
    let world = world();
    let body = form_body(Some("ST_Zone"), &[("speed", "INT")], &[]);
    refused(&world, NEW, &body, StatusCode::CONFLICT, "E-PLT-015").await;
}

#[tokio::test]
async fn types_define_error_e_plt_016_only_empty_rows_or_no_row() {
    let world = world();
    let only_empty = form_body(Some("ST_Empty"), &[("", ""), ("", "")], &[]);
    refused(
        &world,
        NEW,
        &only_empty,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-016",
    )
    .await;
    let no_row = form_body(Some("ST_Empty"), &[], &[]);
    refused(
        &world,
        NEW,
        &no_row,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-016",
    )
    .await;
    refused(
        &world,
        REVISIONS,
        &no_row,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-016",
    )
    .await;
}

#[tokio::test]
async fn types_define_error_e_plt_018_a_type_above_a_size_limit() {
    let world = world();
    let body = form_body(Some("ST_Big"), &[("cells", "ARRAY[1..20000] OF INT")], &[]);
    refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-018",
    )
    .await;
}

#[tokio::test]
async fn types_define_error_e_plt_021_two_members_with_one_name() {
    let world = world();
    let body = form_body(
        Some("ST_Press"),
        &[("speed", "INT"), ("speed", "REAL")],
        &[],
    );
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-021",
    )
    .await;
    assert!(
        reply.body.contains("speed"),
        "the member is named: {}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_web_005_an_unknown_machine_struct_or_path_value() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("speed", "INT")], &[]);
    let revision = form_body(None, &[("speed", "INT")], &[]);
    let cases = [
        ("/machines/M99/types", &body),
        ("/machines/M12/types", &body),
        ("/machines/X7/types", &body),
        ("/machines/M99/types/ST_Zone/revisions", &revision),
        ("/machines/M7/types/ST_Nope/revisions", &revision),
        ("/machines/M7/types/9bad/revisions", &revision),
        ("/machines/M7/types/new/revisions", &revision),
    ];
    for (path, request_body) in cases {
        refused(
            &world,
            path,
            request_body,
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        )
        .await;
    }
}

// --- E-WEB-008: the fields of the form ------------------------------------------------------------

#[tokio::test]
async fn types_define_error_e_web_008_the_name_new_is_refused() {
    let world = world();
    let body = form_body(Some("new"), &[("speed", "INT")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(
        has_line(&reply.body, "type_name", "Type name"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_web_008_a_name_that_is_not_a_type_name() {
    let world = world();
    let long = "A".repeat(65);
    for name in [
        "9speed",
        "INT",
        "STRING",
        "",
        "a b",
        "a-b",
        "Zażółć",
        long.as_str(),
    ] {
        let body = form_body(Some(name), &[("speed", "INT")], &[]);
        let reply = refused(
            &world,
            NEW,
            &body,
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-WEB-008",
        )
        .await;
        assert!(has_line(&reply.body, "type_name", "Type name"), "{name:?}");
    }
}

#[tokio::test]
async fn types_define_error_e_web_008_a_missing_name_is_a_form_error_on_the_name_field() {
    let world = world();
    let body = form_body(None, &[("speed", "INT")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(
        has_line(&reply.body, "type_name", "Type name"),
        "{}",
        reply.body
    );
}

/// ADR-0041 (a): the answer lists the bad name and the bad row at once, the name first.
#[tokio::test]
async fn types_define_error_e_web_008_lists_the_name_and_each_bad_row() {
    let world = world();
    let body = form_body(Some("new"), &[("9speed", "INT")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    let name = reply.body.find("href=\"#type_name\"").unwrap();
    let row = reply.body.find("href=\"#members-0-name\"").unwrap();
    assert!(name < row, "{}", reply.body);
}

/// Spec "Two bad member rows of a UDT" (U25): each bad member name gives its own line.
#[tokio::test]
async fn types_define_error_e_web_008_two_bad_member_rows_give_two_lines() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("9a", "INT"), ("9b", "INT")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    for (id, label) in [
        ("members-0-name", "Name of member 1"),
        ("members-1-name", "Name of member 2"),
    ] {
        assert!(has_line(&reply.body, id, label), "{id}: {}", reply.body);
    }
    assert_eq!(reply.header("hx-push-url"), "false");
}

/// Spec "Two bad member rows of a UDT" (U33): each name input of the out-of-band form has a line.
#[tokio::test]
async fn types_define_error_swaps_the_form_with_the_sent_rows_and_a_line_under_each_input() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("9a", "INT"), ("9b", "<b>")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    let form = reply.body.split(OOB_NEW_TAG).nth(1).unwrap();
    for value in ["ST_Press", "9a", "9b", "INT", "&#60;b&#62;"] {
        assert!(
            form.contains(&format!("value=\"{value}\"")),
            "{value}: {form}"
        );
    }
    for id in ["members-0-name", "members-1-name", "members-1-type"] {
        let line = format!("<div class=\"field-error\" id=\"{id}-error\">");
        assert!(form.contains(&line), "{id}: {form}");
    }
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 3, "{form}");
    assert!(form.contains("id=\"udt-define-submit\""), "{form}");
    assert!(!reply.body.contains("autofocus"), "{}", reply.body);
    assert_clean_html(&reply.body);
}

/// The form of a new revision keeps its action and has no name input in the error answer.
#[tokio::test]
async fn types_define_error_of_a_revision_swaps_the_revision_form_with_a_line() {
    let world = world();
    let body = form_body(None, &[("speed", "INT"), ("9b", "INT")], &[]);
    let reply = refused(
        &world,
        REVISIONS,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    let tag =
        "<form id=\"udt-form\" method=\"post\" action=\"/machines/M7/types/ST_Zone/revisions\"";
    let form = reply.body.split(tag).nth(1).unwrap();
    assert!(form.contains(" hx-swap-oob=\"true\">"), "{form}");
    assert!(!form.contains("id=\"type_name\""), "{form}");
    assert!(form.contains("id=\"members-1-name-error\""), "{form}");
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 1, "{form}");
}

/// A bad name of a new struct gets its line under the name input.
#[tokio::test]
async fn types_define_error_puts_the_line_of_the_name_under_the_name_input() {
    let world = world();
    let body = form_body(Some("new"), &[("speed", "INT")], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    let form = reply.body.split(OOB_NEW_TAG).nth(1).unwrap();
    let input = form.split("<input id=\"type_name\"").nth(1).unwrap();
    let input = input.split('>').next().unwrap();
    assert!(input.contains("aria-invalid=\"true\""), "{input}");
    assert!(
        input.contains("aria-describedby=\"type_name-hint type_name-error\""),
        "{input}"
    );
    assert!(form.contains("id=\"type_name-error\""), "{form}");
}

/// A bad member name and a bad type in another row give both lines with their own codes.
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

#[tokio::test]
async fn types_define_error_e_web_008_more_than_1024_rows() {
    let world = world();
    let body = "name=ST_Press&members[1024].name=a&members[1024].type=INT";
    let reply = refused(
        &world,
        NEW,
        body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(
        has_line(&reply.body, "members-0-name", "Members"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_web_008_a_key_that_the_form_does_not_have() {
    let world = world();
    for body in [
        "name=ST_Press&members[0].name=a&members[0].type=INT&foo=1",
        "name=ST_Press&members[0].name=a&members[0].type=INT&members[0].name=b",
        "name=ST_Press&members[00].name=a&members[0].type=INT",
        "name=ST_Press&name=ST_Other&members[0].name=a&members[0].type=INT",
        "name=ST_Press&members[0].note=a",
    ] {
        refused(
            &world,
            NEW,
            body,
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-WEB-008",
        )
        .await;
    }
}

#[tokio::test]
async fn types_define_error_e_web_008_a_body_that_is_not_a_form() {
    let world = world();
    let before = world.fixture.snapshot();
    let json = request(Method::POST, NEW)
        .header("content-type", "application/json")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", world.engineer.cookie.as_str())
        .header("x-csrf-token", world.engineer.csrf.as_str())
        .header("hx-request", "true")
        .body(Body::from("{\"name\":\"ST_Press\"}"))
        .unwrap();
    let reply = call(&world.fixture.router(), json).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-008"), "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn types_define_a_bad_request_without_htmx_gets_the_full_error_page() {
    let world = world();
    let body = form_body(Some("new"), &[("speed", "INT")], &[]);
    let reply = post_as(&world, &world.engineer, NEW, &body, &[]).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("<!DOCTYPE html>"));
    assert!(reply.body.contains("E-WEB-008"));
    assert_eq!(reply.header("hx-retarget"), "");
}

#[tokio::test]
async fn types_define_error_answers_never_show_the_typed_name_unescaped() {
    let world = world();
    let payload = "<img src=x onerror=alert(1)>";
    let body = form_body(Some(payload), &[(payload, payload)], &[]);
    let reply = refused(
        &world,
        NEW,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
    )
    .await;
    assert!(!reply.body.contains("<img"), "{}", reply.body);
    let type_body = form_body(Some("ST_Press"), &[("speed", payload)], &[]);
    let reply = refused(
        &world,
        NEW,
        &type_body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-013",
    )
    .await;
    assert!(!reply.body.contains("<img"), "{}", reply.body);
}

/// K12, ADR-0041 (e): a refused `DefineUdt` swaps the form with only its line, under its input.
#[tokio::test]
async fn types_define_puts_the_error_of_the_use_case_under_its_input() {
    let world = world();
    let cases = [
        (
            Some("ST_Zone"),
            [("speed", "INT"), ("", "")],
            "type_name",
            "E-PLT-015",
        ),
        (
            Some("ST_Press"),
            [("speed", "INT"), ("speed", "REAL")],
            "members-1-name",
            "E-PLT-021",
        ),
        (
            Some("ST_Press"),
            [("speed", "INT"), ("label", "WSTRING(10)")],
            "members-1-type",
            "E-PLT-006",
        ),
        (
            Some("ST_Press"),
            [("speed", "INT"), ("zone", "ST_Nope@1")],
            "members-1-type",
            "E-PLT-013",
        ),
    ];
    for (name, rows, id, code) in cases {
        let body = form_body(name, &rows, &[]);
        let before = world.fixture.snapshot();
        let reply = post_htmx(&world, &world.engineer, NEW, &body).await;
        assert!(reply.body.contains("Errors: 1."), "{code}: {}", reply.body);
        assert!(reply.body.contains(&format!("href=\"#{id}\"")), "{code}");
        let form = reply.body.split(OOB_NEW_TAG).nth(1).unwrap();
        assert_eq!(form.matches("aria-invalid=\"true\"").count(), 1, "{form}");
        assert_eq!(form.matches("class=\"field-error\"").count(), 1, "{form}");
        let line = format!("<div class=\"field-error\" id=\"{id}-error\">");
        let (_input, below) = form.split_once(&line).unwrap();
        assert!(below.contains(code), "{code}: {form}");
        assert_eq!(world.fixture.snapshot(), before, "{code}");
    }
}
