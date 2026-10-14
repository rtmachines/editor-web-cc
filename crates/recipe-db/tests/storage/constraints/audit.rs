//! Tests of `audit_log`, `audit_chain_head`, and `app_setting` (`data-model.md` sections 3.15 to
//! 3.17, task 7.6).
//!
//! `audit_log` and `audit_chain_head` belong to `audit_owner`, so their tests run as the superuser
//! of the harness. The hash chain trigger 4.9 sets `chain_seq`, `prev_hash`, and `entry_hash` on
//! insert, so the tests that break a chain constraint change the row with an `UPDATE`, which the
//! trigger does not handle. `app_setting` belongs to `recipe_migrator`.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::{
    CHECK_VIOLATION, FOREIGN_KEY_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint,
};

/// The constraints and unique indexes that the tests of this module violate.
pub const COVERED: &[&str] = &[
    "audit_log_pkey",
    "audit_log_actor_user_id_fkey",
    "app_setting_updated_by_fkey",
    "audit_log_action_format",
    "audit_log_entity_type_format",
    "audit_log_aggregate_type_value",
    "audit_log_has_state",
    "audit_log_session_ref_format",
    "audit_log_chain_seq_unique",
    "audit_log_hash_length",
    "audit_chain_head_pkey",
    "audit_chain_head_one_row",
    "audit_chain_head_seq_nonnegative",
    "audit_chain_head_hash_length",
    "app_setting_pkey",
    "app_setting_key_unique",
    "app_setting_key_format",
    "app_setting_value_length",
];

/// Return a migrated database connection as the superuser of the harness.
async fn superuser() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Superuser).await
}

/// Return a migrated database connection as the owner of `app_setting`.
async fn migrator() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Migrator).await
}

// ---- audit_log ----

/// The columns of one `audit_log` row that a test sets. The hash chain columns come from the
/// trigger.
#[derive(Debug, Clone)]
struct EntryRow {
    action: &'static str,
    entity_type: &'static str,
    aggregate_type: &'static str,
    before_state: Option<&'static str>,
    after_state: Option<&'static str>,
    session_ref: Option<&'static str>,
}

impl EntryRow {
    /// A `CreateRecipe` entry of a recipe with an after state and a session reference.
    fn valid() -> Self {
        Self {
            action: "CreateRecipe",
            entity_type: "recipe",
            aggregate_type: "recipe",
            before_state: None,
            after_state: Some(r#"{"name":"Recipe"}"#),
            session_ref: Some("0123456789abcdef"),
        }
    }
}

/// Insert an audit entry and return its id.
async fn insert_entry(conn: &mut PgConnection, row: &EntryRow) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO audit_log \
             (action, entity_type, entity_id, aggregate_type, aggregate_id, before_state, \
              after_state, request_id, session_ref) \
         VALUES ($1, $2, $3, $4, $3, $5::text::jsonb, $6::text::jsonb, $3, $7) \
         RETURNING id",
    )
    .bind(row.action)
    .bind(row.entity_type)
    .bind(Uuid::nil())
    .bind(row.aggregate_type)
    .bind(row.before_state)
    .bind(row.after_state)
    .bind(row.session_ref)
    .fetch_one(conn)
    .await
}

#[tokio::test]
async fn audit_log_row_with_valid_values_inserts() {
    let mut conn = superuser().await;

    insert_entry(&mut conn, &EntryRow::valid()).await.unwrap();
}

#[tokio::test]
async fn audit_log_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = superuser().await;
    let id = insert_entry(&mut conn, &EntryRow::valid()).await.unwrap();

    let result = sqlx::query(
        "INSERT INTO audit_log \
             (id, action, entity_type, entity_id, aggregate_type, aggregate_id, after_state, \
              request_id) \
         OVERRIDING SYSTEM VALUE \
         VALUES ($1, 'CreateRecipe', 'recipe', $2, 'recipe', $2, '{\"a\":1}'::jsonb, $2)",
    )
    .bind(id)
    .bind(Uuid::nil())
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "audit_log_pkey");
}

#[tokio::test]
async fn audit_log_actor_user_id_fkey_rejects_an_unknown_actor() {
    let mut conn = superuser().await;

    let result = sqlx::query(
        "INSERT INTO audit_log \
             (actor_user_id, action, entity_type, entity_id, aggregate_type, aggregate_id, \
              after_state, request_id) \
         VALUES ($1, 'CreateRecipe', 'recipe', $1, 'recipe', $1, '{\"a\":1}'::jsonb, $1)",
    )
    .bind(Uuid::nil())
    .execute(&mut conn)
    .await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "audit_log_actor_user_id_fkey",
    );
}

#[tokio::test]
async fn audit_log_chain_seq_unique_rejects_a_repeated_sequence_number() {
    let mut conn = superuser().await;
    let first = insert_entry(&mut conn, &EntryRow::valid()).await.unwrap();
    let second = insert_entry(&mut conn, &EntryRow::valid()).await.unwrap();
    let first_seq: i64 = sqlx::query_scalar("SELECT chain_seq FROM audit_log WHERE id = $1")
        .bind(first)
        .fetch_one(&mut conn)
        .await
        .unwrap();

    let result = sqlx::query("UPDATE audit_log SET chain_seq = $1 WHERE id = $2")
        .bind(first_seq)
        .bind(second)
        .execute(&mut conn)
        .await;

    assert_constraint(result, UNIQUE_VIOLATION, "audit_log_chain_seq_unique");
}

#[tokio::test]
async fn audit_log_hash_length_rejects_an_entry_hash_that_is_not_32_bytes() {
    let mut conn = superuser().await;
    let id = insert_entry(&mut conn, &EntryRow::valid()).await.unwrap();

    let result = sqlx::query("UPDATE audit_log SET entry_hash = $1 WHERE id = $2")
        .bind(vec![1_u8])
        .bind(id)
        .execute(&mut conn)
        .await;

    assert_constraint(result, CHECK_VIOLATION, "audit_log_hash_length");
}

#[tokio::test]
async fn audit_log_action_format_rejects_an_action_that_starts_in_lowercase() {
    let mut conn = superuser().await;
    let row = EntryRow {
        action: "createRecipe",
        ..EntryRow::valid()
    };

    let result = insert_entry(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "audit_log_action_format");
}

#[tokio::test]
async fn audit_log_entity_type_format_rejects_an_entity_type_with_capitals() {
    let mut conn = superuser().await;
    let row = EntryRow {
        entity_type: "Recipe",
        ..EntryRow::valid()
    };

    let result = insert_entry(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "audit_log_entity_type_format");
}

#[tokio::test]
async fn audit_log_aggregate_type_value_rejects_an_unknown_aggregate() {
    let mut conn = superuser().await;
    let row = EntryRow {
        aggregate_type: "session",
        ..EntryRow::valid()
    };

    let result = insert_entry(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "audit_log_aggregate_type_value");
}

#[tokio::test]
async fn audit_log_has_state_rejects_an_entry_without_any_state() {
    let mut conn = superuser().await;
    let row = EntryRow {
        after_state: None,
        ..EntryRow::valid()
    };

    let result = insert_entry(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "audit_log_has_state");
}

#[tokio::test]
async fn audit_log_session_ref_format_rejects_a_session_reference_that_is_not_hex() {
    let mut conn = superuser().await;
    let row = EntryRow {
        session_ref: Some("XYZ"),
        ..EntryRow::valid()
    };

    let result = insert_entry(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "audit_log_session_ref_format");
}

// ---- audit_chain_head ----

#[tokio::test]
async fn audit_chain_head_holds_one_row_with_sequence_zero_after_the_migration() {
    let mut conn = superuser().await;

    let (rows, last_seq): (i64, i64) =
        sqlx::query_as("SELECT count(*), max(last_seq) FROM audit_chain_head")
            .fetch_one(&mut conn)
            .await
            .unwrap();

    assert_eq!((rows, last_seq), (1, 0));
}

#[tokio::test]
async fn audit_chain_head_pkey_rejects_a_second_row() {
    let mut conn = superuser().await;

    let result = sqlx::query(
        "INSERT INTO audit_chain_head (singleton, last_seq, last_hash) VALUES (true, 0, NULL)",
    )
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "audit_chain_head_pkey");
}

#[tokio::test]
async fn audit_chain_head_one_row_rejects_a_row_that_is_not_the_singleton() {
    let mut conn = superuser().await;

    let result = sqlx::query(
        "INSERT INTO audit_chain_head (singleton, last_seq, last_hash) VALUES (false, 0, NULL)",
    )
    .execute(&mut conn)
    .await;

    assert_constraint(result, CHECK_VIOLATION, "audit_chain_head_one_row");
}

#[tokio::test]
async fn audit_chain_head_seq_nonnegative_rejects_a_negative_sequence() {
    let mut conn = superuser().await;

    let result = sqlx::query("UPDATE audit_chain_head SET last_seq = -1 WHERE singleton")
        .execute(&mut conn)
        .await;

    assert_constraint(result, CHECK_VIOLATION, "audit_chain_head_seq_nonnegative");
}

#[tokio::test]
async fn audit_chain_head_hash_length_rejects_a_head_hash_that_is_not_32_bytes() {
    let mut conn = superuser().await;

    let result = sqlx::query("UPDATE audit_chain_head SET last_hash = $1 WHERE singleton")
        .bind(vec![1_u8])
        .execute(&mut conn)
        .await;

    assert_constraint(result, CHECK_VIOLATION, "audit_chain_head_hash_length");
}

// ---- app_setting ----

/// Insert a setting and return its id.
async fn insert_setting(
    conn: &mut PgConnection,
    key: &str,
    value: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO app_setting (key, value) VALUES ($1, $2) RETURNING id")
        .bind(key)
        .bind(value)
        .fetch_one(conn)
        .await
}

#[tokio::test]
async fn app_setting_row_with_valid_values_inserts() {
    let mut conn = migrator().await;

    insert_setting(&mut conn, "test.alpha", "1").await.unwrap();
}

#[tokio::test]
async fn app_setting_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = migrator().await;
    let id = insert_setting(&mut conn, "test.alpha", "1").await.unwrap();

    let result =
        sqlx::query("INSERT INTO app_setting (id, key, value) VALUES ($1, 'test.beta', '1')")
            .bind(id)
            .execute(&mut conn)
            .await;

    assert_constraint(result, UNIQUE_VIOLATION, "app_setting_pkey");
}

#[tokio::test]
async fn app_setting_updated_by_fkey_rejects_an_unknown_user() {
    let mut conn = migrator().await;

    let result = sqlx::query(
        "INSERT INTO app_setting (key, value, updated_by) VALUES ('test.alpha', '1', $1)",
    )
    .bind(Uuid::nil())
    .execute(&mut conn)
    .await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "app_setting_updated_by_fkey");
}

#[tokio::test]
async fn app_setting_key_unique_rejects_a_second_row_with_the_same_key() {
    let mut conn = migrator().await;
    insert_setting(&mut conn, "test.alpha", "1").await.unwrap();

    let result = insert_setting(&mut conn, "test.alpha", "2").await;

    assert_constraint(result, UNIQUE_VIOLATION, "app_setting_key_unique");
}

#[tokio::test]
async fn app_setting_key_format_rejects_a_key_with_capitals() {
    let mut conn = migrator().await;

    let result = insert_setting(&mut conn, "Test.Alpha", "1").await;

    assert_constraint(result, CHECK_VIOLATION, "app_setting_key_format");
}

#[tokio::test]
async fn app_setting_value_length_rejects_a_value_above_1000_characters() {
    let mut conn = migrator().await;
    let value = "x".repeat(1001);

    let result = insert_setting(&mut conn, "test.alpha", &value).await;

    assert_constraint(result, CHECK_VIOLATION, "app_setting_value_length");
}
