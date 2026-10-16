//! Tests of the UDT form `GET /machines/{code}/types/new` (task 8.2, spec "UDT form with member
//! rows" and "Plant forms without user data in an hx attribute").

use axum::http::StatusCode;

use super::test_kit::{assert_content_answer, between, get_as, get_in};
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::plant::{HTMX, HTMX_CONTENT, POLISH, world};

const NEW: &str = "/machines/M7/types/new";
const REVISION: &str = "/machines/M7/types/new?from=ST_Zone";

/// The HTMX attributes of every UDT form, after `method` and `action` (design D6 and D8).
const FORM_ATTRIBUTES: &str =
    "hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";

/// Return the markup of `main`, without the top bar.
fn main_of(body: &str) -> &str {
    body.split_once("<main id=\"content\">").unwrap().1
}

#[tokio::test]
async fn types_form_of_a_new_type_has_the_name_one_empty_row_and_the_create_button() {
    let world = world();
    let reply = get_as(&world, &world.engineer, NEW, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>New UDT type</h1>"), "{main}");
    let form = format!(
        "<form id=\"udt-form\" method=\"post\" action=\"/machines/M7/types\" {FORM_ATTRIBUTES}"
    );
    assert!(main.contains(&form), "{main}");
    assert!(
        main.contains("<button type=\"submit\" name=\"add\" value=\"1\" id=\"udt-add-member\""),
        "{main}"
    );
    assert!(
        main.contains("<button type=\"submit\" id=\"udt-define-submit\""),
        "{main}"
    );
    let token = format!(
        "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
        world.engineer.csrf
    );
    assert!(main.contains(&token), "{main}");
    assert!(
        main.contains("<label for=\"type_name\">Type name"),
        "{main}"
    );
    assert!(
        main.contains("id=\"type_name\" name=\"name\" type=\"text\""),
        "{main}"
    );
    assert!(main.contains("maxlength=\"64\""), "{main}");
    assert!(main.contains("A letter or an underscore first, then letters, digits, and underscores. At most 64 characters."));
    assert!(
        main.contains("<fieldset id=\"members\"><legend>Members</legend>"),
        "{main}"
    );
    assert!(main.contains("id=\"members-0-name\""), "{main}");
    assert!(main.contains("id=\"members-0-type\""), "{main}");
    assert!(!main.contains("members-1-name"), "{main}");
    assert!(main.contains("Create type</button>"), "{main}");
    assert!(
        !main.contains("autofocus"),
        "the new form has no autofocus: {main}"
    );
}

#[tokio::test]
async fn types_form_has_the_type_hint_once_and_every_type_input_names_it() {
    let world = world();
    let reply = get_as(&world, &world.engineer, REVISION, &[]).await;
    let main = main_of(&reply.body);
    assert_eq!(main.matches("id=\"udt-type-hint\"").count(), 1, "{main}");
    assert!(main.contains(
        "A type expression, for example INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL."
    ));
    assert_eq!(
        main.matches("aria-describedby=\"udt-type-hint\"").count(),
        2
    );
    assert!(main.contains("The system skips a row with an empty name and an empty type."));
}

#[tokio::test]
async fn types_form_proposes_the_primitives_and_the_latest_revision_of_each_struct() {
    let world = world();
    let reply = get_as(&world, &world.engineer, NEW, &[]).await;
    let list = between(&reply.body, "<datalist id=\"type-choices\">", "</datalist>");
    for choice in ["INT", "REAL", "STRING", "ST_Zone@2", "ST_Line@1"] {
        assert!(
            list.contains(&format!("<option value=\"{choice}\">")),
            "{choice}: {list}"
        );
    }
    assert!(!list.contains("ST_Zone@1"), "{list}");
    assert!(
        !list.contains("WSTRING"),
        "the S7-300 has no WSTRING: {list}"
    );
    assert_eq!(reply.body.matches("<datalist").count(), 1);
    assert_eq!(reply.body.matches("list=\"type-choices\"").count(), 1);
}

/// Design D21 (RR-7): the form of a new revision offers no revision of the struct that it revises.
#[tokio::test]
async fn types_form_of_a_new_revision_proposes_no_revision_of_its_own_struct() {
    let world = world();
    let reply = get_as(&world, &world.engineer, REVISION, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = between(&reply.body, "<datalist id=\"type-choices\">", "</datalist>");
    assert!(!list.contains("ST_Zone@"), "{list}");
    for choice in ["INT", "REAL", "ST_Line@1"] {
        assert!(
            list.contains(&format!("<option value=\"{choice}\">")),
            "{choice}: {list}"
        );
    }
}

#[tokio::test]
async fn types_form_puts_the_add_button_before_the_create_button() {
    let world = world();
    let reply = get_as(&world, &world.engineer, NEW, &[]).await;
    let main = main_of(&reply.body);
    let add = main
        .find("<button type=\"submit\" name=\"add\" value=\"1\"")
        .unwrap();
    let create = main.find("Create type</button>").unwrap();
    assert!(add < create, "{main}");
    assert!(main.contains(">Add a member</button>"), "{main}");
    assert_eq!(main.matches("name=\"add\"").count(), 1);
    assert!(!main.contains("The form holds the maximum of 1024 members."));
}

#[tokio::test]
async fn types_form_of_a_new_revision_holds_the_members_of_the_latest_revision() {
    let world = world();
    let reply = get_as(&world, &world.engineer, REVISION, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>New revision of ST_Zone</h1>"), "{main}");
    assert!(
        main.contains("action=\"/machines/M7/types/ST_Zone/revisions\""),
        "{main}"
    );
    assert!(main.contains("Create revision</button>"), "{main}");
    for (id, value) in [
        ("members-0-name", "speed"),
        ("members-0-type", "INT"),
        ("members-1-name", "temp_max"),
        ("members-1-type", "REAL"),
    ] {
        let input = format!("id=\"{id}\"");
        let tag = between(main, &input, ">");
        assert!(tag.contains(&format!("value=\"{value}\"")), "{id}: {tag}");
    }
    assert!(!main.contains("members-2-name"), "no empty row: {main}");
    assert!(
        !main.contains("id=\"type_name\""),
        "the name is in the heading: {main}"
    );
    assert!(!main.contains("name=\"name\""), "{main}");
}

#[tokio::test]
async fn types_form_rows_have_the_labels_and_the_inputs_of_the_row_macro() {
    let world = world();
    let reply = get_as(&world, &world.engineer, REVISION, &[]).await;
    let main = main_of(&reply.body);
    assert!(main.contains("<legend>Member 1</legend>"), "{main}");
    assert!(main.contains("<legend>Member 2</legend>"), "{main}");
    assert!(
        main.contains("<label for=\"members-1-name\">Name of member 2</label>"),
        "{main}"
    );
    assert!(
        main.contains("<label for=\"members-1-type\">Type of member 2</label>"),
        "{main}"
    );
    assert!(main.contains("name=\"members[1].type\""), "{main}");
}

#[tokio::test]
async fn types_form_with_a_csrf_token_in_the_query_gives_422() {
    let world = world();
    for path in [
        "/machines/M7/types/new?csrf_token=tok-zz-qq",
        "/machines/M7/types/new?from=ST_Zone&csrf_token=tok-zz-qq",
        "/machines/M7/types/new?x=1",
        "/machines/M7/types/new?name=ST_A",
    ] {
        let reply = get_as(&world, &world.engineer, path, &[]).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-008"), "{path}: {}", reply.body);
        // The token text has letters outside 0-9a-f, so a request id (a UUID) cannot hold it.
        assert!(!reply.body.contains("tok-zz-qq"), "{path}: {}", reply.body);
    }
    let htmx = get_as(
        &world,
        &world.engineer,
        "/machines/M7/types/new?csrf_token=tok-zz-qq",
        &[HTMX],
    )
    .await;
    assert_eq!(htmx.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(htmx.header("hx-retarget"), "#error-panel");
}

#[tokio::test]
async fn types_form_with_an_unknown_or_malformed_struct_name_gives_404() {
    let world = world();
    for path in [
        "/machines/M7/types/new?from=ST_Nope",
        "/machines/M7/types/new?from=9bad",
        "/machines/M7/types/new?from=",
        "/machines/M7/types/new?from=INT",
        "/machines/M99/types/new",
        "/machines/X7/types/new",
    ] {
        let reply = get_as(&world, &world.engineer, path, &[]).await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-005"), "{path}: {}", reply.body);
    }
}

#[tokio::test]
async fn types_form_is_closed_to_the_operator() {
    let world = world();
    for path in [NEW, REVISION] {
        let reply = get_as(&world, &world.operator, path, &[]).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{path}");
        assert!(reply.body.contains("E-WEB-002"), "{path}: {}", reply.body);
    }
}

#[tokio::test]
async fn types_form_never_sends_get_with_the_token_and_no_url_holds_it() {
    let world = world();
    for path in [NEW, REVISION] {
        let reply = get_as(&world, &world.engineer, path, &[]).await;
        let main = main_of(&reply.body);
        for form in main.split("<form").skip(1) {
            let tag = form.split('>').next().unwrap();
            assert!(tag.contains("method=\"post\""), "{tag}");
            assert!(!tag.contains("method=\"get\""), "{tag}");
        }
        assert!(!reply.body.contains("csrf_token="), "{path}");
        assert!(!reply.body.contains("csrf_token%"), "{path}");
        let token = &world.engineer.csrf;
        for attribute in ["href", "action"] {
            let marker = format!("{attribute}=\"");
            for part in reply.body.split(marker.as_str()).skip(1) {
                let value = part.split('"').next().unwrap();
                assert!(!value.contains(token.as_str()), "{attribute}={value}");
            }
        }
        let next = between(&reply.body, "id=\"lang-next\" name=\"next\" value=\"", "\"");
        assert!(!next.contains("csrf"), "{next}");
    }
}

#[tokio::test]
async fn types_form_hx_attributes_are_literals_without_user_text() {
    let world = world();
    for path in [NEW, REVISION] {
        let reply = get_as(&world, &world.engineer, path, &[]).await;
        assert_clean_html(&reply.body);
        for needle in ["M7", "ST_Zone", "speed", "temp_max"] {
            assert_no_user_text_in_hx_attributes(&reply.body, needle);
        }
        let main = main_of(&reply.body);
        assert_eq!(main.matches("hx-boost=").count(), 1, "{main}");
        assert_eq!(main.matches("hx-target=").count(), 1, "{main}");
        assert_eq!(main.matches("hx-push-url=").count(), 1, "{main}");
        assert_eq!(main.matches("hx-sync=").count(), 1, "{main}");
    }
}

#[tokio::test]
async fn types_form_in_polish_uses_the_polish_words() {
    let world = world();
    let reply = get_in(&world, &world.engineer, NEW, POLISH).await;
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>Nowy typ UDT</h1>"), "{main}");
    assert!(main.contains("<legend>Składniki</legend>"), "{main}");
    assert!(main.contains(">Dodaj składnik</button>"), "{main}");
    assert!(main.contains("Utwórz typ</button>"), "{main}");
    assert!(main.contains("Nazwa składnika 1"), "{main}");
}

#[tokio::test]
async fn types_form_never_echoes_a_payload_of_the_query() {
    let world = world();
    for path in [
        "/machines/M7/types/new?from=%3Cscript%3Ealert(1)%3C/script%3E",
        "/machines/M7/types/new?%3Cscript%3E=1",
        "/machines/%3Cscript%3E/types/new",
    ] {
        let reply = get_as(&world, &world.engineer, path, &[]).await;
        assert_ne!(reply.status, StatusCode::OK, "{path}");
        assert!(!reply.body.contains("<script>"), "{path}: {}", reply.body);
        assert!(!reply.body.contains("alert(1)"), "{path}: {}", reply.body);
    }
    let ok = get_as(&world, &world.engineer, REVISION, &[]).await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.body);
    assert!(!ok.body.contains("<script>"), "{}", ok.body);
}

#[tokio::test]
async fn types_form_content_answer_holds_the_title_and_the_focused_heading() {
    let world = world();
    let reply = get_as(&world, &world.engineer, NEW, &HTMX_CONTENT).await;
    assert_content_answer(&reply, "New UDT type", NEW);
}
