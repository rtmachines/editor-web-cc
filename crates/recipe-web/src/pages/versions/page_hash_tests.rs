//! Tests of the content hash in the header of the version page (task 4.2, ADR-0108 (c), design D2).
//!
//! Spec "Content hash of a version". The world is the approval world. `R-500` has the approved
//! version 1 and the draft 2. `R-600` has the retired version 1. The hash of the export document
//! is the reference for the text of the header.

use axum::http::StatusCode;

use crate::i18n::Lang;
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{ENGLISH, HTMX, TARGET_CONTENT, main_of};
use crate::test_support::post_fields;
use crate::test_support::text;

/// The address of the approved version 1 of `R-500`.
const APPROVED: &str = "/recipes/R-500/versions/1";
/// The address of the draft version 2 of `R-500`.
const DRAFT: &str = "/recipes/R-500/versions/2";
/// The address of the retired version 1 of `R-600`.
const RETIRED: &str = "/recipes/R-600/versions/1";
/// The start of the `dd` that shows the hash.
const HASH_START: &str = "<dd class=\"data hash\">";
/// The length of a SHA-256 hash in hexadecimal characters.
const HASH_LENGTH: usize = 64;

/// Return the text of the first `dd` with the class `hash` in `main`, or `None`.
fn hash_of(main: &str) -> Option<&str> {
    let (_before, rest) = main.split_once(HASH_START)?;
    rest.split_once("</dd>").map(|(hash, _after)| hash)
}

/// Return the `content_sha256` of the export document at `path`, read as the Administrator.
async fn exported_hash(world: &AuditWorld, path: &str) -> String {
    let reply = world.get_en(&world.data.admin, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    let (_before, rest) = reply.body.split_once("\"content_sha256\": \"").unwrap();
    rest.split_once('"').unwrap().0.to_owned()
}

/// Return the hash that the page `path` shows to the Administrator, and assert its shape.
async fn shown_hash(world: &AuditWorld, path: &str) -> String {
    let reply = world.get_en(&world.data.admin, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    let hash = hash_of(main).expect(main);
    assert_eq!(hash.len(), HASH_LENGTH, "{hash}");
    assert!(
        hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "{hash}"
    );
    hash.to_owned()
}

#[tokio::test]
async fn version_page_of_an_approved_version_shows_the_hash_of_its_export() {
    let world = audit_world().await;
    let shown = shown_hash(&world, APPROVED).await;
    let exported = exported_hash(&world, "/export/v1/recipes/R-500/versions/1").await;
    assert_eq!(shown, exported);
}

#[tokio::test]
async fn version_page_of_an_operator_shows_the_hash_with_its_label_in_each_language() {
    let world = audit_world().await;
    for (lang, code) in [(Lang::En, "en"), (Lang::Pl, "pl")] {
        let reply = world.get(&world.operator, code, APPROVED).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        let label = format!("<dt>{}</dt>", text(lang, "version-content-hash"));
        let (_before, after) = main.split_once(&label).expect(main);
        assert!(after.trim_start().starts_with(HASH_START), "{after}");
    }
}

#[tokio::test]
async fn version_page_of_a_retired_version_shows_the_hash_of_its_export() {
    let world = audit_world().await;
    let shown = shown_hash(&world, RETIRED).await;
    let exported = exported_hash(&world, "/export/v1/recipes/R-600/versions/1").await;
    assert_eq!(shown, exported);
}

#[tokio::test]
async fn version_page_keeps_the_hash_after_the_approval_of_the_next_version_retires_it() {
    let world = audit_world().await;
    let before = shown_hash(&world, APPROVED).await;
    let row_version = world.row_version_of("R-500", 2).await;
    let body = format!("row_version={row_version}");
    let headers = [ENGLISH, HTMX, TARGET_CONTENT];
    let router = world.fixture.router();
    let path = format!("{DRAFT}/approve");
    let reply = post_fields(&router, &path, Some(&world.data.admin), &headers, &body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let page = world.get_en(&world.data.admin, APPROVED).await;
    assert!(page.body.contains("badge-retired"), "{}", page.body);
    assert_eq!(shown_hash(&world, APPROVED).await, before);
}

#[tokio::test]
async fn version_page_of_a_draft_shows_no_hash_row() {
    let world = audit_world().await;
    let reply = world.get_en(&world.operator, DRAFT).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert_eq!(hash_of(main), None, "{main}");
    assert!(!main.contains("Content hash"), "{main}");
}
