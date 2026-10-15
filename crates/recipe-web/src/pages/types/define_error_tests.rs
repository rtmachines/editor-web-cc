//! Tests of the errors of `POST /machines/{code}/types` and `POST .../revisions` (task 8.3, design
//! D16, `api.md` sections 2.3 and 5).
//!
//! Each test sends an HTMX request and checks the status, the code in the fragment, the retarget
//! header, and that the system wrote nothing.

use axum::body::Body;
use axum::http::{Method, StatusCode};

use super::test_kit::{form_body, post_as, post_htmx, seed_cycle};
use crate::test_support::plant::{HTMX, POLISH, World, world};
use crate::test_support::{Reply, call, post_fields, request};

const NEW: &str = "/machines/M7/types";
const REVISIONS: &str = "/machines/M7/types/ST_Zone/revisions";
/// The header that an error answer sets for the swap.
const RESWAP: &str = "innerHTML show:#error-panel:top";

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

/// The line of a field error that links to the input `id`.
fn line(id: &str, label: &str) -> String {
    format!("<a class=\"target-link\" href=\"#{id}\">{label}</a>")
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
        reply.body.contains(&line("type_name", "Type name")),
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
        assert!(
            reply.body.contains(&line("type_name", "Type name")),
            "{name:?}"
        );
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
        reply.body.contains(&line("type_name", "Type name")),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_define_error_e_web_008_the_name_is_checked_before_the_rows() {
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
    assert!(reply.body.contains("#type_name"), "{}", reply.body);
    assert!(!reply.body.contains("#members-0-name"), "{}", reply.body);
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
        reply
            .body
            .contains(&line("members-0-name", "Name of member 1")),
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
        reply
            .body
            .contains(&line("members-1-name", "Name of member 2")),
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
        reply
            .body
            .contains(&line("members-0-name", "Nazwa składnika 1")),
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
        reply
            .body
            .contains(&line("members-1-type", "Type of member 2")),
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
        reply
            .body
            .contains(&line("members-0-name", "Name of member 1")),
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
        reply
            .body
            .contains(&line("members-1-name", "Name of member 2")),
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
        reply.body.contains(&line("members-0-name", "Members")),
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
