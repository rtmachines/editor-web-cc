//! Tests of the type list `GET /machines/{code}/types` (task 8.1, spec "Type pages").

use axum::http::StatusCode;

use super::test_kit::{assert_content_answer, get_as, get_in, row_of, seed_empty_machine};
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::plant::{HTMX_CONTENT, POLISH, world};

const LIST: &str = "/machines/M7/types";

#[tokio::test]
async fn types_list_shows_one_row_for_each_struct_revision_with_the_latest_word() {
    let world = world();
    let reply = get_as(&world, &world.operator, LIST, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let latest = row_of(&reply.body, "href=\"/machines/M7/types/ST_Zone?rev=2\"");
    assert_eq!(
        latest,
        "<tr><td class=\"data\"><a href=\"/machines/M7/types/ST_Zone?rev=2\">ST_Zone</a></td><td class=\"num\">2</td><td class=\"num\">2</td><td>latest</td>"
    );
    let first = row_of(&reply.body, "href=\"/machines/M7/types/ST_Zone?rev=1\"");
    assert_eq!(
        first,
        "<tr><td class=\"data\"><a href=\"/machines/M7/types/ST_Zone?rev=1\">ST_Zone</a></td><td class=\"num\">1</td><td class=\"num\">1</td><td></td>"
    );
    let line = row_of(&reply.body, "href=\"/machines/M7/types/ST_Line?rev=1\"");
    assert!(
        line.ends_with("<td class=\"num\">1</td><td>latest</td>"),
        "{line}"
    );
    assert_eq!(reply.body.matches("<td>latest</td>").count(), 2);
}

#[tokio::test]
async fn types_list_orders_the_names_by_bytes_and_the_latest_revision_first() {
    let world = world();
    let reply = get_as(&world, &world.operator, LIST, &[]).await;
    let order: Vec<usize> = [
        "types/ST_Line?rev=1",
        "types/ST_Zone?rev=2",
        "types/ST_Zone?rev=1",
    ]
    .iter()
    .map(|href| reply.body.find(href).unwrap())
    .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

#[tokio::test]
async fn types_list_has_the_heading_the_caption_and_the_columns() {
    let world = world();
    let reply = get_as(&world, &world.operator, LIST, &[]).await;
    let body = &reply.body;
    assert!(body.contains("<h1>UDT types of machine M7</h1>"), "{body}");
    assert!(body.contains("<caption id=\"types-caption\">Struct revisions</caption>"));
    for column in ["Name", "Revision", "Members", "State"] {
        assert!(
            body.contains(&format!("<th scope=\"col\">{column}</th>")),
            "{column}"
        );
    }
    assert!(body.contains("<title>UDT types of machine M7 · "), "{body}");
}

#[tokio::test]
async fn types_list_link_to_the_new_type_form_needs_the_udt_permission() {
    let world = world();
    let link = "<a class=\"target-link\" href=\"/machines/M7/types/new\">New UDT type</a>";
    for (session, shown) in [
        (&world.admin, true),
        (&world.engineer, true),
        (&world.operator, false),
    ] {
        let reply = get_as(&world, session, LIST, &[]).await;
        assert_eq!(reply.body.contains(link), shown, "{}", reply.body);
        assert_eq!(reply.body.contains("/types/new"), shown);
    }
}

#[tokio::test]
async fn types_list_links_back_to_the_machine_page() {
    let world = world();
    let reply = get_as(&world, &world.operator, LIST, &[]).await;
    assert!(
        reply
            .body
            .contains("<a class=\"target-link\" href=\"/machines/M7\">Machine M7</a>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_list_of_a_machine_without_a_struct_shows_the_empty_state() {
    let world = world();
    seed_empty_machine(&world.fixture, 3);
    let reply = get_as(&world, &world.engineer, "/machines/M3/types", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<p class=\"empty\">The machine has no UDT type.</p>"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("<td class=\"data\">"));
}

#[tokio::test]
async fn types_list_of_an_unknown_archived_or_malformed_machine_is_not_found() {
    let world = world();
    for path in [
        "/machines/M99/types",
        "/machines/M12/types",
        "/machines/X7/types",
        "/machines/m7/types",
    ] {
        let reply = get_as(&world, &world.admin, path, &[]).await;
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
async fn types_list_in_polish_uses_the_polish_words() {
    let world = world();
    let reply = get_in(&world, &world.engineer, LIST, POLISH).await;
    let body = &reply.body;
    assert!(body.contains("<h1>Typy UDT maszyny M7</h1>"), "{body}");
    assert!(body.contains("<td>najnowsza</td>"), "{body}");
    assert!(body.contains(">Nowy typ UDT</a>"), "{body}");
    assert!(body.contains("<th scope=\"col\">Rewizja</th>"), "{body}");
}

#[tokio::test]
async fn types_list_holds_no_user_text_in_an_hx_attribute_and_no_inline_code() {
    let world = world();
    let reply = get_as(&world, &world.engineer, LIST, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    for needle in ["M7", "ST_Zone", "ST_Line"] {
        assert_no_user_text_in_hx_attributes(&reply.body, needle);
    }
}

#[tokio::test]
async fn types_list_content_answer_holds_the_title_and_the_focused_heading() {
    let world = world();
    let reply = get_as(&world, &world.engineer, LIST, &HTMX_CONTENT).await;
    assert_content_answer(&reply, "UDT types of machine M7", LIST);
}
