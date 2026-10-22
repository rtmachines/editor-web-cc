//! Tests of `GET /recipes/{ref}/versions/{n}/diff/{m}`: the header, the sections, the summary, the
//! empty state, the fragment, and the languages (task 12.1, ADR-0050 (d), spec "Diff page").

use axum::http::StatusCode;

use super::test_world::{R500_DIFF, section_count};
use crate::pages::audit::test_world::audit_world;
use crate::pages::test_helpers::{
    assert_before, assert_clean_html, part_of, section, seed_unedited_version_3,
};
use crate::test_support::icon_tag;
use crate::test_support::plant::main_of;

#[tokio::test]
async fn diff_page_shows_the_title_the_summary_and_the_sections_in_order() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, R500_DIFF).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("<h1>Changes of version 2 of recipe R-500 against version 1</h1>"),
        "{main}"
    );
    let order = ["limits", "values", "added", "removed"].map(|kind| {
        let marker = format!("id=\"diff-{kind}\"");
        assert!(main.contains(&marker), "{marker}: {main}");
        main.find(&marker).unwrap()
    });
    assert!(order.is_sorted(), "{main}");
    assert!(!main.contains("id=\"diff-definitions\""), "{main}");
    assert!(main.contains("Summary of the changes"), "{main}");
    assert!(main.contains("href=\"#diff-limits\""), "{main}");
    assert!(!main.contains("<form"), "{main}");
    assert!(!main.contains("csrf_token"), "{main}");
}

#[tokio::test]
async fn diff_page_limits_section_shows_the_old_limit_before_the_new_limit() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, R500_DIFF).await;
    let limits = section(main_of(&reply.body), "limits");
    assert!(limits.contains(&icon_tag("diff-limit")), "{limits}");
    assert!(limits.contains("Limit changed"), "{limits}");
    assert_eq!(
        main_of(&reply.body)
            .matches("class=\"table-wrap diff-limits\"")
            .count(),
        1
    );
    let row = part_of(limits, ">SPEED_TABLE<", "</tr>");
    assert!(row.contains("[*]"), "{row}");
    assert_before(
        row,
        "<td class=\"diff-value\">0 to 3000</td>",
        "<td class=\"diff-value\">10 to 3000</td>",
    );
    assert_before(
        limits,
        "<th scope=\"col\" class=\"diff-value\">Version 1</th>",
        "<th scope=\"col\" class=\"diff-value\">Version 2</th>",
    );
}

#[tokio::test]
async fn diff_page_values_section_shows_the_leaf_of_a_struct_in_a_struct() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, R500_DIFF).await;
    let values = section(main_of(&reply.body), "values");
    let row = part_of(values, ".head.a", "</tr>");
    assert_before(
        row,
        "<td class=\"diff-value\">1</td>",
        "<td class=\"diff-value\">2</td>",
    );
    assert!(row.contains("0 to 100"), "{row}");
    assert!(row.contains("Value changed"), "{row}");
}

#[tokio::test]
async fn diff_page_links_the_back_to_the_new_version() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, R500_DIFF).await;
    let main = main_of(&reply.body);
    assert!(
        main.contains("href=\"/recipes/R-500/versions/2\">Back to version 2</a>"),
        "{main}"
    );
}

#[tokio::test]
async fn diff_page_of_the_same_version_shows_the_empty_state() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.data.admin, "/recipes/R-500/versions/1/diff/1")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("<p class=\"empty\">Versions 1 and 1 have the same parameters.</p>"),
        "{main}"
    );
    assert_eq!(section_count(main), 0, "{main}");
    assert!(!main.contains("class=\"pager\""), "{main}");
}

#[tokio::test]
async fn diff_page_of_two_versions_without_a_change_shows_the_empty_state() {
    let world = audit_world().await;
    seed_unedited_version_3(&world.fixture, &world.data).await;
    let reply = world
        .get_en(&world.data.admin, "/recipes/R-500/versions/3/diff/2")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    // The text names the two different versions, so a view that compares `new` with `old` fails.
    assert!(
        main.contains("<p class=\"empty\">Versions 3 and 2 have the same parameters.</p>"),
        "{main}"
    );
    assert_eq!(section_count(main), 0, "{main}");
    assert!(!main.contains("class=\"pager\""), "{main}");
    assert!(!main.contains("class=\"link-list\""), "{main}");
}

#[tokio::test]
async fn diff_page_after_the_last_page_of_an_empty_diff_shows_the_notice_and_no_empty_state() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.data.admin, "/recipes/R-500/versions/1/diff/1?page=2")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("Page 2 is after the last page."), "{main}");
    assert!(!main.contains("class=\"empty\""), "{main}");
    assert!(!main.contains("have the same parameters"), "{main}");
    assert_eq!(section_count(main), 0, "{main}");
}

#[tokio::test]
async fn diff_page_with_changes_shows_no_empty_state() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, R500_DIFF).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("class=\"empty\""), "{}", reply.body);
}

#[tokio::test]
async fn diff_page_answers_an_operator() {
    let world = audit_world().await;
    let reply = world.get_en(&world.operator, R500_DIFF).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        main_of(&reply.body).contains("id=\"diff-limits\""),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn diff_page_content_answer_focuses_the_heading() {
    let world = audit_world().await;
    let reply = world.get_content(&world.data.admin, R500_DIFF).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("<html"), "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Changes of version 2"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn diff_page_speaks_polish() {
    let world = audit_world().await;
    let reply = world.get(&world.data.admin, "pl", R500_DIFF).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("<h1>Zmiany wersji 2 receptury R-500 względem wersji 1</h1>"),
        "{main}"
    );
    let empty = world
        .get(&world.data.admin, "pl", "/recipes/R-500/versions/1/diff/1")
        .await;
    assert!(
        main_of(&empty.body).contains("Wersje 1 i 1 mają te same parametry."),
        "{}",
        empty.body
    );
}

#[tokio::test]
async fn diff_page_of_a_deleted_recipe_shows_the_state_and_no_form_to_an_administrator() {
    let world = audit_world().await;
    let path = "/recipes/R-700/versions/1/diff/1";
    let reply = world.get_en(&world.data.admin, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("badge-deleted"), "{main}");
    assert!(main.contains("Deleted"), "{main}");
    assert!(!main.contains("<form"), "{main}");
}
