//! Tests of the two machine routes of the transfer client (design D3 and D4, ADR-0109 (g), (h), (j)).
//!
//! The data comes from `seed_approval_recipes`. `R-100` on `M7` has an approved version 2 with the
//! zero hash of the memory seed. `R-500` on `M7` has the approved version 1 with the real hash and
//! a draft version 2. `R-600` on `M7` has only a retired version. `R-200` on `M20` has only a
//! draft. The `ExportClient` account `user1` sends every request with HTTP Basic.

use axum::body::Body;
use axum::http::{Method, StatusCode};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_domain::audit::AuditAction;
use recipe_domain::identity::{Role, UserId};
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::{Recipe, RecipeId, RecipeVersionId, ReferenceNumber};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::test_support::plant::World;
use crate::test_support::recipes::approval::ApprovalData;
use crate::test_support::recipes::date_world::affected_approved_world;
use crate::test_support::{Fixture, Reply, call, get_with, request};

/// The password of the seeded `ExportClient` account `user1`.
const PASSWORD: &str = "tr4nsfer-s3cret";
/// The number of the account `user1`.
const CLIENT_NUMBER: u8 = 1;
/// The list of the live machine `M7`.
const M7_LIST: &str = "/export/v1/machines/M7/recipes";
/// The approved version of `R-500` on `M7`.
const M7_R500: &str = "/export/v1/machines/M7/recipes/R-500/approved";
/// The instant of the archive of `M7` in Unix seconds.
const ARCHIVED_AT_SECONDS: i64 = 1_000_000;

fn authorization() -> String {
    format!("Basic {}", STANDARD.encode(format!("user1:{PASSWORD}")))
}

/// Return the identifier of the account `user1`.
pub(super) fn client() -> UserId {
    UserId::from(Uuid::from_u128(u128::from(CLIENT_NUMBER)))
}

/// Send `GET path` as the account `user1`.
pub(super) async fn client_get(fixture: &Fixture, path: &str) -> Reply {
    let header = authorization();
    get_with(&fixture.router(), path, &[("authorization", &header)]).await
}

/// Build a fixture with the account `user1` and the approval data.
pub(super) async fn seeded() -> (Fixture, ApprovalData) {
    let fixture = Fixture::new();
    fixture.seed_account(CLIENT_NUMBER, Role::ExportClient, PASSWORD, false);
    let data = fixture.seed_approval_recipes().await;
    (fixture, data)
}

/// Store the recipe `R-501` of `M7`. It has only a live draft.
fn seed_draft_only(fixture: &Fixture, data: &ApprovalData) {
    let recipe = Recipe::create(
        RecipeId::from(Uuid::from_u128(0x7357_0a01_0000_0000_0000_0000_0000_0001)),
        data.recipes.plant.m7,
        ReferenceNumber::parse("R-501").unwrap(),
        Text::parse("Draft only").unwrap(),
        RecipeVersionId::from(Uuid::from_u128(0x7357_0a02_0000_0000_0000_0000_0000_0001)),
    );
    fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::S7_300, data.anowak_id);
}

/// Archive `M7`, so no live machine `M7` remains.
///
/// The use case `ArchiveMachine` refuses a machine with a live draft, and `M7` holds two drafts.
/// The helper therefore stores the archived machine over the live row.
fn archive_m7(fixture: &Fixture, data: &ApprovalData) {
    let mut machine = fixture
        .store
        .state()
        .machine(data.recipes.plant.m7)
        .unwrap();
    let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(ARCHIVED_AT_SECONDS).unwrap());
    machine.archive(at, None).unwrap();
    fixture.store.seed_machine(&machine);
}

/// Return the actor of each `VersionExported` entry, the oldest first.
pub(super) fn exported_actors(fixture: &Fixture) -> Vec<Option<UserId>> {
    fixture
        .store
        .audit_entries()
        .iter()
        .filter(|entry| entry.action() == AuditAction::VersionExported)
        .map(|entry| entry.actor())
        .collect()
}

pub(super) fn audit_count(fixture: &Fixture) -> usize {
    fixture.store.audit_entries().len()
}

/// Return the text of the value of the first field `name` of a pretty JSON body.
pub(super) fn field<'a>(body: &'a str, name: &str) -> &'a str {
    let key = format!("\"{name}\": ");
    let (_before, rest) = body.split_once(&key).unwrap();
    let end = rest.find([',', '\n']).unwrap_or(rest.len());
    rest.get(..end).unwrap()
}

/// Return the text of the row of `recipes[]` for `reference`, up to the end of the row.
fn row_of<'a>(body: &'a str, reference: &str) -> &'a str {
    let key = format!("\"reference_number\": \"{reference}\"");
    let (_before, rest) = body.split_once(&key).unwrap();
    rest.split_once('}').unwrap().0
}

/// Count the rows of `recipes[]` in a list body.
fn rows(body: &str) -> usize {
    body.matches("\"reference_number\"").count()
}

#[tokio::test]
async fn export_list_holds_the_approved_recipes_and_no_draft_or_retired_recipe() {
    let (fixture, data) = seeded().await;
    seed_draft_only(&fixture, &data);
    let before = audit_count(&fixture);

    let reply = client_get(&fixture, M7_LIST).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(field(&reply.body, "code"), "\"M7\"");
    assert_eq!(field(&reply.body, "archived"), "false");
    assert_eq!(rows(&reply.body), 2, "{}", reply.body);
    let first = reply.body.find("\"R-100\"").unwrap();
    let second = reply.body.find("\"R-500\"").unwrap();
    assert!(first < second, "the rows are not in reference order");
    let row = row_of(&reply.body, "R-500");
    assert_eq!(field(row, "version_number"), "1");
    assert!(row.contains("\"name\": "), "{row}");
    assert!(row.contains("\"approved_at\": "), "{row}");
    assert_eq!(field(row, "content_sha256").len(), 66);
    assert!(!reply.body.contains("R-501"), "{}", reply.body);
    assert!(!reply.body.contains("R-600"), "{}", reply.body);
    assert_eq!(audit_count(&fixture), before, "the list wrote an entry");
}

#[tokio::test]
async fn export_list_hash_equals_the_hash_of_the_approved_document() {
    let (fixture, _data) = seeded().await;

    let list = client_get(&fixture, M7_LIST).await;
    let document = client_get(&fixture, M7_R500).await;

    assert_eq!(list.status, StatusCode::OK, "{}", list.body);
    assert_eq!(document.status, StatusCode::OK, "{}", document.body);
    assert_eq!(
        field(row_of(&list.body, "R-500"), "content_sha256"),
        field(&document.body, "content_sha256")
    );
}

#[tokio::test]
async fn export_list_of_a_machine_without_an_approved_recipe_is_empty() {
    let (fixture, _data) = seeded().await;

    let reply = client_get(&fixture, "/export/v1/machines/M20/recipes").await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(rows(&reply.body), 0, "{}", reply.body);
    assert_eq!(field(&reply.body, "code"), "\"M20\"");
}

#[tokio::test]
async fn export_list_of_an_archived_machine_marks_the_machine_archived() {
    let (fixture, data) = seeded().await;
    archive_m7(&fixture, &data);

    let reply = client_get(&fixture, M7_LIST).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(field(&reply.body, "archived"), "true");
    assert_eq!(rows(&reply.body), 2, "{}", reply.body);
    assert_eq!(field(row_of(&reply.body, "R-500"), "version_number"), "1");
}

#[tokio::test]
async fn export_list_of_an_unknown_machine_gives_not_found() {
    let (fixture, _data) = seeded().await;

    let reply = client_get(&fixture, "/export/v1/machines/M99/recipes").await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-005"), "{}", reply.body);
}

#[tokio::test]
async fn export_list_with_a_bad_machine_code_gives_not_found() {
    let (fixture, _data) = seeded().await;
    for path in [
        "/export/v1/machines/m7/recipes",
        "/export/v1/machines/%FF/recipes",
    ] {
        let reply = client_get(&fixture, path).await;

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
async fn export_list_needs_the_basic_header() {
    let (fixture, _data) = seeded().await;

    let reply = get_with(&fixture.router(), M7_LIST, &[]).await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-012"), "{}", reply.body);
}

#[tokio::test]
async fn export_list_cross_site_request_gives_forbidden_before_the_password_check() {
    let (fixture, _data) = seeded().await;
    let header = authorization();

    let reply = get_with(
        &fixture.router(),
        M7_LIST,
        &[("authorization", &header), ("sec-fetch-site", "cross-site")],
    )
    .await;

    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"), "{}", reply.body);
    assert_eq!(fixture.store.state().login_attempt_count(), 0);
}

#[tokio::test]
async fn export_approved_route_sends_the_v1_document_and_writes_one_entry_of_the_account() {
    let (fixture, _data) = seeded().await;
    let before = exported_actors(&fixture);

    let reply = client_get(&fixture, M7_R500).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(
        reply.header("content-disposition"),
        "attachment; filename=\"recipe_R-500_v1.json\""
    );
    assert!(reply.body.starts_with("{\n  \"schema_version\": \"1\",\n"));
    assert_eq!(field(&reply.body, "loadable"), "true");
    assert_eq!(field(&reply.body, "exported_by"), "\"user1\"");
    let after = exported_actors(&fixture);
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(after.last(), Some(&Some(client())));
}

/// Build the world of an approved version that breaks the current rules, with the account `user1`.
///
/// The version is `R-400` on `M30`. Its stored `DATE` 72700 passes the earlier rule set
/// (ADR-0119 (f), (g)).
pub(super) async fn affected_world() -> World {
    let world = affected_approved_world().await;
    world
        .fixture
        .seed_account(CLIENT_NUMBER, Role::ExportClient, PASSWORD, false);
    world
}

#[tokio::test]
async fn export_approved_route_of_a_version_with_a_rule_fault_sends_loadable_false() {
    let world = affected_world().await;

    let reply = client_get(
        &world.fixture,
        "/export/v1/machines/M30/recipes/R-400/approved",
    )
    .await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    // The version stays `Approved`, so the file name has no `RETIRED_` prefix.
    assert_eq!(
        reply.header("content-disposition"),
        "attachment; filename=\"recipe_R-400_v1.json\""
    );
    assert_eq!(field(&reply.body, "loadable"), "false");
    assert_eq!(field(&reply.body, "status"), "\"Approved\"");
    assert_eq!(exported_actors(&world.fixture), vec![Some(client())]);
}

#[tokio::test]
async fn export_approved_route_of_an_archived_machine_sends_the_document() {
    let (fixture, data) = seeded().await;
    archive_m7(&fixture, &data);
    let before = exported_actors(&fixture).len();

    let reply = client_get(&fixture, M7_R500).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(field(&reply.body, "number"), "1");
    assert_eq!(field(&reply.body, "loadable"), "true");
    let after = exported_actors(&fixture);
    assert_eq!(after.len(), before + 1);
    assert_eq!(after.last(), Some(&Some(client())));
}

#[tokio::test]
async fn export_approved_route_of_another_machine_gives_not_found_and_no_entry() {
    let (fixture, _data) = seeded().await;
    let before = exported_actors(&fixture).len();

    let reply = client_get(&fixture, "/export/v1/machines/M20/recipes/R-500/approved").await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-005"), "{}", reply.body);
    assert_eq!(exported_actors(&fixture).len(), before);
}

#[tokio::test]
async fn export_approved_route_without_an_approved_version_gives_no_approved_version() {
    let (fixture, _data) = seeded().await;
    let before = exported_actors(&fixture).len();

    let reply = client_get(&fixture, "/export/v1/machines/M20/recipes/R-200/approved").await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    assert!(reply.body.contains("E-EXP-001"), "{}", reply.body);
    assert_eq!(exported_actors(&fixture).len(), before);
}

#[tokio::test]
async fn export_approved_route_with_a_bad_path_value_gives_not_found() {
    let (fixture, _data) = seeded().await;
    for path in [
        "/export/v1/machines/m7/recipes/R-500/approved",
        "/export/v1/machines/M7/recipes/%FF/approved",
        "/export/v1/machines/M99/recipes/R-500/approved",
    ] {
        let reply = client_get(&fixture, path).await;

        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-005"), "{path}: {}", reply.body);
    }
    assert!(exported_actors(&fixture).is_empty());
}

#[tokio::test]
async fn export_machine_routes_accept_only_get() {
    let (fixture, _data) = seeded().await;
    let header = authorization();
    for path in [M7_LIST, M7_R500] {
        let builder = request(Method::HEAD, path).header("authorization", &header);

        let head = call(&fixture.router(), builder.body(Body::empty()).unwrap()).await;

        assert_eq!(head.status, StatusCode::NOT_FOUND, "{path}: {}", head.body);
    }
    assert!(exported_actors(&fixture).is_empty());
}
