//! Tests of the two export routes: the file, the headers, the entry, and `HEAD` (design D11 T11).
//!
//! The data comes from `seed_approval_recipes` (design D14): `R-500` has the approved version 1,
//! and `R-600` has the retired version 1. Both versions have the real stored hash.

use axum::body::Body;
use axum::http::{Method, StatusCode};
use recipe_domain::audit::{AuditAction, RequestId};
use uuid::Uuid;

use super::{content_disposition, rows};
use crate::error::WebError;
use crate::export_path::is_export_path;
use crate::pages::test_helpers::CSP;
use crate::test_support::{Fixture, Reply, TestSession, call, get_with, request};

/// The export of the approved version of `R-500`.
const R500_APPROVED: &str = "/export/v1/recipes/R-500/approved";
/// The export of the approved version 1 of `R-500` by its number.
const R500_V1: &str = "/export/v1/recipes/R-500/versions/1";
/// The export of the retired version 1 of `R-600` by its number.
const R600_V1: &str = "/export/v1/recipes/R-600/versions/1";

/// Send `GET path` with the session cookie of `session`.
pub(super) async fn export(fixture: &Fixture, session: &TestSession, path: &str) -> Reply {
    let cookie = [("cookie", session.cookie.as_str())];
    get_with(&fixture.router(), path, &cookie).await
}

/// Return the `content_sha256` of each `VersionExported` entry, the oldest first.
pub(super) fn exports(fixture: &Fixture) -> Vec<String> {
    fixture
        .store
        .audit_entries()
        .iter()
        .filter(|entry| entry.action() == AuditAction::VersionExported)
        .map(|entry| {
            let after = entry.after().unwrap();
            assert_eq!(after.get("schema_version").unwrap().as_str(), Some("1"));
            after
                .get("content_sha256")
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

/// Return the text of the value of the first field `name` of a pretty JSON document.
///
/// A string keeps its quotes. The text ends before the comma or the line end.
fn field<'a>(body: &'a str, name: &str) -> &'a str {
    let key = format!("\"{name}\": ");
    let (_before, rest) = body.split_once(&key).unwrap();
    let end = rest.find([',', '\n']).unwrap_or(rest.len());
    rest.get(..end).unwrap()
}

/// Assert a file answer with the name `file_name` and the headers of `api.md` sections 6.3 and 9.
fn assert_file(reply: &Reply, file_name: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json");
    let disposition = format!("attachment; filename=\"{file_name}\"");
    assert_eq!(reply.header("content-disposition"), disposition);
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(reply.header("x-content-type-options"), "nosniff");
    assert_eq!(reply.header("content-security-policy"), CSP);
    assert!(reply.body.starts_with("{\n  \"schema_version\": \"1\",\n"));
}

/// Assert one new `VersionExported` entry since `before`, with the hash of the document.
fn assert_one_new_entry(fixture: &Fixture, before: &[String], body: &str) {
    let after = exports(fixture);
    assert_eq!(after.len(), before.len().checked_add(1).unwrap());
    let hash = field(body, "content_sha256").trim_matches('"');
    assert_eq!(after.last().unwrap(), hash);
    assert_eq!(hash.len(), 64);
}

#[tokio::test]
async fn export_approved_route_sends_the_approved_version_as_a_file_and_writes_one_entry() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let before = exports(&fixture);
    let reply = export(&fixture, &data.admin, R500_APPROVED).await;
    assert_file(&reply, "recipe_R-500_v1.json");
    let body = &reply.body;
    assert_eq!(field(body, "exported_by"), "\"admin\"");
    assert_eq!(field(body, "loadable"), "true");
    assert_eq!(field(body, "reference_number"), "\"R-500\"");
    assert_eq!(field(body, "number"), "1");
    assert_eq!(field(body, "status"), "\"Approved\"");
    assert_eq!(field(body, "approved_by"), "\"admin\"");
    assert_one_new_entry(&fixture, &before, body);
}

#[tokio::test]
async fn export_version_route_sends_an_approved_version_by_its_number() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let before = exports(&fixture);
    let reply = export(&fixture, &data.admin, R500_V1).await;
    assert_file(&reply, "recipe_R-500_v1.json");
    assert_eq!(field(&reply.body, "loadable"), "true");
    assert_eq!(field(&reply.body, "status"), "\"Approved\"");
    assert_one_new_entry(&fixture, &before, &reply.body);
}

#[tokio::test]
async fn export_version_route_names_the_file_of_a_retired_version_with_retired_first() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let before = exports(&fixture);
    let reply = export(&fixture, &data.admin, R600_V1).await;
    assert_file(&reply, "RETIRED_recipe_R-600_v1.json");
    assert_eq!(field(&reply.body, "loadable"), "false");
    assert_eq!(field(&reply.body, "reference_number"), "\"R-600\"");
    assert_eq!(field(&reply.body, "status"), "\"Retired\"");
    assert_one_new_entry(&fixture, &before, &reply.body);
}

#[tokio::test]
async fn export_two_exports_of_one_version_give_the_same_hash() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let first = export(&fixture, &data.admin, R500_APPROVED).await;
    let second = export(&fixture, &data.admin, R500_V1).await;
    let hash = |reply: &Reply| field(&reply.body, "content_sha256").to_owned();
    assert_eq!(hash(&first), hash(&second));
    assert_eq!(exports(&fixture).len(), 2);
}

#[tokio::test]
async fn export_in_read_only_mode_sends_the_file_and_writes_the_entry_on_both_routes() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    fixture.state.read_only().set(true);
    let cases = [
        (R500_APPROVED, "recipe_R-500_v1.json"),
        (R600_V1, "RETIRED_recipe_R-600_v1.json"),
    ];
    for (path, file_name) in cases {
        let before = exports(&fixture);
        let reply = export(&fixture, &data.admin, path).await;
        assert_file(&reply, file_name);
        assert_one_new_entry(&fixture, &before, &reply.body);
    }
}

/// The header holds the parsed reference, not the raw path text (domain-model section 2).
///
/// `ReferenceNumber::parse` folds `r-500` to `R-500`. A handler that copies the raw path text
/// into `Content-Disposition` would name the file `recipe_r-500_v1.json`.
#[tokio::test]
async fn export_approved_route_names_the_file_from_the_folded_reference() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let reply = export(&fixture, &data.admin, "/export/v1/recipes/r-500/approved").await;
    assert_file(&reply, "recipe_R-500_v1.json");
    assert_eq!(field(&reply.body, "reference_number"), "\"R-500\"");
}

/// Send `HEAD path` with the session cookie of `session`.
async fn head(fixture: &Fixture, session: &TestSession, path: &str) -> Reply {
    let head = request(Method::HEAD, path)
        .header("cookie", session.cookie.as_str())
        .body(Body::empty())
        .unwrap();
    call(&fixture.router(), head).await
}

#[tokio::test]
async fn export_head_on_both_routes_gets_the_answer_of_an_unlisted_method_and_no_entry() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    for path in [R500_APPROVED, R600_V1] {
        let before = fixture.snapshot();
        let reply = head(&fixture, &data.admin, path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(reply.header("content-disposition"), "", "{path}");
        assert_ne!(reply.header("content-type"), "application/json", "{path}");
        assert_eq!(fixture.snapshot(), before, "{path}");
    }
    assert!(exports(&fixture).is_empty());
}

#[tokio::test]
async fn export_head_on_a_page_route_still_uses_the_row_of_get() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let reply = head(&fixture, &data.admin, "/recipes").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

#[test]
fn export_each_row_path_starts_with_the_prefix_of_the_export_routes() {
    let rows = rows();
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row.method, Method::GET, "{}", row.path);
        assert!(is_export_path(row.path), "{}", row.path);
    }
}

#[test]
fn export_content_disposition_names_the_file_as_an_attachment() {
    let request_id = RequestId::from(Uuid::nil());
    let value = content_disposition("recipe_R-100_v3.json", request_id).unwrap();
    assert_eq!(value, "attachment; filename=\"recipe_R-100_v3.json\"");
}

#[test]
fn export_content_disposition_with_a_bad_name_gives_web_010_and_no_panic() {
    let request_id = RequestId::from(Uuid::nil());
    let error = content_disposition("bad\nname.json", request_id).unwrap_err();
    assert!(matches!(error, WebError::Internal { .. }), "{error:?}");
}
