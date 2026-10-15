//! Tests of `POST /machines/{code}/types` and `POST .../revisions`: the definition and the answer
//! with `add=1` (task 8.3, spec "UDT definition through the form" and "UDT form with member rows").

use axum::http::StatusCode;

use super::test_kit::{between, form_body, get_as, post_as, post_htmx, row_of, seed_empty_machine};
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::plant::world;

const NEW: &str = "/machines/M7/types";
const REVISIONS: &str = "/machines/M7/types/ST_Zone/revisions";

/// Return the tag of the input with the `id`, from `id=` up to the end of the tag.
fn input<'a>(body: &'a str, id: &str) -> &'a str {
    between(body, &format!("id=\"{id}\""), ">")
}

// --- Definition of a struct ---------------------------------------------------------------------

#[tokio::test]
async fn types_define_creates_the_struct_and_redirects_to_its_page() {
    let world = world();
    let before = world.fixture.store.audit_entries().len();
    let body = form_body(
        Some("ST_Press"),
        &[("speed", "INT"), ("zone", "ST_Zone@2")],
        &[],
    );
    let reply = post_as(&world, &world.engineer, NEW, &body, &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7/types/ST_Press");
    assert!(world.fixture.store.audit_entries().len() > before);
    let list = get_as(&world, &world.operator, NEW, &[]).await;
    let row = row_of(&list.body, "href=\"/machines/M7/types/ST_Press?rev=1\"");
    assert!(
        row.ends_with("<td class=\"num\">1</td><td class=\"num\">2</td><td>latest</td>"),
        "{row}"
    );
}

#[tokio::test]
async fn types_define_answers_an_htmx_request_with_the_same_redirect() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("speed", "INT")], &[]);
    let reply = post_htmx(&world, &world.admin, NEW, &body).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7/types/ST_Press");
}

#[tokio::test]
async fn types_define_skips_a_row_with_an_empty_name_and_an_empty_type() {
    let world = world();
    let code = seed_empty_machine(&world.fixture, 3);
    assert_eq!(code.to_string(), "M3");
    let body = form_body(
        Some("ST_Zone"),
        &[("temp_max", "REAL"), ("", ""), ("", "")],
        &[],
    );
    let reply = post_as(&world, &world.engineer, "/machines/M3/types", &body, &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M3/types/ST_Zone");
    let list = get_as(&world, &world.engineer, "/machines/M3/types", &[]).await;
    let row = row_of(&list.body, "href=\"/machines/M3/types/ST_Zone?rev=1\"");
    assert!(
        row.ends_with("<td class=\"num\">1</td><td class=\"num\">1</td><td>latest</td>"),
        "ST_Zone@1 has one member: {row}"
    );
}

#[tokio::test]
async fn types_define_reads_the_keys_of_a_browser_with_percent_encoded_brackets() {
    let world = world();
    let body = "name=ST_Press&members%5B0%5D.name=speed&members%5B0%5D.type=ARRAY%5B1..8%5D+OF+REAL&csrf_token=ignored";
    let reply = post_as(&world, &world.engineer, NEW, body, &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7/types/ST_Press");
}

#[tokio::test]
async fn types_define_a_revision_redirects_to_the_new_revision() {
    let world = world();
    let body = form_body(
        None,
        &[("speed", "INT"), ("temp_max", "REAL"), ("temp_min", "REAL")],
        &[],
    );
    let reply = post_as(&world, &world.engineer, REVISIONS, &body, &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7/types/ST_Zone?rev=3");
    let list = get_as(&world, &world.operator, NEW, &[]).await;
    let third = row_of(&list.body, "href=\"/machines/M7/types/ST_Zone?rev=3\"");
    assert!(
        third.ends_with("<td class=\"num\">3</td><td class=\"num\">3</td><td>latest</td>"),
        "{third}"
    );
    let second = row_of(&list.body, "href=\"/machines/M7/types/ST_Zone?rev=2\"");
    assert!(
        second.ends_with("<td></td>"),
        "no longer the latest: {second}"
    );
}

#[tokio::test]
async fn types_define_a_revision_takes_the_name_from_the_path_and_ignores_a_name_field() {
    let world = world();
    let body = form_body(Some("ST_Other"), &[("speed", "INT")], &[]);
    let reply = post_as(&world, &world.engineer, REVISIONS, &body, &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7/types/ST_Zone?rev=3");
    let list = get_as(&world, &world.engineer, NEW, &[]).await;
    assert!(!list.body.contains("ST_Other"), "{}", list.body);
}

#[tokio::test]
async fn types_define_is_closed_to_the_operator_and_changes_nothing() {
    let world = world();
    let before = world.fixture.snapshot();
    for path in [NEW, REVISIONS] {
        let body = form_body(Some("ST_Press"), &[("speed", "INT")], &[]);
        let reply = post_as(&world, &world.operator, path, &body, &[]).await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-002"), "{path}");
        let add = form_body(Some("ST_Press"), &[("speed", "INT")], &[("add", "1")]);
        let reply = post_as(&world, &world.operator, path, &add, &[]).await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{path} add: {}",
            reply.body
        );
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn types_define_in_read_only_mode_refuses_the_command_and_the_add_row() {
    let world = world();
    world.fixture.state.read_only().set(true);
    let before = world.fixture.snapshot();
    for path in [NEW, REVISIONS] {
        for extra in [Vec::new(), vec![("add", "1")]] {
            let body = form_body(Some("ST_Press"), &[("speed", "INT")], &extra);
            let reply = post_htmx(&world, &world.engineer, path, &body).await;
            let what = format!("{path} {extra:?}");
            assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE, "{what}");
            assert!(reply.body.contains("E-WEB-006"), "{what}: {}", reply.body);
            assert_eq!(reply.header("hx-retarget"), "#error-panel", "{what}");
        }
    }
    assert_eq!(world.fixture.snapshot(), before);
    let form = get_as(&world, &world.engineer, "/machines/M7/types/new", &[]).await;
    assert_eq!(form.status, StatusCode::OK, "queries stay available");
}

// --- The answer with add=1 ----------------------------------------------------------------------

#[tokio::test]
async fn types_add_member_shows_the_sent_rows_and_one_empty_row_and_creates_no_type() {
    let world = world();
    seed_empty_machine(&world.fixture, 3);
    let before = world.fixture.snapshot();
    let body = form_body(Some("ST_Zone"), &[("temp_max", "REAL")], &[("add", "1")]);
    let reply = post_as(&world, &world.engineer, "/machines/M3/types", &body, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-push-url"), "false");
    let main = reply.body.split_once("<main id=\"content\">").unwrap().1;
    assert!(
        input(main, "type_name").contains("value=\"ST_Zone\""),
        "{main}"
    );
    assert!(
        input(main, "members-0-name").contains("value=\"temp_max\""),
        "{main}"
    );
    assert!(
        input(main, "members-0-type").contains("value=\"REAL\""),
        "{main}"
    );
    let empty = input(main, "members-1-name");
    assert!(empty.contains("value=\"\""), "{empty}");
    assert!(empty.contains(" autofocus"), "{empty}");
    assert!(
        input(main, "members-1-type").contains("value=\"\""),
        "{main}"
    );
    assert!(!main.contains("members-2-name"), "{main}");
    assert_eq!(main.matches("autofocus").count(), 1, "{main}");
    assert_eq!(world.fixture.snapshot(), before, "no type is created");
    let list = get_as(&world, &world.engineer, "/machines/M3/types", &[]).await;
    assert!(
        list.body.contains("The machine has no UDT type."),
        "{}",
        list.body
    );
}

#[tokio::test]
async fn types_add_member_keeps_the_form_action_and_the_title() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("speed", "INT")], &[("add", "1")]);
    let reply = post_as(&world, &world.engineer, NEW, &body, &[]).await;
    assert!(
        reply.body.contains("<h1>New UDT type</h1>"),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("action=\"/machines/M7/types\""),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("<datalist id=\"type-choices\">"));
    assert!(reply.body.contains("<option value=\"ST_Zone@2\">"));
}

#[tokio::test]
async fn types_add_member_to_a_revision_keeps_the_revision_route() {
    let world = world();
    let body = form_body(None, &[("speed", "INT")], &[("add", "1")]);
    let reply = post_as(&world, &world.engineer, REVISIONS, &body, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-push-url"), "false");
    let body = &reply.body;
    assert!(body.contains("<h1>New revision of ST_Zone</h1>"), "{body}");
    assert!(
        body.contains("action=\"/machines/M7/types/ST_Zone/revisions\""),
        "{body}"
    );
    assert!(!body.contains("id=\"type_name\""), "{body}");
    assert!(input(body, "members-0-name").contains("value=\"speed\""));
    assert!(input(body, "members-1-name").contains(" autofocus"));
    assert!(body.contains("Create revision</button>"));
}

/// Design D21 (RR-7): the `add=1` answer of a revision offers no revision of its own struct.
#[tokio::test]
async fn types_add_member_to_a_revision_proposes_no_revision_of_its_own_struct() {
    let world = world();
    let body = form_body(None, &[("speed", "INT")], &[("add", "1")]);
    let reply = post_as(&world, &world.engineer, REVISIONS, &body, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = between(&reply.body, "<datalist id=\"type-choices\">", "</datalist>");
    assert!(!list.contains("ST_Zone@"), "{list}");
    assert!(list.contains("<option value=\"ST_Line@1\">"), "{list}");
}

/// Design D21 (RR-7): a revision that holds an earlier revision of itself gives `E-PLT-005`.
#[tokio::test]
async fn types_define_a_revision_with_a_member_of_its_own_struct_gives_422_e_plt_005() {
    let world = world();
    let before = world.fixture.snapshot();
    let body = form_body(None, &[("previous", "ST_Zone@1")], &[]);
    let reply = post_htmx(&world, &world.engineer, REVISIONS, &body).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-PLT-005"), "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(world.fixture.snapshot(), before, "no revision is created");
    let list = get_as(&world, &world.operator, NEW, &[]).await;
    let latest = row_of(&list.body, "href=\"/machines/M7/types/ST_Zone?rev=2\"");
    assert!(latest.ends_with("<td>latest</td>"), "{latest}");
    assert!(!list.body.contains("ST_Zone?rev=3"), "{}", list.body);
}

#[tokio::test]
async fn types_add_member_renumbers_the_rows_from_zero() {
    let world = world();
    let body = "name=ST_Press&members[7].name=b&members[7].type=INT&members[2].name=a&members[2].type=REAL&add=1";
    let reply = post_as(&world, &world.engineer, NEW, body, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(input(&reply.body, "members-0-name").contains("value=\"a\""));
    assert!(input(&reply.body, "members-1-name").contains("value=\"b\""));
    assert!(input(&reply.body, "members-2-name").contains(" autofocus"));
    assert!(!reply.body.contains("members-3-name"));
}

#[tokio::test]
async fn types_add_member_does_not_check_the_rows_it_only_shows_them() {
    let world = world();
    let before = world.fixture.snapshot();
    let body = form_body(
        Some("9bad name"),
        &[("9speed", "NOT A TYPE"), ("only_name", "")],
        &[("add", "1")],
    );
    let reply = post_as(&world, &world.engineer, NEW, &body, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(input(&reply.body, "type_name").contains("value=\"9bad name\""));
    assert!(input(&reply.body, "members-0-type").contains("value=\"NOT A TYPE\""));
    assert!(input(&reply.body, "members-2-name").contains(" autofocus"));
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn types_add_member_escapes_what_it_echoes() {
    let world = world();
    let payload = "\"><script>alert(1)</script>";
    let body = form_body(Some(payload), &[(payload, payload)], &[("add", "1")]);
    let reply = post_htmx(&world, &world.engineer, NEW, &body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("<script"), "{}", reply.body);
    assert_eq!(reply.body.matches("&#34;&#62;&#60;script&#62;").count(), 3);
    assert_clean_html(&reply.body);
    assert_no_user_text_in_hx_attributes(&reply.body, "script");
}

#[tokio::test]
async fn types_add_member_in_an_htmx_request_moves_the_focus_to_the_new_row_only() {
    let world = world();
    let body = form_body(Some("ST_Press"), &[("speed", "INT")], &[("add", "1")]);
    let reply = post_htmx(&world, &world.engineer, NEW, &body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("<!DOCTYPE"), "{}", reply.body);
    assert_eq!(reply.body.matches("autofocus").count(), 1, "{}", reply.body);
    assert!(input(&reply.body, "members-1-name").contains(" autofocus"));
    assert!(
        reply.body.contains("<h1>New UDT type</h1>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_add_member_to_a_full_form_gives_422_with_the_members_line() {
    let world = world();
    let rows: Vec<(String, String)> = (0..1024)
        .map(|index| (format!("m{index}"), "INT".to_owned()))
        .collect();
    let borrowed: Vec<(&str, &str)> = rows
        .iter()
        .map(|(name, type_text)| (name.as_str(), type_text.as_str()))
        .collect();
    let before = world.fixture.snapshot();
    let body = form_body(Some("ST_Big"), &borrowed, &[("add", "1")]);
    let reply = post_htmx(&world, &world.engineer, NEW, &body).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert!(reply.body.contains("E-WEB-008"));
    assert!(
        reply
            .body
            .contains("<a class=\"target-link\" href=\"#members-0-name\">Members</a>"),
        "{}",
        reply.body
    );
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn types_add_member_with_1023_rows_adds_the_last_row_and_shows_the_limit_notice() {
    let world = world();
    let rows: Vec<(String, String)> = (0..1023)
        .map(|index| (format!("m{index}"), "INT".to_owned()))
        .collect();
    let borrowed: Vec<(&str, &str)> = rows
        .iter()
        .map(|(name, type_text)| (name.as_str(), type_text.as_str()))
        .collect();
    let body = form_body(Some("ST_Big"), &borrowed, &[("add", "1")]);
    let reply = post_as(&world, &world.engineer, NEW, &body, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(input(&reply.body, "members-1023-name").contains(" autofocus"));
    assert!(
        reply
            .body
            .contains("The form holds the maximum of 1024 members.")
    );
    assert!(!reply.body.contains("name=\"add\""));
}

#[tokio::test]
async fn types_add_member_with_a_bad_flag_value_gives_422() {
    let world = world();
    for value in ["0", "2", "yes", ""] {
        let body = form_body(Some("ST_Press"), &[("speed", "INT")], &[("add", value)]);
        let reply = post_htmx(&world, &world.engineer, NEW, &body).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{value:?}");
        assert!(reply.body.contains("E-WEB-008"), "{value:?}");
        assert_eq!(reply.header("hx-retarget"), "#error-panel");
    }
}

#[tokio::test]
async fn types_add_member_of_an_unknown_machine_or_struct_gives_404() {
    let world = world();
    let before = world.fixture.snapshot();
    let body = form_body(None, &[("speed", "INT")], &[("add", "1")]);
    for path in [
        "/machines/M99/types",
        "/machines/M7/types/ST_Nope/revisions",
        "/machines/M7/types/9bad/revisions",
        "/machines/X7/types",
    ] {
        let reply = post_htmx(&world, &world.engineer, path, &body).await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-005"), "{path}");
        assert_eq!(reply.header("hx-retarget"), "#error-panel");
    }
    assert_eq!(world.fixture.snapshot(), before);
}
