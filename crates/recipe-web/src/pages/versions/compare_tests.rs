//! Tests of the compare links of the version page (task 12.2, ADR-0049 (i), ADR-0050 (l), design
//! D7 and D8).
//!
//! The world is the approval world. `R-500` has the approved version 1 and the draft 2, and the
//! draft is based on version 1. `R-600` has one retired version. A test that needs more versions
//! runs the real commands over HTTP, as a user does.

use axum::http::StatusCode;

use super::actions_tests::LINKS_TAG;
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{ENGLISH, HTMX, TARGET_CONTENT, main_of};
use crate::test_support::{Reply, post_fields};

/// The headers of an HTMX request that targets `main`, in English.
const CONTENT: [(&str, &str); 3] = [ENGLISH, HTMX, TARGET_CONTENT];
/// The headers of a plain form request, in English.
const PLAIN: [(&str, &str); 1] = [ENGLISH];
/// The address of version 2 of `R-500`.
const V2: &str = "/recipes/R-500/versions/2";

/// Return the markup of the compare link of `R-500` with the new side `n` and the old side `m`.
fn link(n: u32, m: u32, text: &str) -> String {
    format!("<a class=\"target-link\" href=\"/recipes/R-500/versions/{n}/diff/{m}\">{text}</a>")
}

/// Return the row of links after the header `dl` of `main`.
fn row_of(main: &str) -> &str {
    let (_before, area) = main.split_once("</dl>").unwrap();
    let area = area.trim_start().strip_prefix(LINKS_TAG).expect(main);
    area.split("</p>").next().unwrap()
}

/// Assert that every part of `parts` stands in `row`, in the given order.
fn assert_in_order(row: &str, parts: &[&str]) {
    let positions: Vec<usize> = parts
        .iter()
        .map(|part| row.find(part).expect(part))
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{row}");
}

/// Send the form `body` to `path` as the Administrator with the headers `headers`.
async fn post(world: &AuditWorld, path: &str, body: &str, headers: &[(&str, &str)]) -> Reply {
    let router = world.fixture.router();
    post_fields(&router, path, Some(&world.data.admin), headers, body).await
}

/// Approve version 2 of `R-500` as the Administrator and return the answer.
async fn approve_v2(world: &AuditWorld) -> Reply {
    let row_version = world.row_version_of("R-500", 2).await;
    let body = format!("row_version={row_version}");
    let path = format!("{V2}/approve");
    let reply = post(world, &path, &body, &CONTENT).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply
}

/// Return the page `path` of the Administrator in the language `code`.
async fn page(world: &AuditWorld, code: &str, path: &str) -> Reply {
    let reply = world.get(&world.data.admin, code, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    reply
}

/// Assert that the row of links of the page `path` holds the history link and no compare link.
async fn assert_no_compare_link(world: &AuditWorld, path: &str) {
    let reply = page(world, "en", path).await;
    let row = row_of(main_of(&reply.body));
    assert!(!row.contains("/diff/"), "{path}: {row}");
    assert!(!row.contains("Compare"), "{path}: {row}");
    assert!(row.contains("/history\""), "{path}: {row}");
}

#[tokio::test]
async fn compare_draft_based_on_the_approved_version_shows_only_the_approved_link() {
    let world = audit_world().await;
    let reply = page(&world, "en", V2).await;
    let row = row_of(main_of(&reply.body));
    let approved = link(2, 1, "Compare with the approved version 1");
    assert!(row.contains(&approved), "{row}");
    assert_eq!(row.matches("/diff/").count(), 1, "{row}");
    assert!(!row.contains("base version"), "{row}");
    let history = "href=\"/recipes/R-500/history\"";
    assert_in_order(row, &["/versions/2/approve\"", &approved, history]);
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn compare_draft_based_on_a_retired_version_shows_both_links_in_order() {
    let world = audit_world().await;
    approve_v2(&world).await;
    let body = "based_on=1&change_note=Third+version";
    let created = post(&world, "/recipes/R-500/versions", body, &PLAIN).await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
    let reply = page(&world, "en", "/recipes/R-500/versions/3").await;
    let row = row_of(main_of(&reply.body));
    let approved = link(3, 2, "Compare with the approved version 2");
    let base = link(3, 1, "Compare with the base version 1");
    assert_eq!(row.matches("/diff/").count(), 2, "{row}");
    let history = "href=\"/recipes/R-500/history\"";
    assert_in_order(row, &["/versions/3/approve\"", &approved, &base, history]);
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn compare_approved_version_with_a_base_shows_only_the_base_link() {
    let world = audit_world().await;
    approve_v2(&world).await;
    let reply = page(&world, "en", V2).await;
    let row = row_of(main_of(&reply.body));
    let base = link(2, 1, "Compare with the base version 1");
    assert!(row.contains(&base), "{row}");
    assert_eq!(row.matches("/diff/").count(), 1, "{row}");
    assert!(!row.contains("approved version"), "{row}");
    assert!(!row.contains("/versions/2/diff/2"), "{row}");
}

#[tokio::test]
async fn compare_retired_version_links_to_the_newer_approved_version_with_it_as_n() {
    let world = audit_world().await;
    approve_v2(&world).await;
    let reply = page(&world, "en", "/recipes/R-500/versions/1").await;
    let row = row_of(main_of(&reply.body));
    let approved = link(2, 1, "Compare with the approved version 2");
    assert!(row.contains(&approved), "{row}");
    assert_eq!(row.matches("/diff/").count(), 1, "{row}");
}

#[tokio::test]
async fn compare_version_without_a_base_and_without_an_approved_version_shows_no_link() {
    let world = audit_world().await;
    // `R-600` has one retired version, no base, and no approved version.
    assert_no_compare_link(&world, "/recipes/R-600/versions/1").await;
}

#[tokio::test]
async fn compare_approved_version_without_a_base_shows_no_link_to_itself() {
    let world = audit_world().await;
    // Version 1 of `R-500` is the approved version itself and has no base.
    assert_no_compare_link(&world, "/recipes/R-500/versions/1").await;
}

#[tokio::test]
async fn compare_links_use_the_polish_texts() {
    let world = audit_world().await;
    let draft = page(&world, "pl", V2).await;
    let approved = link(2, 1, "Porównaj z zatwierdzoną wersją 1");
    assert!(
        row_of(main_of(&draft.body)).contains(&approved),
        "{}",
        draft.body
    );
    approve_v2(&world).await;
    let page_two = page(&world, "pl", V2).await;
    let base = link(2, 1, "Porównaj z wersją bazową 1");
    assert!(
        row_of(main_of(&page_two.body)).contains(&base),
        "{}",
        page_two.body
    );
}

#[tokio::test]
async fn compare_deleted_recipe_keeps_the_links_and_shows_no_form() {
    let world = audit_world().await;
    approve_v2(&world).await;
    let row_version = world.row_version_of("R-500", 2).await;
    let retire = format!("row_version={row_version}");
    let retired = post(&world, &format!("{V2}/retire"), &retire, &CONTENT).await;
    assert_eq!(retired.status, StatusCode::OK, "{}", retired.body);
    let deleted = post(&world, "/recipes/R-500/delete", "", &PLAIN).await;
    assert_eq!(deleted.status, StatusCode::SEE_OTHER, "{}", deleted.body);
    let reply = page(&world, "en", V2).await;
    let main = main_of(&reply.body);
    assert!(main.contains("Deleted"), "{main}");
    assert!(!main.contains("<form"), "{main}");
    let base = link(2, 1, "Compare with the base version 1");
    assert!(row_of(main).contains(&base), "{main}");
    assert!(main.contains("href=\"/recipes/R-500/history\""), "{main}");
}

#[tokio::test]
async fn compare_approve_answer_shows_the_links_of_the_new_state() {
    let world = audit_world().await;
    let before = page(&world, "en", V2).await;
    let approved = link(2, 1, "Compare with the approved version 1");
    assert!(row_of(main_of(&before.body)).contains(&approved));
    let reply = approve_v2(&world).await;
    assert!(
        reply.body.contains("Version 2 is approved."),
        "{}",
        reply.body
    );
    let row = row_of(&reply.body);
    let base = link(2, 1, "Compare with the base version 1");
    assert!(row.contains(&base), "{row}");
    assert!(!row.contains("approved version"), "{row}");
    assert_eq!(row.matches("/diff/").count(), 1, "{row}");
}
