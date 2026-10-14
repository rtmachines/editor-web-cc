//! Tests of the sections 3.0, 4.4, and 4.7: `set_updated_at`, `bump_row_version`, and
//! `recipe_version_transition`.
//!
//! The tests run as the table owner `recipe_migrator`. `recipe_app` has no grant before
//! `0003_grants.sql`, and task 5.2 repeats the forbidden transitions as `recipe_app`.

use sqlx::PgConnection;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use crate::support::fixtures::{self, Draft, TWIN_CAT};
use crate::support::{Role, TestDatabase, assert_trigger};

const TRANSITION: &str = "E-RCP-033";

async fn plant() -> (PgConnection, Draft) {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let draft = Draft::create(&mut conn).await;
    (conn, draft)
}

async fn approved() -> (PgConnection, Draft) {
    let (mut conn, draft) = plant().await;
    fixtures::approve(&mut conn, draft.version, draft.user).await;
    (conn, draft)
}

async fn retired() -> (PgConnection, Draft) {
    let (mut conn, draft) = approved().await;
    fixtures::retire(&mut conn, draft.version).await;
    (conn, draft)
}

/// Run one statement with the version as `$1`. The statement is a literal of the test.
async fn run(
    conn: &mut PgConnection,
    statement: &'static str,
    version: Uuid,
) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query(statement).bind(version).execute(conn).await
}

async fn row_version(conn: &mut PgConnection, version: Uuid) -> i64 {
    sqlx::query_scalar("SELECT row_version FROM recipe_version WHERE id = $1")
        .bind(version)
        .fetch_one(conn)
        .await
        .unwrap()
}

async fn status(conn: &mut PgConnection, version: Uuid) -> String {
    sqlx::query_scalar("SELECT status FROM recipe_version WHERE id = $1")
        .bind(version)
        .fetch_one(conn)
        .await
        .unwrap()
}

// ---- 4.4 bump_row_version (I12) ----

#[tokio::test]
async fn bump_an_allowed_update_adds_one_to_the_row_version() {
    let (mut conn, draft) = plant().await;
    assert_eq!(row_version(&mut conn, draft.version).await, 1);

    let first = run(
        &mut conn,
        "UPDATE recipe_version SET change_note = 'one' WHERE id = $1",
        draft.version,
    )
    .await
    .unwrap();
    let second = run(
        &mut conn,
        "UPDATE recipe_version SET change_note = 'two' WHERE id = $1",
        draft.version,
    )
    .await
    .unwrap();

    assert_eq!((first.rows_affected(), second.rows_affected()), (1, 1));
    assert_eq!(row_version(&mut conn, draft.version).await, 3);
}

#[tokio::test]
async fn bump_ignores_a_row_version_that_the_statement_sets() {
    let (mut conn, draft) = plant().await;

    run(
        &mut conn,
        "UPDATE recipe_version SET row_version = 99 WHERE id = $1",
        draft.version,
    )
    .await
    .unwrap();

    assert_eq!(row_version(&mut conn, draft.version).await, 2);
}

#[tokio::test]
async fn bump_sets_updated_at_to_the_time_of_the_transaction() {
    let (mut conn, draft) = plant().await;

    let is_now: bool = sqlx::query_scalar(
        "UPDATE recipe_version SET change_note = 'later' \
         WHERE id = $1 AND updated_at < now() RETURNING updated_at = now()",
    )
    .bind(draft.version)
    .fetch_one(&mut conn)
    .await
    .unwrap();

    assert!(is_now);
}

#[tokio::test]
async fn bump_a_rejected_update_leaves_the_row_version_unchanged() {
    let (mut conn, draft) = approved().await;
    let before = row_version(&mut conn, draft.version).await;

    let rejected = run(
        &mut conn,
        "UPDATE recipe_version SET status = 'draft', approved_by = NULL, approved_at = NULL, \
             content_sha256 = NULL WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(rejected, TRANSITION);
    assert_eq!(row_version(&mut conn, draft.version).await, before);
}

#[tokio::test]
async fn bump_fires_before_the_transition_trigger() {
    let (mut conn, _draft) = plant().await;

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT tgname::text FROM pg_trigger \
         WHERE tgrelid = 'recipe_version'::regclass AND NOT tgisinternal \
         ORDER BY tgname COLLATE \"C\"",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();

    assert_eq!(
        names,
        ["recipe_version_a_bump", "recipe_version_b_transition"]
    );
}

// ---- 4.7 allowed transitions ----

#[tokio::test]
async fn transition_a_draft_can_change_and_stay_a_draft() {
    let (mut conn, draft) = plant().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET change_note = 'edit' WHERE id = $1",
        draft.version,
    )
    .await
    .unwrap();

    assert_eq!(result.rows_affected(), 1);
    assert_eq!(status(&mut conn, draft.version).await, "draft");
}

#[tokio::test]
async fn transition_a_draft_can_become_approved_with_the_status_fields() {
    let (mut conn, draft) = plant().await;

    fixtures::approve(&mut conn, draft.version, draft.user).await;

    assert_eq!(status(&mut conn, draft.version).await, "approved");
    assert_eq!(row_version(&mut conn, draft.version).await, 2);
}

#[tokio::test]
async fn transition_an_approved_version_can_become_retired() {
    let (mut conn, draft) = approved().await;

    fixtures::retire(&mut conn, draft.version).await;

    assert_eq!(status(&mut conn, draft.version).await, "retired");
}

#[tokio::test]
async fn transition_a_draft_can_be_discarded() {
    let (mut conn, draft) = plant().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET deleted_at = now() WHERE id = $1",
        draft.version,
    )
    .await
    .unwrap();

    assert_eq!(result.rows_affected(), 1);
}

// ---- 4.7 forbidden transitions (E-RCP-033) ----

#[tokio::test]
async fn transition_an_approved_version_cannot_become_a_draft() {
    let (mut conn, draft) = approved().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET status = 'draft', approved_by = NULL, approved_at = NULL, \
             content_sha256 = NULL WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
    assert_eq!(status(&mut conn, draft.version).await, "approved");
}

#[tokio::test]
async fn transition_a_retired_version_cannot_become_approved() {
    let (mut conn, draft) = retired().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET status = 'approved', retired_at = NULL WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
    assert_eq!(status(&mut conn, draft.version).await, "retired");
}

#[tokio::test]
async fn transition_a_retired_version_cannot_become_a_draft() {
    let (mut conn, draft) = retired().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET status = 'draft', approved_by = NULL, approved_at = NULL, \
             retired_at = NULL, content_sha256 = NULL WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
    assert_eq!(status(&mut conn, draft.version).await, "retired");
}

#[tokio::test]
async fn transition_a_retired_version_never_changes() {
    let (mut conn, draft) = retired().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET change_note = 'late' WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

/// The grant of `recipe_app` allows an update of `change_note`. The trigger still rejects it.
#[tokio::test]
async fn transition_a_retired_version_never_changes_for_recipe_app() {
    let database = TestDatabase::create().await;
    let mut owner = database.connect(Role::Migrator).await;
    let draft = Draft::create(&mut owner).await;
    fixtures::approve(&mut owner, draft.version, draft.user).await;
    fixtures::retire(&mut owner, draft.version).await;
    let mut app = database.connect(Role::App).await;

    let result = run(
        &mut app,
        "UPDATE recipe_version SET change_note = 'late' WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
    assert_eq!(status(&mut owner, draft.version).await, "retired");
}

#[tokio::test]
async fn transition_a_discarded_draft_cannot_be_restored() {
    let (mut conn, draft) = plant().await;
    run(
        &mut conn,
        "UPDATE recipe_version SET deleted_at = now() WHERE id = $1",
        draft.version,
    )
    .await
    .unwrap();

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET deleted_at = NULL WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn transition_an_approved_version_cannot_be_discarded() {
    let (mut conn, draft) = approved().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET deleted_at = now() WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

// ---- 4.7 frozen columns ----

#[tokio::test]
async fn transition_the_number_of_an_approved_version_is_frozen() {
    let (mut conn, draft) = approved().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET number = 2 WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn transition_the_recipe_of_an_approved_version_is_frozen() {
    let (mut conn, draft) = approved().await;
    let other = fixtures::recipe(&mut conn, draft.machine, TWIN_CAT, draft.user).await;

    let result = sqlx::query("UPDATE recipe_version SET recipe_id = $2 WHERE id = $1")
        .bind(draft.version)
        .bind(other)
        .execute(&mut conn)
        .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn transition_the_base_version_of_an_approved_version_is_frozen() {
    let (mut conn, draft) = approved().await;

    let result = sqlx::query("UPDATE recipe_version SET based_on_version_id = $1 WHERE id = $1")
        .bind(draft.version)
        .execute(&mut conn)
        .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn transition_the_content_hash_of_an_approved_version_is_frozen() {
    let (mut conn, draft) = approved().await;

    let result = run(
        &mut conn,
        "UPDATE recipe_version SET content_sha256 = repeat('b', 64) WHERE id = $1",
        draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

// ---- 3.0 set_updated_at on the five tables ----

/// Run an update with `$1` as the row id. The statement returns `updated_at = now()` and
/// only matches a row that an earlier transaction wrote.
async fn updated_at_is_now(conn: &mut PgConnection, statement: &'static str, id: Uuid) -> bool {
    sqlx::query_scalar(statement)
        .bind(id)
        .fetch_one(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn set_updated_at_machine() {
    let (mut conn, draft) = plant().await;

    let is_now = updated_at_is_now(
        &mut conn,
        "UPDATE machine SET name = 'renamed' WHERE id = $1 AND updated_at < now() \
         RETURNING updated_at = now()",
        draft.machine,
    )
    .await;

    assert!(is_now);
}

#[tokio::test]
async fn set_updated_at_unit() {
    let (mut conn, draft) = plant().await;

    let is_now = updated_at_is_now(
        &mut conn,
        "UPDATE unit SET name = 'renamed' WHERE id = $1 AND updated_at < now() \
         RETURNING updated_at = now()",
        draft.unit,
    )
    .await;

    assert!(is_now);
}

#[tokio::test]
async fn set_updated_at_recipe() {
    let (mut conn, draft) = plant().await;

    let is_now = updated_at_is_now(
        &mut conn,
        "UPDATE recipe SET name = 'renamed' WHERE id = $1 AND updated_at < now() \
         RETURNING updated_at = now()",
        draft.recipe,
    )
    .await;

    assert!(is_now);
}

#[tokio::test]
async fn set_updated_at_app_user() {
    let (mut conn, draft) = plant().await;

    let is_now = updated_at_is_now(
        &mut conn,
        "UPDATE app_user SET must_change_password = true WHERE id = $1 AND updated_at < now() \
         RETURNING updated_at = now()",
        draft.user,
    )
    .await;

    assert!(is_now);
}

#[tokio::test]
async fn set_updated_at_app_setting() {
    let (mut conn, _draft) = plant().await;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO app_setting (key, value) VALUES ('audit.example', '1') RETURNING id",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();

    let is_now = updated_at_is_now(
        &mut conn,
        "UPDATE app_setting SET value = '2' WHERE id = $1 AND updated_at < now() \
         RETURNING updated_at = now()",
        id,
    )
    .await;

    assert!(is_now);
}
