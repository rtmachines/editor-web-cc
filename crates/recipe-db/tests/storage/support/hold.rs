//! Helpers for a test that holds a pool connection in use (task 4.5).
//!
//! A test never sleeps a fixed time to prove that a connection is in use. It polls
//! `pg_stat_activity` with a bound. Every step that waits for a spawned task has an outer time
//! limit, so a hang fails the test and never blocks it.

use std::future::Future;
use std::time::{Duration, Instant};

use sqlx::PgConnection;

/// The outer time limit of one awaited step.
const OUTER_LIMIT: Duration = Duration::from_secs(30);
/// The pause between two polls of `pg_stat_activity`.
const POLL: Duration = Duration::from_millis(20);
/// The longest time that a poll waits for its condition.
const BOUND: Duration = Duration::from_secs(5);

/// Count the sessions of `recipe_app` in this database that wait for a lock.
const WAITING_APP_SESSIONS: &str = "SELECT count(*) FROM pg_stat_activity \
    WHERE datname = current_database() AND usename = 'recipe_app' \
      AND wait_event_type = 'Lock'";

/// The key of the one-key advisory lock of the snapshot gate. No production code uses it.
const SNAPSHOT_GATE_KEY: i64 = 7_240_300;

/// Replace `replication_state()` with a function that waits for the snapshot gate.
///
/// The function takes the session lock of the gate and releases it at once. The snapshot
/// statement calls the function before it reads any table, so a test holds the statement at a
/// known point after its data snapshot. The owner, the signature, `SECURITY DEFINER`, the
/// `search_path`, and the grants stay as in migration 0006.
pub fn plant_snapshot_gate() -> String {
    format!(
        "CREATE OR REPLACE FUNCTION replication_state(OUT connected BOOLEAN, OUT lag_bytes NUMERIC) \
         LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$ \
         BEGIN \
             PERFORM pg_catalog.pg_advisory_lock({SNAPSHOT_GATE_KEY}); \
             PERFORM pg_catalog.pg_advisory_unlock({SNAPSHOT_GATE_KEY}); \
             connected := false; \
             lag_bytes := NULL; \
         END $$"
    )
}

/// The writer of the gate test. It runs in ONE transaction and changes every table and every field
/// that the snapshot reads: a failed login, a CSRF rejection, a chain break, the value of
/// `audit.last_check_at`, and the oldest `created_at` of `app_setting`.
pub const LATER_IMAGE: [&str; 7] = [
    "BEGIN",
    "INSERT INTO security_event (kind, login_hmac, client_ip, request_id) \
     VALUES ('login_failed', '\\x00'::bytea, '10.0.0.2'::inet, gen_random_uuid())",
    "INSERT INTO security_event (kind, login_hmac, client_ip, request_id) \
     VALUES ('csrf_rejected', '\\x00'::bytea, '10.0.0.3'::inet, gen_random_uuid())",
    "INSERT INTO security_event (kind, login_hmac, request_id) \
     VALUES ('audit_chain_broken', '\\x00'::bytea, gen_random_uuid())",
    "UPDATE app_setting SET value = '2026-10-23T03:00:00.000000Z' WHERE key = 'audit.last_check_at'",
    "INSERT INTO app_setting (key, value, created_at) \
     VALUES ('heartbeat.last_at', '2026-10-23T03:00:00.000000Z', '2026-01-01 00:00:00+00')",
    "COMMIT",
];

/// Close the snapshot gate on a connection. A snapshot then waits inside `replication_state()`.
pub async fn close_snapshot_gate(connection: &mut PgConnection) {
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(SNAPSHOT_GATE_KEY)
        .execute(connection)
        .await
        .unwrap();
}

/// Open the snapshot gate on the connection that closed it.
pub async fn open_snapshot_gate(connection: &mut PgConnection) {
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(SNAPSHOT_GATE_KEY)
        .execute(connection)
        .await
        .unwrap();
}

/// Await a future and fail the test when it needs more than the outer time limit.
pub async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(OUTER_LIMIT, future)
        .await
        .expect("a step of the test passed the outer time limit")
}

/// Poll until a session of `recipe_app` waits for a lock, which proves a connection is in use.
pub async fn wait_until_app_session_waits(admin: &mut PgConnection) {
    let start = Instant::now();
    loop {
        let waiting: i64 = sqlx::query_scalar(WAITING_APP_SESSIONS)
            .fetch_one(&mut *admin)
            .await
            .unwrap();
        if waiting > 0 {
            return;
        }
        assert!(start.elapsed() < BOUND, "timeout: a connection is in use");
        tokio::time::sleep(POLL).await;
    }
}
