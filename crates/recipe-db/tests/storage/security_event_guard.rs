//! Tests of the guard of `security_event` (`0005_security_event_guard.sql`, `data-model.md`
//! section 5.4, ADR-0035, design D15).
//!
//! `recipe_app` inserts only the columns without a database default. The trigger
//! `security_event_guard` lets an update pass only when `count` grows by 1 and no other column
//! changes. The tests of the trigger use `assert_trigger`: SQLSTATE `23514` and the code `E-AUD-003`
//! at the start of the message. The map from this message to `StorageError` has its own unit test
//! in `src/error/tests.rs`.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::{
    Role, TestDatabase, assert_privilege_denied, assert_trigger, created_names, document_sql,
    missing_names, wait_for_early_second,
};

const GUARD_FUNCTION: &str = "security_event_guard()";
const GUARD_TRIGGER: &str = "security_event_guard";

/// The aggregation statement of the sink: no `occurred_at` and no `count` in the column list.
const AGGREGATE_LOGIN_FAILED: &str = "\
INSERT INTO security_event (kind, login_hmac, client_ip, request_id) \
VALUES ('login_failed', $1, '10.0.0.9'::inet, $2) \
ON CONFLICT (client_ip, kind, (date_trunc('minute', occurred_at AT TIME ZONE 'UTC'))) \
    WHERE kind IN ('login_failed', 'login_throttled', 'csrf_rejected') \
DO UPDATE SET count = security_event.count + 1";

/// Insert one `login_failed` row as `recipe_app`, the way the sink does. Return its id.
///
/// The kind aggregates, so the check `security_event_count_aggregated` allows a count above 1.
async fn insert_login_failed(app: &mut PgConnection) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO security_event (kind, login_hmac, client_ip, request_id) \
         VALUES ('login_failed', '\\x01'::bytea, '10.0.0.1'::inet, $1) RETURNING id",
    )
    .bind(Uuid::new_v4())
    .fetch_one(app)
    .await
    .unwrap()
}

/// Read the `count` of a row as the superuser.
async fn count_of(admin: &mut PgConnection, id: i64) -> i32 {
    sqlx::query_scalar("SELECT count FROM security_event WHERE id = $1")
        .bind(id)
        .fetch_one(admin)
        .await
        .unwrap()
}

#[tokio::test]
async fn security_event_guard_an_insert_that_names_occurred_at_gives_42501() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;

    let result = sqlx::query(
        "INSERT INTO security_event (kind, login_hmac, client_ip, request_id, occurred_at) \
         VALUES ('forbidden', '\\x01'::bytea, '10.0.0.1'::inet, $1, now() - interval '500 days')",
    )
    .bind(Uuid::new_v4())
    .execute(&mut app)
    .await;

    assert_privilege_denied(result);
}

#[tokio::test]
async fn security_event_guard_an_insert_that_names_count_gives_42501() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;

    let result = sqlx::query(
        "INSERT INTO security_event (kind, login_hmac, client_ip, request_id, count) \
         VALUES ('login_failed', '\\x01'::bytea, '10.0.0.1'::inet, $1, 5)",
    )
    .bind(Uuid::new_v4())
    .execute(&mut app)
    .await;

    assert_privilege_denied(result);
}

#[tokio::test]
async fn security_event_guard_an_insert_without_the_two_columns_gets_the_defaults() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;

    let id = insert_login_failed(&mut app).await;

    let (count, age_seconds): (i32, f64) = sqlx::query_as(
        "SELECT count, extract(epoch FROM now() - occurred_at)::float8 \
         FROM security_event WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&mut admin)
    .await
    .unwrap();
    assert_eq!(count, 1);
    assert!((0.0..60.0).contains(&age_seconds), "{age_seconds}");
}

#[tokio::test]
async fn security_event_guard_the_upsert_of_an_aggregated_kind_passes() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;
    // The three statements must share one minute.
    wait_for_early_second(&mut admin).await;

    for _ in 0..3 {
        sqlx::query(AGGREGATE_LOGIN_FAILED)
            .bind(vec![1_u8; 32])
            .bind(Uuid::new_v4())
            .execute(&mut app)
            .await
            .unwrap();
    }

    let rows: Vec<i32> = sqlx::query_scalar("SELECT count FROM security_event")
        .fetch_all(&mut admin)
        .await
        .unwrap();
    assert_eq!(rows, vec![3]);
}

#[tokio::test]
async fn security_event_guard_count_plus_1_passes_as_recipe_app() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;
    let id = insert_login_failed(&mut app).await;

    sqlx::query("UPDATE security_event SET count = count + 1 WHERE id = $1")
        .bind(id)
        .execute(&mut app)
        .await
        .unwrap();

    assert_eq!(count_of(&mut admin, id).await, 2);
}

#[tokio::test]
async fn security_event_guard_count_1_gives_e_aud_003() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let id = insert_login_failed(&mut app).await;

    let result = sqlx::query("UPDATE security_event SET count = 1 WHERE id = $1")
        .bind(id)
        .execute(&mut app)
        .await;

    assert_trigger(result, "E-AUD-003");
}

#[tokio::test]
async fn security_event_guard_count_plus_2_gives_e_aud_003() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;
    let id = insert_login_failed(&mut app).await;

    let result = sqlx::query("UPDATE security_event SET count = count + 2 WHERE id = $1")
        .bind(id)
        .execute(&mut app)
        .await;

    assert_trigger(result, "E-AUD-003");
    assert_eq!(count_of(&mut admin, id).await, 1);
}

#[tokio::test]
async fn security_event_guard_a_change_of_detail_by_the_superuser_gives_e_aud_003() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;
    let id = insert_login_failed(&mut app).await;

    let result =
        sqlx::query("UPDATE security_event SET detail = '{\"x\": 1}'::jsonb WHERE id = $1")
            .bind(id)
            .execute(&mut admin)
            .await;

    assert_trigger(result, "E-AUD-003");
}

#[tokio::test]
async fn security_event_guard_count_plus_1_with_another_column_gives_e_aud_003() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;
    let id = insert_login_failed(&mut app).await;

    let result = sqlx::query(
        "UPDATE security_event SET count = count + 1, user_agent = 'changed' WHERE id = $1",
    )
    .bind(id)
    .execute(&mut admin)
    .await;

    assert_trigger(result, "E-AUD-003");
}

#[tokio::test]
async fn security_event_guard_a_change_of_occurred_at_by_recipe_app_gives_42501() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let id = insert_login_failed(&mut app).await;

    let result = sqlx::query("UPDATE security_event SET occurred_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut app)
        .await;

    assert_privilege_denied(result);
}

#[tokio::test]
async fn security_event_guard_the_migrator_cannot_drop_the_function() {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;

    let result = sqlx::query("DROP FUNCTION security_event_guard() CASCADE")
        .execute(&mut migrator)
        .await;

    assert_privilege_denied(result);
}

#[tokio::test]
async fn security_event_guard_the_migrator_cannot_drop_the_trigger() {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;

    let result = sqlx::query("DROP TRIGGER security_event_guard ON security_event")
        .execute(&mut migrator)
        .await;

    assert_privilege_denied(result);
}

#[tokio::test]
async fn security_event_guard_the_event_trigger_stops_a_drop_by_a_role_other_than_audit_owner() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;
    let mut migrator = database.connect(Role::Migrator).await;
    // The migrator owns the schema `public`. Make it the owner of the function too, so that
    // only the event trigger of ADR-0034 can stop the drop.
    sqlx::query("ALTER FUNCTION security_event_guard() OWNER TO recipe_migrator")
        .execute(&mut admin)
        .await
        .unwrap();

    let result = sqlx::query("DROP FUNCTION security_event_guard() CASCADE")
        .execute(&mut migrator)
        .await;

    assert_privilege_denied(result);
}

#[tokio::test]
async fn security_event_guard_the_role_setting_of_recipe_app_is_terse() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;

    let verbosity: String = sqlx::query_scalar("SHOW log_error_verbosity")
        .fetch_one(&mut app)
        .await
        .unwrap();
    let stored: Vec<String> = sqlx::query_scalar(
        "SELECT unnest(setconfig)::text FROM pg_db_role_setting \
         WHERE setrole = 'recipe_app'::regrole AND setdatabase = 0",
    )
    .fetch_all(&mut admin)
    .await
    .unwrap();

    assert_eq!(verbosity, "terse");
    assert!(
        stored.contains(&"log_error_verbosity=terse".to_owned()),
        "{stored:?}"
    );
}

#[tokio::test]
async fn security_event_guard_the_role_setting_of_recipe_app_logs_no_bind_value() {
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    let mut admin = database.connect(Role::Superuser).await;

    let length: String = sqlx::query_scalar("SHOW log_parameter_max_length")
        .fetch_one(&mut app)
        .await
        .unwrap();
    let stored: Vec<String> = sqlx::query_scalar(
        "SELECT unnest(setconfig)::text FROM pg_db_role_setting \
         WHERE setrole = 'recipe_app'::regrole AND setdatabase = 0",
    )
    .fetch_all(&mut admin)
    .await
    .unwrap();

    assert_eq!(length, "0");
    assert!(
        stored.contains(&"log_parameter_max_length=0".to_owned()),
        "{stored:?}"
    );
}

#[tokio::test]
async fn security_event_guard_the_function_belongs_to_audit_owner_without_public_execute() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    let owner: String = sqlx::query_scalar(
        "SELECT pg_get_userbyid(proowner)::text FROM pg_proc WHERE oid = to_regprocedure($1)",
    )
    .bind(GUARD_FUNCTION)
    .fetch_one(&mut admin)
    .await
    .unwrap();
    let app_can_execute: bool = sqlx::query_scalar("SELECT has_function_privilege($1, $2, $3)")
        .bind("recipe_app")
        .bind(GUARD_FUNCTION)
        .bind("EXECUTE")
        .fetch_one(&mut admin)
        .await
        .unwrap();

    assert_eq!(owner, "audit_owner");
    assert!(!app_can_execute);
}

#[tokio::test]
async fn security_event_guard_every_name_of_section_5_4_exists_in_the_catalog() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let names = created_names(&document_sql(&["5.4"], &[]));

    let missing = missing_names(&mut conn, &names).await;

    assert_eq!(names.functions, vec!["security_event_guard".to_owned()]);
    assert_eq!(names.triggers, vec![GUARD_TRIGGER.to_owned()]);
    assert!(missing.is_empty(), "missing: {missing:?}");
}
