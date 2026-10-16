//! Tests of `GET /recipes/{ref}/versions/{n}`: the header and the parameter table (task 9.4).
//!
//! Specs "Version page", "Pages of a deleted recipe", and "Content answer of a page".
//! ADR-0043 (e), (i), and (j), design D7.

use axum::http::StatusCode;

use crate::i18n::Lang;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error};
use crate::test_support::plant::main_of;
use crate::test_support::{icon_tag, text};

/// The link that loads the form of a new parameter of the draft of `R-100` into `#parameter-form`.
const ADD_LINK: &str = "<a class=\"target-link\" href=\"/recipes/R-100/versions/3/parameters/new\" hx-boost=\"true\" hx-target=\"#parameter-form\" hx-swap=\"innerHTML\" hx-push-url=\"false\">Add a parameter</a>";
/// The tags of the draft of `R-100` in the order unit path, ordinal, and tag.
const TAGS: [&str; 8] = [
    "LINE",
    "SPEED",
    "LABEL",
    "HOLD",
    "SPEED_TABLE",
    "GRID",
    "LINE_A",
    "ZONES",
];

/// Return the position of the link of the parameter `tag` in `html`.
fn row_of(html: &str, tag: &str) -> usize {
    html.find(&format!("\">{tag}</a></td>")).unwrap()
}

#[tokio::test]
async fn versions_page_of_an_operator_shows_the_header_and_the_table_and_no_form() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.operator, "/recipes/R-100/versions/3")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("<h1>Version 3 of recipe R-100</h1>"),
        "{main}"
    );
    assert!(
        main.contains("<span class=\"badge badge-large badge-draft\">"),
        "{main}"
    );
    assert!(main.contains("<dd class=\"data\">user1</dd>"), "{main}");
    let platform = format!("<dd>{}</dd>", text(Lang::En, "platform-s7-300"));
    assert!(main.contains(&platform), "{main}");
    assert!(
        main.contains("<a class=\"target-link\" href=\"/recipes/R-100\">Recipe R-100</a>"),
        "{main}"
    );
    assert!(!main.contains("<form"), "{main}");
    assert!(!main.contains("/parameters/new"), "{main}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn versions_page_rows_come_in_the_order_unit_path_ordinal_and_tag() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.operator, "/recipes/R-100/versions/3")
        .await;
    let main = main_of(&reply.body);
    let table = main.split("<tbody id=\"parameter-rows\">").nth(1).unwrap();
    let positions: Vec<usize> = TAGS.iter().map(|tag| row_of(table, tag)).collect();
    assert!(positions.is_sorted(), "{positions:?}: {table}");
    assert!(
        table.contains("<td class=\"data\">DB2.DBW0</td>"),
        "{table}"
    );
    assert!(table.contains("<td class=\"data\">U1.3</td>"), "{table}");
    assert!(
        table.contains("<td class=\"data\">ARRAY[1..8] OF INT</td>"),
        "{table}"
    );
    assert!(
        main.contains("<th scope=\"col\">PLC address</th>"),
        "{main}"
    );
}

#[tokio::test]
async fn versions_page_marks_the_outdated_parameter() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.operator, "/recipes/R-100/versions/3")
        .await;
    let main = main_of(&reply.body);
    let (_list, table) = main.split_once("<tbody id=\"parameter-rows\">").unwrap();
    let zones = table.split("\">ZONES</a></td>").nth(1).unwrap();
    let zones = zones.split("</tr>").next().unwrap();
    assert!(zones.contains(&icon_tag("alert-warning")), "{zones}");
    assert!(zones.contains("<span>Outdated</span>"), "{zones}");
    let speed = main.split("\">SPEED</a></td>").nth(1).unwrap();
    let speed = speed.split("</tr>").next().unwrap();
    assert!(!speed.contains("Outdated"), "{speed}");
}

#[tokio::test]
async fn versions_page_of_a_process_engineer_on_the_draft_holds_the_add_link() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-100/versions/3")
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains(ADD_LINK), "{main}");
    assert!(main.contains("<div id=\"parameter-form\"></div>"), "{main}");
    assert!(main.contains("<div id=\"outdated-list\">"), "{main}");
    assert!(main.contains("<div id=\"parameter-table\">"), "{main}");
}

#[tokio::test]
async fn versions_page_of_an_approved_version_has_no_add_link_and_shows_the_empty_state() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-100/versions/2")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(!main.contains("/parameters/new"), "{main}");
    assert!(main.contains("The version has no parameters."), "{main}");
    assert!(
        main.contains("<span class=\"badge badge-large badge-approved\">"),
        "{main}"
    );
}

#[tokio::test]
async fn versions_page_of_a_platform_without_addresses_has_no_address_column() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-200/versions/1")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("PLC address</th>"), "{}", reply.body);
}

#[tokio::test]
async fn versions_page_of_a_deleted_recipe_shows_the_state_only_to_an_administrator() {
    let world = recipe_world();
    let admin = world
        .get_en(&world.admin, "/recipes/R-300/versions/1")
        .await;
    assert_eq!(admin.status, StatusCode::OK, "{}", admin.body);
    let main = main_of(&admin.body);
    assert!(
        main.contains("<span class=\"badge badge-large badge-deleted\">"),
        "{main}"
    );
    assert!(!main.contains("/parameters/new"), "{main}");
    let operator = world
        .get_en(&world.operator, "/recipes/R-300/versions/1")
        .await;
    assert_error(&operator, StatusCode::CONFLICT, "E-RCP-012", "deleted");
}

#[tokio::test]
async fn versions_page_refuses_a_bad_or_unknown_path_value_with_e_web_005() {
    let world = recipe_world();
    let paths = [
        "/recipes/R-100/versions/9",
        "/recipes/R-100/versions/0",
        "/recipes/R-100/versions/03",
        "/recipes/R-100/versions/x",
        "/recipes/-x/versions/1",
    ];
    for path in paths {
        let reply = world.get_en(&world.operator, path).await;
        assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", path);
    }
}

#[tokio::test]
async fn versions_page_refuses_a_unit_that_the_version_does_not_use_or_a_bad_unit() {
    let world = recipe_world();
    let unknown = world
        .get_en(&world.operator, "/recipes/R-100/versions/3?unit=U2")
        .await;
    assert_error(&unknown, StatusCode::NOT_FOUND, "E-WEB-005", "U2");
    let bad = world
        .get_en(&world.operator, "/recipes/R-100/versions/3?unit=X1")
        .await;
    assert_error(&bad, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-017", "X1");
}

#[tokio::test]
async fn versions_page_content_answer_holds_the_title_and_the_focused_heading() {
    let world = recipe_world();
    let reply = world
        .get_content(&world.engineer, "/recipes/R-100/versions/3")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>Version 3 of recipe R-100 · Recipe editor</title>")
    );
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Version 3 of recipe R-100</h1>")
    );
    assert!(!reply.body.contains("class=\"topbar\""), "{}", reply.body);
}
