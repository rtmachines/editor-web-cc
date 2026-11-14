//! Tests of section 4.14, `recipe_version_insert_draft` (I4, ADR-0169, migration 0016).
//!
//! The tests run as the table owner `recipe_migrator`. The trigger fires for every role.

use sqlx::PgConnection;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use crate::support::fixtures::{self, Draft};
use crate::support::{Role, TestDatabase, assert_trigger, document_sql, migration_sql};

const NEW_VERSION: &str = "E-RCP-037";
const MIGRATION: &str = "0016_recipe_version_insert.sql";

async fn plant() -> (PgConnection, Draft) {
    let mut conn = TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await;
    let draft = Draft::create(&mut conn).await;
    // The partial index of the live draft allows one live draft, so the plant discards the first.
    sqlx::query("UPDATE recipe_version SET deleted_at = now() WHERE id = $1")
        .bind(draft.version)
        .execute(&mut conn)
        .await
        .unwrap();
    (conn, draft)
}

/// Insert a second version of the recipe. The test names the columns that it wants to set.
async fn insert(
    conn: &mut PgConnection,
    draft: &Draft,
    columns: &'static str,
    values: &'static str,
) -> Result<PgQueryResult, sqlx::Error> {
    let statement = format!(
        "INSERT INTO recipe_version (recipe_id, plc_platform, number, change_note, created_by{columns}) \
         VALUES ($1, 'TwinCat3', 2, 'note', $2{values})"
    );
    sqlx::query(&statement)
        .bind(draft.recipe)
        .bind(draft.user)
        .execute(conn)
        .await
}

async fn count_versions(conn: &mut PgConnection, recipe: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM recipe_version WHERE recipe_id = $1")
        .bind(recipe)
        .fetch_one(conn)
        .await
        .unwrap()
}

#[test]
fn migration_0016_equals_the_sql_block_of_section_4_14() {
    let document = document_sql(&["4.14"], &[]);
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("CREATE FUNCTION recipe_version_insert_draft()"));
    assert_eq!(file, document);
}

#[tokio::test]
async fn recipe_version_insert_of_a_live_draft_is_accepted() {
    let (mut conn, draft) = plant().await;

    insert(&mut conn, &draft, "", "").await.unwrap();

    let state: (String, i64, bool) = sqlx::query_as(
        "SELECT status, row_version, deleted_at IS NULL FROM recipe_version \
         WHERE recipe_id = $1 AND number = 2",
    )
    .bind(draft.recipe)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(state, ("draft".to_owned(), 1, true));
}

#[tokio::test]
async fn recipe_version_insert_of_approved_row_is_rejected() {
    let (mut conn, draft) = plant().await;
    let before = count_versions(&mut conn, draft.recipe).await;

    let result = insert(
        &mut conn,
        &draft,
        ", status, approved_by, approved_at, content_sha256",
        ", 'approved', $2, now(), repeat('b', 64)",
    )
    .await;

    assert_trigger(result, NEW_VERSION);
    assert_eq!(count_versions(&mut conn, draft.recipe).await, before);
}

#[tokio::test]
async fn recipe_version_insert_of_retired_row_is_rejected() {
    let (mut conn, draft) = plant().await;

    let result = insert(
        &mut conn,
        &draft,
        ", status, approved_by, approved_at, content_sha256, retired_at",
        ", 'retired', $2, now(), repeat('b', 64), now()",
    )
    .await;

    assert_trigger(result, NEW_VERSION);
}

#[tokio::test]
async fn recipe_version_insert_with_a_row_version_other_than_1_is_rejected() {
    let (mut conn, draft) = plant().await;

    let result = insert(&mut conn, &draft, ", row_version", ", 2").await;

    assert_trigger(result, NEW_VERSION);
}

#[tokio::test]
async fn recipe_version_insert_with_a_discard_time_is_rejected() {
    let (mut conn, draft) = plant().await;

    let result = insert(&mut conn, &draft, ", deleted_at", ", now()").await;

    assert_trigger(result, NEW_VERSION);
}

#[tokio::test]
async fn recipe_version_insert_of_an_unknown_status_gives_the_trigger_code() {
    let (mut conn, draft) = plant().await;

    let result = insert(&mut conn, &draft, ", status", ", 'pending'").await;

    assert_trigger(result, NEW_VERSION);
}

#[tokio::test]
async fn recipe_version_insert_trigger_leaves_a_draft_to_the_approval_transition() {
    let (mut conn, draft) = plant().await;
    insert(&mut conn, &draft, "", "").await.unwrap();
    let version: Uuid =
        sqlx::query_scalar("SELECT id FROM recipe_version WHERE recipe_id = $1 AND number = 2")
            .bind(draft.recipe)
            .fetch_one(&mut conn)
            .await
            .unwrap();

    fixtures::approve(&mut conn, version, draft.user).await;

    let status: String = sqlx::query_scalar("SELECT status FROM recipe_version WHERE id = $1")
        .bind(version)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(status, "approved");
}
