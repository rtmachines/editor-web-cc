//! Tests of the usage page `GET /machines/{code}/types/{name}/usage` (task 8.4, spec "Type pages",
//! scenarios "Usage page with a row" and "Usage page without a row").

use axum::http::StatusCode;

use super::test_kit::{assert_content_answer, get_as, get_in, row_of};
use super::usage_kit::seed_zone_parameters;
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::icon_tag;
use crate::test_support::plant::{HTMX_CONTENT, POLISH, world};

const USAGE: &str = "/machines/M7/types/ST_Zone/usage";
const USAGE_2: &str = "/machines/M7/types/ST_Zone/usage?rev=2";
/// The address of the usage page of the revision 2 with the page number, as it stands in `href`.
const PAGE_LINK: &str = "/machines/M7/types/ST_Zone/usage?rev=2&#38;page=";

// --- Spec scenarios -----------------------------------------------------------------------------

#[tokio::test]
async fn types_usage_shows_the_recipe_the_version_with_its_badge_the_tag_and_the_type() {
    let world = world();
    let reply = get_as(&world, &world.operator, USAGE_2, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let row = row_of(&reply.body, "<td class=\"data\">R-100</td>");
    assert!(
        row.starts_with(
            "<tr><td class=\"data\">R-100</td><td class=\"num\">3</td><td><span class=\"badge badge-draft\">"
        ),
        "{row}"
    );
    assert!(row.contains(&icon_tag("status-draft")), "{row}");
    assert!(
        row.ends_with(
            "<span>Draft</span></span></td><td class=\"data\">LINE</td><td class=\"data\">ST_Line@1</td>"
        ),
        "{row}"
    );
    assert_eq!(
        reply.body.matches("<td class=\"data\">R-100</td>").count(),
        1
    );
}

#[tokio::test]
async fn types_usage_without_a_row_shows_the_empty_state_and_no_pager() {
    let world = world();
    let reply = get_as(
        &world,
        &world.operator,
        "/machines/M7/types/ST_Zone/usage?rev=1",
        &[],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<p class=\"empty\">No parameter uses this revision.</p>"),
        "{}",
        reply.body
    );
    assert!(
        !reply.body.contains("<td class=\"data\">"),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("class=\"pager\""), "{}", reply.body);
}

#[tokio::test]
async fn types_usage_without_a_revision_shows_the_latest_revision() {
    let world = world();
    let reply = get_as(&world, &world.operator, USAGE, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("<h1>Use of ST_Zone@2</h1>"),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("<td class=\"data\">R-100</td>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_usage_has_the_heading_the_caption_the_columns_and_the_link_to_the_type() {
    let world = world();
    let reply = get_as(&world, &world.operator, USAGE_2, &[]).await;
    let body = &reply.body;
    assert!(body.contains("<h1>Use of ST_Zone@2</h1>"), "{body}");
    assert!(body.contains("<title>Use of ST_Zone@2 · "), "{body}");
    assert!(
        body.contains("<caption id=\"usage-caption\">Parameters that use the revision</caption>")
    );
    for column in ["Recipe", "Version", "State", "Tag", "Type"] {
        assert!(
            body.contains(&format!("<th scope=\"col\">{column}</th>")),
            "{column}"
        );
    }
    let link =
        "<a class=\"target-link\" href=\"/machines/M7/types/ST_Zone?rev=2\">Type ST_Zone@2</a>";
    assert!(body.contains(link), "{body}");
}

#[tokio::test]
async fn types_usage_page_is_open_to_every_role() {
    let world = world();
    for session in [&world.admin, &world.engineer, &world.operator] {
        let reply = get_as(&world, session, USAGE_2, &[]).await;
        assert_eq!(reply.status, StatusCode::OK);
    }
}

// --- The pager ----------------------------------------------------------------------------------

#[tokio::test]
async fn types_usage_pager_links_the_next_page_of_a_full_page() {
    let world = world();
    seed_zone_parameters(&world, 100).await;
    let reply = get_as(&world, &world.operator, USAGE_2, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    assert_eq!(
        body.matches("<tr><td class=\"data\">R-100</td>").count(),
        100,
        "100 rows"
    );
    assert!(
        body.contains("<nav class=\"pager\" aria-label=\"Pages\">"),
        "{body}"
    );
    assert!(body.contains("<span class=\"pager-current\" aria-current=\"page\">Page 1</span>"));
    let next = format!("<a class=\"target-link\" href=\"{PAGE_LINK}2\">Next page</a>");
    assert!(body.contains(&next), "{body}");
    assert!(!body.contains("Previous page"), "{body}");
    assert!(
        body.contains("<td class=\"data\">LINE</td>"),
        "the tag LINE comes first"
    );
    assert!(body.contains("<td class=\"data\">P0099</td>"));
    assert!(!body.contains("<td class=\"data\">P0100</td>"));
}

#[tokio::test]
async fn types_usage_pager_links_back_from_the_last_page() {
    let world = world();
    seed_zone_parameters(&world, 100).await;
    let reply = get_as(&world, &world.operator, &format!("{USAGE_2}&page=2"), &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    assert_eq!(body.matches("<tr><td class=\"data\">R-100</td>").count(), 1);
    assert!(body.contains("<td class=\"data\">P0100</td>"), "{body}");
    assert!(body.contains("<span class=\"pager-current\" aria-current=\"page\">Page 2</span>"));
    let previous = format!("<a class=\"target-link\" href=\"{PAGE_LINK}1\">Previous page</a>");
    assert!(body.contains(&previous), "{body}");
    assert!(!body.contains("Next page"), "{body}");
}

#[tokio::test]
async fn types_usage_page_after_the_end_shows_the_after_end_text_and_a_way_back() {
    let world = world();
    let reply = get_as(&world, &world.operator, &format!("{USAGE_2}&page=3"), &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    assert!(body.contains("Page 3 is after the last page."), "{body}");
    assert!(!body.contains("No parameter uses this revision."), "{body}");
    assert!(!body.contains("class=\"empty\""), "{body}");
    let first = format!("<a class=\"target-link\" href=\"{PAGE_LINK}1\">Go to the first page</a>");
    assert!(body.contains(&first), "{body}");
    // RQ-5: the page before the page after the end can also lie after the end, so no previous link.
    assert!(!body.contains("Previous page"), "{body}");
    assert!(!body.contains("class=\"pager\""), "{body}");
    assert!(!body.contains("Next page"), "{body}");
}

#[tokio::test]
async fn types_usage_page_after_the_end_in_polish_uses_the_polish_texts() {
    let world = world();
    let reply = get_in(
        &world,
        &world.operator,
        &format!("{USAGE_2}&page=3"),
        POLISH,
    )
    .await;
    let body = &reply.body;
    assert!(
        body.contains("Strona 3 jest za ostatni\u{105} stron\u{105}."),
        "{body}"
    );
    assert!(body.contains("Przejd\u{17a} do pierwszej strony"), "{body}");
}

#[tokio::test]
async fn types_usage_first_page_without_a_row_shows_no_after_end_text() {
    let world = world();
    let reply = get_as(
        &world,
        &world.operator,
        USAGE_2.replace("rev=2", "rev=1").as_str(),
        &[],
    )
    .await;
    assert!(
        !reply.body.contains("after the last page"),
        "{}",
        reply.body
    );
    assert!(
        !reply.body.contains("Go to the first page"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_usage_pager_keeps_the_revision_that_the_request_left_out() {
    let world = world();
    seed_zone_parameters(&world, 100).await;
    let reply = get_as(&world, &world.operator, USAGE, &[]).await;
    let next = format!("<a class=\"target-link\" href=\"{PAGE_LINK}2\">Next page</a>");
    assert!(reply.body.contains(&next), "{}", reply.body);
}

// --- Errors -------------------------------------------------------------------------------------

#[tokio::test]
async fn types_usage_of_an_unknown_or_malformed_value_is_not_found() {
    let world = world();
    for path in [
        "/machines/M7/types/ST_Nope/usage",
        "/machines/M7/types/9bad/usage",
        "/machines/M7/types/ST_Zone/usage?rev=9",
        "/machines/M7/types/ST_Zone/usage?rev=0",
        "/machines/M7/types/ST_Zone/usage?rev=abc",
        "/machines/M7/types/ST_Zone/usage?rev=%3Cscript%3E",
        "/machines/M99/types/ST_Zone/usage",
        "/machines/X7/types/ST_Zone/usage",
    ] {
        let reply = get_as(&world, &world.operator, path, &[]).await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-005"), "{path}");
        assert!(!reply.body.contains("<script>"), "{path}");
    }
}

#[tokio::test]
async fn types_usage_with_a_bad_page_or_an_unknown_key_gives_422() {
    let world = world();
    for path in [
        "/machines/M7/types/ST_Zone/usage?page=0",
        "/machines/M7/types/ST_Zone/usage?page=-1",
        "/machines/M7/types/ST_Zone/usage?page=abc",
        "/machines/M7/types/ST_Zone/usage?page=",
        "/machines/M7/types/ST_Zone/usage?page=01",
        "/machines/M7/types/ST_Zone/usage?page=%3Cscript%3E",
        "/machines/M7/types/ST_Zone/usage?x=1",
        "/machines/M7/types/ST_Zone/usage?csrf_token=abc",
    ] {
        let reply = get_as(&world, &world.operator, path, &[]).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-008"), "{path}");
        assert!(!reply.body.contains("<script>"), "{path}");
    }
}

#[tokio::test]
async fn types_usage_with_a_bad_page_gives_the_error_fragment_in_an_htmx_request() {
    let world = world();
    let reply = get_as(
        &world,
        &world.operator,
        "/machines/M7/types/ST_Zone/usage?page=0",
        &[("hx-request", "true")],
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
}

// --- Languages, markup, and the content answer ----------------------------------------------------

#[tokio::test]
async fn types_usage_in_polish_uses_the_polish_words() {
    let world = world();
    let reply = get_in(&world, &world.operator, USAGE_2, POLISH).await;
    let body = &reply.body;
    assert!(body.contains("<h1>Użycie ST_Zone@2</h1>"), "{body}");
    assert!(body.contains("<span>Szkic</span>"), "{body}");
    assert!(body.contains("<th scope=\"col\">Receptura</th>"), "{body}");
    assert!(body.contains("Parametry, które używają rewizji"), "{body}");
}

#[tokio::test]
async fn types_usage_holds_no_form_no_user_text_in_an_hx_attribute_and_no_inline_code() {
    let world = world();
    let reply = get_as(&world, &world.engineer, USAGE_2, &[]).await;
    assert_clean_html(&reply.body);
    for needle in ["M7", "ST_Zone", "R-100", "LINE"] {
        assert_no_user_text_in_hx_attributes(&reply.body, needle);
    }
    let main = reply.body.split_once("<main id=\"content\">").unwrap().1;
    assert!(!main.contains("<form"), "{main}");
    assert!(!main.contains("hx-"), "{main}");
}

#[tokio::test]
async fn types_usage_content_answer_holds_the_title_and_the_focused_heading() {
    let world = world();
    let reply = get_as(&world, &world.operator, USAGE_2, &HTMX_CONTENT).await;
    assert_content_answer(&reply, "Use of ST_Zone@2", USAGE_2);
}
