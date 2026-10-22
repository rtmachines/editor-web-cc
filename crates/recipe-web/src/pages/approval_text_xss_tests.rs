//! Tests of the XSS corpus for the texts of a parameter on the approval preview and the diff page.
//! They cover task 13.4 and design D10, row "Texts of a parameter on the new pages".
//!
//! The fields are `name_en`, `description_en`, the name and the description of a translation, and
//! the text of a `STRING` value. A test stores a payload through the real form routes as `anowak`,
//! then reads the pages as the Administrator. Each stored text shows as escaped text in its cell. No
//! answer holds the raw payload, no `hx-*` attribute holds it, and each `<svg` is the fixed tag of
//! its icon.
//!
//! - A `Text` field refuses a control character (`reliability.md` section 4.3). The tests send the
//!   payloads without a control character to `name_en`, `description_en`, and the translation. The
//!   test `recipe_xss_text_fields_accept_a_valid_text_and_refuse_a_control_character` of
//!   `recipe_xss_tests.rs` covers that refusal.
//! - The value check of a `STRING` refuses a character outside Latin-1 and a text that is longer
//!   than the type (`reliability.md` section 4.4, ADR-0028). It does not refuse a control
//!   character. The corpus holds only ASCII. Its longest payload has 150 characters.
//!   `STRING` has 254 characters on the S7-300 machine `M7`, so it accepts every payload, also the
//!   NUL payload.
//! - The test `approval_text_xss_a_string_value_accepts_every_payload_and_shows_it_escaped` sends
//!   every payload as the value of a new parameter. No payload gives a refusal code.
//! - The test
//!   `approval_text_xss_a_changed_string_value_is_escaped_on_the_new_side_of_the_preview` changes
//!   the value of an approved parameter. The value row of the preview then shows the payload on
//!   its new side.
//!
//! The draft version 2 of `R-500` is the new side of `/diff/1` and the old side of the reverse diff
//! `/versions/1/diff/2`. The test
//! `approval_text_xss_changed_texts_are_escaped_on_both_sides_of_the_diff_and_the_preview` approves
//! version 2 and starts version 3. The approval preview of version 3 then shows the payload on its
//! old side.

use axum::http::StatusCode;

use super::audit::test_world::{AuditWorld, audit_world};
use super::test_helpers::{askama_escape, encode, payloads};
use super::xss_kit::{assert_safe, plain_payloads, post_as_engineer};
use crate::test_support::plant::{ENGLISH, HTMX, TARGET_CONTENT, main_of};
use crate::test_support::{Reply, post_fields};

/// The text that replaces a payload when a test needs a clean second side.
const PLAIN: &str = "Plain text";
/// The draft version 2 of `R-500`.
const DRAFT: &str = "/recipes/R-500/versions/2";
/// The approval preview of the draft version 2.
const PREVIEW: &str = "/recipes/R-500/versions/2/approve";
/// The diff of version 2 against version 1: the draft is the new side.
const DRAFT_NEW: &str = "/recipes/R-500/versions/2/diff/1";
/// The diff of version 1 against version 2: the draft is the old side.
const DRAFT_OLD: &str = "/recipes/R-500/versions/1/diff/2";
/// The S7 address of the `STRING` parameter `HOST`. No other parameter of `R-500` uses `DB20`.
const HOST_ADDRESS: &str = "DB20.DBB0";
/// The ordinal of the parameter `HOST`. Version 2 of `R-500` uses the ordinals 0 to 4.
const HOST_ORDINAL: u32 = 6;
/// The limit row of `HOST`: the longest text of the type `STRING` on the S7-300 machine `M7`.
const HOST_LIMIT: &str = "limits%5B0%5D.leaf_path=&limits%5B0%5D.max_length=254";
/// The limit row of `SPEED` that a definition form must send again, so the limit stays.
const SPEED_LIMIT: &str = "limits%5B0%5D.leaf_path=&limits%5B0%5D.min=0&limits%5B0%5D.max=3000";

/// Return the key of the parameter `tag` in the version `number` of `R-500`, read from its link.
async fn key_of(world: &AuditWorld, number: u32, tag: &str) -> String {
    let path = format!("/recipes/R-500/versions/{number}/parameters");
    let page = world.get_en(&world.data.admin, &path).await;
    let main = main_of(&page.body);
    let end = main.find(&format!(">{tag}<")).unwrap();
    let (before, _after) = main.split_at(end);
    let (_head, link) = before.rsplit_once("href=\"").unwrap();
    let href = link.split('"').next().unwrap();
    href.rsplit('/').next().unwrap().to_owned()
}

/// Return the form body of `HOST` with the three texts: the name, the description, and the value.
///
/// The parameter has a limit on its length, because an approval needs a process limit.
fn host_body(row_version: i64, texts: (&str, &str, &str)) -> String {
    let (name, description, value) = texts;
    format!(
        "row_version={row_version}&unit_path=U1&tag=HOST&name_en={}&description_en={}&type=STRING&value_type=STRING&s7_address={HOST_ADDRESS}&ordinal={HOST_ORDINAL}&value={}&{HOST_LIMIT}",
        encode(name),
        encode(description),
        encode(value),
    )
}

/// Add the `STRING` parameter `HOST` to the version `number` with the three texts.
async fn add_host(world: &AuditWorld, number: u32, texts: (&str, &str, &str)) {
    let row_version = world.row_version_of("R-500", number).await;
    let body = host_body(row_version, texts);
    let path = format!("/recipes/R-500/versions/{number}/parameters");
    post_as_engineer(world, &path, &body).await;
}

/// Save the definition of `HOST` in the version `number` with the three texts.
async fn edit_host(world: &AuditWorld, number: u32, texts: (&str, &str, &str)) {
    let key = key_of(world, number, "HOST").await;
    let row_version = world.row_version_of("R-500", number).await;
    let body = host_body(row_version, texts);
    let path = format!("/recipes/R-500/versions/{number}/parameters/{key}/definition");
    post_as_engineer(world, &path, &body).await;
}

/// Save the definition of `SPEED` in the version `number` with the English name and description.
async fn edit_speed(world: &AuditWorld, number: u32, texts: (&str, &str)) {
    let (name, description) = texts;
    let key = key_of(world, number, "SPEED").await;
    let row_version = world.row_version_of("R-500", number).await;
    let body = format!(
        "row_version={row_version}&unit_path=U1&tag=SPEED&name_en={}&description_en={}&type=INT&value_type=INT&s7_address=DB10.DBW0&ordinal=0&value=1200&{SPEED_LIMIT}",
        encode(name),
        encode(description),
    );
    let path = format!("/recipes/R-500/versions/{number}/parameters/{key}/definition");
    post_as_engineer(world, &path, &body).await;
}

/// Save the Polish translation of `SPEED` in the version `number`.
async fn translate_speed(world: &AuditWorld, number: u32, texts: (&str, &str)) {
    let (name, description) = texts;
    let key = key_of(world, number, "SPEED").await;
    let row_version = world.row_version_of("R-500", number).await;
    let body = format!(
        "row_version={row_version}&name={}&description={}",
        encode(name),
        encode(description),
    );
    let path = format!("/recipes/R-500/versions/{number}/parameters/{key}/translations/pl");
    post_as_engineer(world, &path, &body).await;
}

/// Read each page as the Administrator, in English. Check the status and `assert_safe`.
async fn read_pages(world: &AuditWorld, paths: &[&str], payload: &str) -> Vec<(String, Reply)> {
    let mut replies = Vec::new();
    for path in paths {
        let reply = world.get_en(&world.data.admin, path).await;
        let what = format!("{path} {payload:?}");
        assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
        assert_safe(&reply, payload, &what);
        replies.push(((*path).to_owned(), reply));
    }
    replies
}

/// Assert that `main` of each reply holds `cell` exactly `count` times.
fn assert_cells(replies: &[(String, Reply)], cell: &str, count: usize) {
    for (path, reply) in replies {
        let main = main_of(&reply.body);
        let found = main.matches(cell).count();
        assert_eq!(found, count, "{path}: {found} times {cell}");
    }
}

/// Return the cell of an English text: `name_en` or `description_en`. It carries `lang="en"`
/// (finding CX-DR-1).
fn text_cell(text: &str) -> String {
    format!("<td lang=\"en\">{}</td>", askama_escape(text))
}

/// Return the cell of a value.
fn value_cell(text: &str) -> String {
    format!("<td class=\"diff-value\">{}</td>", askama_escape(text))
}

/// Return the labeled text of the Polish translation. The text carries `lang="pl"`, the label
/// does not (finding CX-DR-1).
fn pair_cell(label: &str, text: &str) -> String {
    format!(
        "<dt>{label}</dt><dd lang=\"pl\">{}</dd>",
        askama_escape(text)
    )
}

/// The three views of the draft that show the payload: the preview and both diffs.
const DRAFT_PAGES: [&str; 3] = [PREVIEW, DRAFT_NEW, DRAFT_OLD];
/// The three views of the draft version 3 after the approval of version 2: the preview and both
/// diffs against version 2.
const VERSION_3_PAGES: [&str; 3] = [
    "/recipes/R-500/versions/3/approve",
    "/recipes/R-500/versions/3/diff/2",
    "/recipes/R-500/versions/2/diff/3",
];

#[tokio::test]
async fn approval_text_xss_name_en_is_escaped_in_the_added_row_and_the_definition_row() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        add_host(&world, 2, (&payload, "A host", PLAIN)).await;
        edit_speed(&world, 2, (&payload, "A parameter of the approval data")).await;
        let replies = read_pages(&world, &DRAFT_PAGES, &payload).await;
        assert_cells(&replies, &text_cell(&payload), 2);
    }
}

#[tokio::test]
async fn approval_text_xss_description_en_is_escaped_in_the_definition_row() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        edit_speed(&world, 2, ("SPEED", &payload)).await;
        let replies = read_pages(&world, &DRAFT_PAGES, &payload).await;
        assert_cells(&replies, &text_cell(&payload), 1);
    }
}

#[tokio::test]
async fn approval_text_xss_translation_texts_are_escaped_in_the_translation_row() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        translate_speed(&world, 2, (&payload, &payload)).await;
        let replies = read_pages(&world, &DRAFT_PAGES, &payload).await;
        assert_cells(&replies, &pair_cell("Name", &payload), 1);
        assert_cells(&replies, &pair_cell("Description", &payload), 1);
    }
}

#[tokio::test]
async fn approval_text_xss_a_string_value_accepts_every_payload_and_shows_it_escaped() {
    for payload in payloads() {
        let world = audit_world().await;
        add_host(&world, 2, ("Host", "A host", &payload)).await;
        let replies = read_pages(&world, &DRAFT_PAGES, &payload).await;
        assert_cells(&replies, &value_cell(&payload), 1);
    }
}

#[tokio::test]
async fn approval_text_xss_changed_texts_are_escaped_on_both_sides_of_the_diff_and_the_preview() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        add_host(&world, 2, (&payload, &payload, &payload)).await;
        edit_speed(&world, 2, (&payload, &payload)).await;
        translate_speed(&world, 2, (&payload, &payload)).await;
        approve_version_2_and_start_version_3(&world).await;
        let plain = (PLAIN, PLAIN);
        edit_host(&world, 3, (PLAIN, PLAIN, PLAIN)).await;
        edit_speed(&world, 3, plain).await;
        translate_speed(&world, 3, plain).await;
        let replies = read_pages(&world, &VERSION_3_PAGES, &payload).await;
        assert_cells(&replies, &text_cell(&payload), 4);
        assert_cells(&replies, &value_cell(&payload), 1);
        assert_cells(&replies, &pair_cell("Name", &payload), 1);
        assert_cells(&replies, &pair_cell("Description", &payload), 1);
    }
}

#[tokio::test]
async fn approval_text_xss_a_changed_string_value_is_escaped_on_the_new_side_of_the_preview() {
    for payload in payloads() {
        let world = audit_world().await;
        add_host(&world, 2, ("Host", "A host", PLAIN)).await;
        approve_version_2_and_start_version_3(&world).await;
        edit_host(&world, 3, ("Host", "A host", &payload)).await;
        let replies = read_pages(&world, &VERSION_3_PAGES, &payload).await;
        assert_cells(&replies, &value_cell(&payload), 1);
        assert_cells(&replies, &value_cell(PLAIN), 1);
    }
}

/// Approve the draft version 2 as the Administrator, then start version 3 on it as `anowak`.
async fn approve_version_2_and_start_version_3(world: &AuditWorld) {
    let row_version = world.row_version_of("R-500", 2).await;
    let headers = [ENGLISH, HTMX, TARGET_CONTENT];
    let router = world.fixture.router();
    let body = format!("row_version={row_version}");
    let approved = post_fields(
        &router,
        &format!("{DRAFT}/approve"),
        Some(&world.data.admin),
        &headers,
        &body,
    )
    .await;
    assert_eq!(approved.status, StatusCode::OK, "{}", approved.body);
    let created = post_as_engineer(
        world,
        "/recipes/R-500/versions",
        "based_on=2&change_note=Third+version",
    )
    .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
}
