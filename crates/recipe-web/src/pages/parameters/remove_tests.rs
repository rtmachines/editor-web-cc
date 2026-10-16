//! Tests of `POST .../parameters/{key}/delete` and of `GET .../parameters` (tasks 11.3 and 11.1).
//!
//! Spec "Deletion of a parameter", design D7, ADR-0040 (c) and (f), `api.md` section 2.6.

use axum::http::StatusCode;

use super::test_kit::{ADD, INT_FIELDS, assert_panel_error, changed, parameter_path, post_htmx};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{ENGLISH, HTMX, HTMX_CONTENT, main_of};
use crate::test_support::recipes::SPEED_KEY;

/// The path of the parameter table of the draft 3 of `R-100`.
const TABLE: &str = "/recipes/R-100/versions/3/parameters";

#[tokio::test]
async fn parameters_page_of_a_draft_holds_the_danger_disclosure_of_the_removal() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    let main = main_of(&reply.body);
    let action = format!("action=\"{}\"", parameter_path(SPEED_KEY, "/delete"));
    assert!(main.contains("<details class=\"danger\">"), "{main}");
    assert!(main.contains(&action), "{main}");
    assert!(main.contains("Delete parameter SPEED</button>"), "{main}");
}

#[tokio::test]
async fn parameters_delete_for_the_content_answers_the_version_page() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/delete");
    let reply = world
        .post_with(&world.engineer, &path, "row_version=1", &HTMX_CONTENT)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    assert_eq!(reply.header("hx-push-url"), "/recipes/R-100/versions/3");
    assert!(!reply.body.contains("<html"), "{}", reply.body);
    assert!(
        reply.body.contains("Version 3 of recipe R-100"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains(">SPEED</a>"), "{}", reply.body);
}

#[tokio::test]
async fn parameters_delete_without_the_content_target_answers_the_table() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "1")]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("id=\"parameter-rows\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains(">SPEED</a>"), "{}", reply.body);
    let plain = world
        .post(
            &world.engineer,
            &parameter_path(uuid::Uuid::from_u128(0xdead), "/delete"),
            "row_version=2",
            false,
        )
        .await;
    assert_eq!(plain.status, StatusCode::NOT_FOUND, "{}", plain.body);
}

#[tokio::test]
async fn parameters_delete_errors_keep_the_url() {
    let world = recipe_world();
    let missing = post_htmx(
        &world,
        &parameter_path(uuid::Uuid::from_u128(0xdead), "/delete"),
        &[("row_version", "1")],
    )
    .await;
    assert_panel_error(&missing, StatusCode::NOT_FOUND, "E-RCP-051");
    let malformed = post_htmx(
        &world,
        "/recipes/R-100/versions/3/parameters/x/delete",
        &[("row_version", "1")],
    )
    .await;
    assert_panel_error(&malformed, StatusCode::NOT_FOUND, "E-WEB-005");
}

/// The toast of the removal of `SPEED`, with the icon element of `components/toast.html`.
const REMOVED_TOAST: &str = "<div class=\"toast\"><svg class=\"icon\"";
/// The sentence of the toast of the removal of `SPEED` in English.
const REMOVED_TEXT: &str = "<span>Parameter SPEED is deleted.</span></div>";

/// K10: the content answer of a removal shows the toast in the status region of the new page.
#[tokio::test]
async fn parameters_delete_for_the_content_shows_the_toast_in_the_status_region() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/delete");
    let reply = world
        .post_with(&world.engineer, &path, "row_version=1", &HTMX_CONTENT)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let region = format!("<div id=\"toast\" role=\"status\" aria-live=\"polite\">{REMOVED_TOAST}");
    assert_eq!(
        reply.body.matches("id=\"toast\"").count(),
        1,
        "{}",
        reply.body
    );
    assert!(reply.body.contains(&region), "{}", reply.body);
    assert!(reply.body.contains(REMOVED_TEXT), "{}", reply.body);
    assert_clean_html(&reply.body);
}

/// K10: the table answer of a removal sends the toast and the empty error panel out of band.
#[tokio::test]
async fn parameters_delete_without_the_content_target_sends_the_toast_out_of_band() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "1")]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let toast = format!("<div id=\"toast\" hx-swap-oob=\"innerHTML\">{REMOVED_TOAST}");
    assert!(reply.body.contains(&toast), "{}", reply.body);
    assert!(reply.body.contains(REMOVED_TEXT), "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn parameters_delete_without_htmx_goes_to_the_version_page() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/delete");
    let reply = world
        .post(&world.engineer, &path, "row_version=1", false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-100/versions/3");
}

#[tokio::test]
async fn parameters_table_answers_the_fragment_or_the_version_page() {
    let world = recipe_world();
    let fragment = world.get(&world.operator, TABLE, &[ENGLISH, HTMX]).await;
    assert_eq!(fragment.status, StatusCode::OK, "{}", fragment.body);
    assert!(!fragment.body.contains("<main"), "{}", fragment.body);
    assert!(fragment.body.contains(">SPEED</a>"), "{}", fragment.body);
    assert!(
        fragment.body.contains("id=\"parameter-rows\""),
        "{}",
        fragment.body
    );
    let page = world.get_en(&world.operator, TABLE).await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    assert!(main_of(&page.body).contains("Version 3 of recipe R-100</h1>"));
    assert_clean_html(&page.body);
}

/// Return the text of the type cell of the row of the tag `tag` in a parameter table.
fn type_cell(body: &str, tag: &str) -> String {
    let (_before, row) = body.split_once(&format!(">{tag}</a></td>")).unwrap();
    let (_name, rest) = row.split_once("<td class=\"data\">").unwrap();
    rest.split("</td>").next().unwrap().to_owned()
}

#[tokio::test]
async fn parameters_new_row_shows_the_type_text_of_the_table() {
    let world = recipe_world();
    let cases = [
        ("NEW_S", "STRING(80)", "DB9.DBB0"),
        ("NEW_A", "ARRAY[1..2] OF STRING", "DB9.DBB100"),
    ];
    let mut row_version = 1_i64;
    for (tag, type_text, address) in cases {
        let version = row_version.to_string();
        let pairs = changed(
            &INT_FIELDS,
            &[
                ("tag", Some(tag)),
                ("type", Some(type_text)),
                ("value_type", Some(type_text)),
                ("s7_address", Some(address)),
                ("row_version", Some(version.as_str())),
                ("value", None),
            ],
        );
        let mut pairs = pairs;
        let leaves: Vec<(String, &str)> = if type_text.starts_with("ARRAY") {
            vec![("value[1]".to_owned(), "a"), ("value[2]".to_owned(), "b")]
        } else {
            vec![("value".to_owned(), "a")]
        };
        pairs.extend(leaves.iter().map(|(name, text)| (name.as_str(), *text)));
        let added = post_htmx(&world, ADD, &pairs).await;
        assert_eq!(added.status, StatusCode::OK, "{tag}: {}", added.body);
        row_version = row_version.checked_add(1).unwrap();
        let table = world.get(&world.engineer, TABLE, &[ENGLISH, HTMX]).await;
        assert_eq!(
            type_cell(&added.body, tag),
            type_cell(&table.body, tag),
            "{tag}"
        );
    }
}
