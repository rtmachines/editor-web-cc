//! Tests of the migration `0006_operations.sql` (`data-model.md` section 5.5, ADR-0069).
//!
//! The migration creates the index `security_event_occurred_at_idx` and the function
//! `replication_state()` as `audit_owner`. Only `recipe_app` can run the function. The index
//! serves the purge of section 4.12 and the two alert sums of `reliability.md` section 3.7.

use std::process::Command;
use std::time::Duration;

use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgConnection;

use crate::support::{Role, TestDatabase, document_sql, migration_sql, server};

const MIGRATION: &str = "0006_operations.sql";
const INDEX: &str = "security_event_occurred_at_idx";
const FUNCTION: &str = "replication_state()";
/// The number of rows of the plan test (ADR-0069 (l)).
const PLAN_ROWS: i64 = 50_000;
/// Rows of four kinds, one row each 5 minutes, all older than 1 hour and younger than 400 days.
/// Each row has its own minute, so the aggregate index of section 3.8 accepts every row.
const FILL_ROWS: &str = r"INSERT INTO security_event (occurred_at, kind, login_hmac, client_ip, request_id)
    SELECT now() - INTERVAL '1 hour' - g * INTERVAL '5 minutes',
           (ARRAY['login_failed', 'forbidden', 'csrf_rejected', 'unlock'])[g % 4 + 1],
           '\x00'::bytea, '10.0.0.1'::inet, gen_random_uuid()
    FROM generate_series(1, $1::bigint) AS g";
/// The sum of failed logins of `reliability.md` 3.7 (`alerts.failed_login_window` 15 min).
const FAILED_LOGIN_SUM: &str = "SELECT COALESCE(sum(count), 0) FROM security_event \
     WHERE kind = 'login_failed' AND occurred_at >= now() - INTERVAL '15 minutes'";
/// A stand-in for the security-window sum of `reliability.md` 3.7 (`alerts.security_window`
/// 15 min). Task 4.5 writes the real statement. This one filters on the kinds `csrf_rejected`
/// and `forbidden` only. It omits the condition "a `forbidden` on `UserManage`". The `EXPLAIN`
/// result does not depend on that condition.
const SECURITY_WINDOW_SUM: &str = "SELECT COALESCE(sum(count), 0) FROM security_event \
     WHERE kind IN ('csrf_rejected', 'forbidden') AND occurred_at >= now() - INTERVAL '15 minutes'";
/// The program of the streaming test: `pg_receivewal` as the service `walarchive`, at most 60 s.
const RECEIVER_PROGRAM: &str = "mkdir -p /tmp/walarchive-test && exec timeout 60 pg_receivewal \
     --no-password -D /tmp/walarchive-test \
     -d 'host=127.0.0.1 user=recipe_replicator application_name=walarchive'";
const STREAM_ATTEMPTS: u32 = 120;
const STREAM_PAUSE: Duration = Duration::from_millis(500);

/// The `DELETE` statement of the function `security_event_purge()` of section 4.12.
fn purge_statement() -> String {
    let function = document_sql(&["4.12"], &[]);
    let start = function.find("DELETE FROM").expect("4.12 holds a DELETE");
    let rest = function.get(start..).unwrap();
    let end = rest.find(';').expect("the DELETE ends with a semicolon");
    rest.get(..end).unwrap().to_owned()
}

/// Every value of the key "Index Name" in a plan of `EXPLAIN (FORMAT JSON)`.
fn index_names(node: &Value, names: &mut Vec<String>) {
    match node {
        Value::Object(map) => {
            if let Some(Value::String(name)) = map.get("Index Name") {
                names.push(name.clone());
            }
            map.values().for_each(|value| index_names(value, names));
        }
        Value::Array(items) => items.iter().for_each(|item| index_names(item, names)),
        _ => {}
    }
}

/// The index names of the plan of one statement.
async fn plan_indexes(conn: &mut PgConnection, statement: &str) -> Vec<String> {
    let plan: Value = sqlx::query_scalar(&format!("EXPLAIN (FORMAT JSON) {statement}"))
        .fetch_one(conn)
        .await
        .unwrap();
    let mut names = Vec::new();
    index_names(&plan, &mut names);
    names
}

/// The row of `replication_state()` as `recipe_app`.
async fn replication_state(app: &mut PgConnection) -> (bool, Option<Decimal>) {
    sqlx::query_as("SELECT connected, lag_bytes FROM replication_state()")
        .fetch_one(app)
        .await
        .unwrap()
}

#[test]
fn migration_0006_equals_the_sql_block_of_section_5_5() {
    let document = document_sql(&["5.5"], &[]);
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("GRANT CREATE ON SCHEMA public TO audit_owner;"));
    assert!(document.ends_with("REVOKE CREATE ON SCHEMA public FROM audit_owner;"));
    assert_eq!(file, document);
}

#[tokio::test]
async fn the_index_on_occurred_at_exists_and_belongs_to_audit_owner() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    let row: (String, String) = sqlx::query_as(
        "SELECT pg_get_userbyid(c.relowner)::text, pg_get_indexdef(c.oid) FROM pg_class c \
         WHERE c.oid = to_regclass($1)",
    )
    .bind(format!("public.{INDEX}"))
    .fetch_one(&mut admin)
    .await
    .unwrap();

    assert_eq!(row.0, "audit_owner");
    assert!(
        row.1
            .ends_with("ON public.security_event USING btree (occurred_at)"),
        "{}",
        row.1
    );
}

#[tokio::test]
async fn the_function_is_a_definer_of_audit_owner_with_a_fixed_search_path() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    let row: (String, bool, Vec<String>) = sqlx::query_as(
        "SELECT pg_get_userbyid(proowner)::text, prosecdef, proconfig FROM pg_proc \
         WHERE oid = to_regprocedure($1)",
    )
    .bind(FUNCTION)
    .fetch_one(&mut admin)
    .await
    .unwrap();

    let expected = vec!["search_path=pg_catalog, public, pg_temp".to_owned()];
    assert_eq!(row, ("audit_owner".to_owned(), true, expected));
}

#[tokio::test]
async fn only_recipe_app_can_execute_replication_state() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    for (role, expected) in [
        ("public", false),
        ("recipe_backup", false),
        ("recipe_migrator", false),
        ("recipe_replicator", false),
        ("recipe_app", true),
    ] {
        let allowed: bool = sqlx::query_scalar("SELECT has_function_privilege($1, $2, 'EXECUTE')")
            .bind(role)
            .bind(FUNCTION)
            .fetch_one(&mut admin)
            .await
            .unwrap();

        assert_eq!(allowed, expected, "EXECUTE of {role} on {FUNCTION}");
    }
}

/// Start `pg_receivewal` in the container as the service `walarchive` would. It stops by itself.
fn start_receiver() {
    let password = format!("PGPASSWORD={}", Role::Replicator.password());
    let output = Command::new("docker")
        .args(["exec", "-d", "-e", &password, &server().container_id])
        .args(["bash", "-c", RECEIVER_PROGRAM])
        .output()
        .expect("docker exec runs");
    assert!(output.status.success(), "{output:?}");
}

#[tokio::test]
async fn replication_state_gives_one_row_before_and_during_the_stream_of_walarchive() {
    // The only test that starts a replication connection, so no other test changes the row.
    let database = TestDatabase::create().await;
    let mut app = database.connect(Role::App).await;
    assert_eq!(replication_state(&mut app).await, (false, None));

    start_receiver();
    let mut state = (false, None);
    for _ in 0..STREAM_ATTEMPTS {
        state = replication_state(&mut app).await;
        // At start-up the row of walarchive can exist before sent_lsn has a value.
        if state.0 && state.1.is_some() {
            break;
        }
        tokio::time::sleep(STREAM_PAUSE).await;
    }

    // A lag value proves that the definer reads sent_lsn through pg_read_all_stats.
    assert!(state.0, "walarchive does not stream");
    let lag = state.1.expect("the lag is known while walarchive streams");
    assert!(lag >= Decimal::ZERO, "{lag}");
}

#[tokio::test]
async fn the_purge_and_the_two_alert_sums_use_the_index_on_50_000_rows() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;
    sqlx::query(FILL_ROWS)
        .bind(PLAN_ROWS)
        .execute(&mut admin)
        .await
        .unwrap();
    sqlx::query("ANALYZE security_event")
        .execute(&mut admin)
        .await
        .unwrap();

    for statement in [
        purge_statement(),
        FAILED_LOGIN_SUM.to_owned(),
        SECURITY_WINDOW_SUM.to_owned(),
    ] {
        let indexes = plan_indexes(&mut admin, &statement).await;

        assert!(
            indexes.iter().any(|name| name == INDEX),
            "{statement} uses {indexes:?}"
        );
    }
}
