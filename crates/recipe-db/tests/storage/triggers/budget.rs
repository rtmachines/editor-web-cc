//! Tests of the sections 4.10 and 4.12: `version_size_budget` and `security_event_purge`.
//!
//! The version budget tests run as the table owner `recipe_migrator`. The purge tests insert their
//! backdated events as the superuser, because `recipe_app` cannot name `occurred_at` after `0005`
//! (ADR-0035). They call the purge as `recipe_app`. A bulk insert is one `INSERT ... SELECT`,
//! so a test stays fast. The rows obey every other trigger: the parameters sit in a live draft of
//! a machine with the right platform.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{Draft, TWIN_CAT};
use crate::support::{Role, TestDatabase, assert_trigger};

const MAX_PARAMETERS: i32 = 4000;
const PURGE_BATCH: i64 = 5000;

async fn plant() -> (PgConnection, Draft) {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let draft = Draft::create(&mut conn).await;
    (conn, draft)
}

/// Insert the parameters `P<from>` to `P<to>` into a version with one statement.
async fn bulk_parameters(
    conn: &mut PgConnection,
    draft: &Draft,
    version: Uuid,
    from: i32,
    to: i32,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, value) \
         SELECT $1, $2, $3, 'P' || g, 'name', 'description', $4, '0'::jsonb \
         FROM generate_series($5::int, $6::int) AS g",
    )
    .bind(version)
    .bind(TWIN_CAT)
    .bind(draft.unit)
    .bind(draft.dint)
    .bind(from)
    .bind(to)
    .execute(conn)
    .await
}

/// Insert the limit rows `[from]` to `[to]` for the parameters `P<first>` to `P<last>`.
async fn bulk_limits(
    conn: &mut PgConnection,
    version: Uuid,
    parameters: (i32, i32),
    leaves: (i32, i32),
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO parameter_limit \
             (parameter_id, recipe_version_id, leaf_path, min_value, max_value) \
         SELECT p.id, p.recipe_version_id, '[' || g || ']', 0, 10 \
         FROM parameter AS p CROSS JOIN generate_series($4::int, $5::int) AS g \
         WHERE p.recipe_version_id = $1 \
           AND substr(p.tag, 2)::int BETWEEN $2 AND $3",
    )
    .bind(version)
    .bind(parameters.0)
    .bind(parameters.1)
    .bind(leaves.0)
    .bind(leaves.1)
    .execute(conn)
    .await
}

// ---- E-RCP-035: parameters per version (4.10) ----

#[tokio::test]
async fn e_rcp_035_a_version_accepts_4000_parameters() {
    let (mut conn, draft) = plant().await;

    let result = bulk_parameters(&mut conn, &draft, draft.version, 1, MAX_PARAMETERS)
        .await
        .unwrap();

    assert_eq!(result.rows_affected(), 4000);
}

#[tokio::test]
async fn e_rcp_035_parameter_4001_of_a_version_is_rejected() {
    let (mut conn, draft) = plant().await;
    bulk_parameters(&mut conn, &draft, draft.version, 1, MAX_PARAMETERS)
        .await
        .unwrap();

    let result = bulk_parameters(&mut conn, &draft, draft.version, 4001, 4001).await;

    assert_trigger(result, "E-RCP-035");
}

#[tokio::test]
async fn e_rcp_035_one_statement_of_4001_rows_is_rejected() {
    let (mut conn, draft) = plant().await;

    let result = bulk_parameters(&mut conn, &draft, draft.version, 1, MAX_PARAMETERS + 1).await;

    assert_trigger(result, "E-RCP-035");
}

#[tokio::test]
async fn e_rcp_035_the_budget_counts_each_version_on_its_own() {
    let (mut conn, draft) = plant().await;
    let other = draft.other_draft(&mut conn).await;
    bulk_parameters(&mut conn, &draft, draft.version, 1, MAX_PARAMETERS)
        .await
        .unwrap();

    let result = bulk_parameters(&mut conn, &draft, other, 1, 1)
        .await
        .unwrap();

    assert_eq!(result.rows_affected(), 1);
}

// ---- E-RCP-036: limit rows per version (4.10) ----

#[tokio::test]
async fn e_rcp_036_a_version_accepts_32768_limit_rows() {
    let (mut conn, draft) = plant().await;
    bulk_parameters(&mut conn, &draft, draft.version, 1, 3)
        .await
        .unwrap();

    let result = bulk_limits(&mut conn, draft.version, (1, 2), (1, 16_384))
        .await
        .unwrap();

    assert_eq!(result.rows_affected(), 32_768);
}

#[tokio::test]
async fn e_rcp_036_limit_row_32769_of_a_version_is_rejected() {
    let (mut conn, draft) = plant().await;
    bulk_parameters(&mut conn, &draft, draft.version, 1, 3)
        .await
        .unwrap();
    bulk_limits(&mut conn, draft.version, (1, 2), (1, 16_384))
        .await
        .unwrap();

    let result = bulk_limits(&mut conn, draft.version, (3, 3), (1, 1)).await;

    assert_trigger(result, "E-RCP-036");
}

#[tokio::test]
async fn e_rcp_036_one_statement_of_32769_rows_is_rejected() {
    let (mut conn, draft) = plant().await;
    bulk_parameters(&mut conn, &draft, draft.version, 1, 3)
        .await
        .unwrap();

    let result = bulk_limits(&mut conn, draft.version, (1, 3), (1, 10_923)).await;

    assert_trigger(result, "E-RCP-036");
}

// ---- security_event_purge (4.12) ----

/// Insert `count` events with ids in ascending order. They are `age_days` old.
async fn insert_events(conn: &mut PgConnection, count: i64, age_days: i32) {
    sqlx::query(
        "INSERT INTO security_event (occurred_at, kind, login_hmac, client_ip, request_id) \
         SELECT now() - make_interval(days => $2::int), 'forbidden', '\\x00'::bytea, \
                '10.0.0.1'::inet, gen_random_uuid() \
         FROM generate_series(1, $1::bigint)",
    )
    .bind(count)
    .bind(age_days)
    .execute(conn)
    .await
    .unwrap();
}

async fn purge(conn: &mut PgConnection) -> i64 {
    sqlx::query_scalar("SELECT security_event_purge()")
        .fetch_one(conn)
        .await
        .unwrap()
}

async fn remaining_ages(conn: &mut PgConnection) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT (now()::date - occurred_at::date)::int FROM security_event ORDER BY id",
    )
    .fetch_all(conn)
    .await
    .unwrap()
}

#[tokio::test]
async fn purge_deletes_5000_rows_per_call_and_returns_the_count() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;
    insert_events(&mut admin, PURGE_BATCH + 1, 401).await;
    insert_events(&mut admin, 1, 1).await;

    let counts = [
        purge(&mut conn).await,
        purge(&mut conn).await,
        purge(&mut conn).await,
    ];

    assert_eq!(counts, [PURGE_BATCH, 1, 0]);
    assert_eq!(remaining_ages(&mut conn).await, [1]);
}

#[tokio::test]
async fn purge_keeps_a_row_that_is_younger_than_400_days() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;
    insert_events(&mut admin, 1, 399).await;
    insert_events(&mut admin, 1, 401).await;

    let deleted = purge(&mut conn).await;

    assert_eq!((deleted, remaining_ages(&mut conn).await), (1, vec![399]));
}

#[tokio::test]
async fn purge_deletes_the_oldest_rows_by_id_first() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;
    insert_events(&mut admin, PURGE_BATCH + 1, 401).await;
    let last_id: i64 = sqlx::query_scalar("SELECT max(id) FROM security_event")
        .fetch_one(&mut conn)
        .await
        .unwrap();

    purge(&mut conn).await;

    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM security_event")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(ids, [last_id]);
}
