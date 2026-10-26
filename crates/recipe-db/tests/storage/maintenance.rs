//! Tests of `MaintenanceStore` on PostgreSQL (task 4.3, design D14 row T5, `data-model.md` 6).
//!
//! Every test goes through the port of `recipe-app`. The adapter connects as `recipe_app`, so each
//! statement runs with the grants of that role. The tests seed and read the tables as the
//! superuser of the harness. A test trigger of the superuser injects each fault.

use std::sync::Arc;
use std::time::Duration;

use recipe_app::error::StorageError;
use recipe_app::ports::{ChainSeq, MaintenanceStore, StateValue};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::timestamp::Timestamp;
use sqlx::PgConnection;
use time::{Date, Month, UtcOffset};
use uuid::Uuid;

use crate::support::fault::FAULT;
use crate::support::{Role, TestDatabase, fixtures, probe};

/// `login_attempt.purge_batch` and `security_event.purge_batch` (`reliability.md` 7.11).
const BATCH: u32 = 5_000;
/// One row more than a full batch.
const ROWS_OVER_ONE_BATCH: i64 = 5_001;
/// `login.attempt_retention`.
const ATTEMPT_RETENTION: Duration = Duration::from_secs(24 * 60 * 60);
/// `session.absolute_timeout`.
const ABSOLUTE_TIMEOUT: Duration = Duration::from_secs(12 * 60 * 60);
/// A shorter absolute timeout of the second call of the session test.
const SHORT_TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// The SQLSTATE of the injected fault.
const FAULT_STATE: &str = "P0001";
const SEQ_KEY: &str = "audit.last_check_seq";
const CHECK_AT_KEY: &str = "audit.last_check_at";
const HEARTBEAT_KEY: &str = "heartbeat.last_at";

/// The adapter as `recipe_app` and a session of the superuser for the seed and the reads.
struct Fixture {
    store: Arc<dyn MaintenanceStore>,
    admin: PgConnection,
}

impl Fixture {
    async fn create() -> Self {
        let database = TestDatabase::create().await;
        let app = Database::connect(&database.target(Role::App), &PoolSettings::default())
            .await
            .unwrap();
        Self {
            store: app.maintenance(),
            admin: database.connect(Role::Superuser).await,
        }
    }

    /// Count the rows that a query selects.
    async fn count(&mut self, sql: &str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(&mut self.admin)
            .await
            .unwrap()
    }

    /// Insert a session. The two intervals are texts of the test, such as `13 hours`.
    async fn seed_session(&mut self, user: Uuid, number: u32, created_ago: &str, expires_in: &str) {
        sqlx::query(
            "INSERT INTO session (id, user_id, epoch, csrf_token, created_at, expires_at) \
             VALUES ($1, $2, 0, $1, now() - $3::interval, now() + $4::interval)",
        )
        .bind(format!("{number:064x}"))
        .bind(user)
        .bind(created_ago)
        .bind(expires_in)
        .execute(&mut self.admin)
        .await
        .unwrap();
    }

    /// The numbers of the sessions that remain, in order.
    async fn session_numbers(&mut self) -> Vec<i64> {
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM session ORDER BY id")
            .fetch_all(&mut self.admin)
            .await
            .unwrap();
        ids.iter()
            .map(|id| i64::from_str_radix(id, 16).unwrap())
            .collect()
    }

    /// Insert `count` failed attempts that are older than the retention, the youngest first.
    /// The logins are `l1`, `l2`, and so on. `l1` is the youngest and has the lowest `id`. The last
    /// login is the oldest and comes last in the heap. So only `ORDER BY attempted_at` deletes it
    /// first. The heap order and the `id` order give the opposite result.
    async fn seed_old_attempts(&mut self, count: i64) {
        sqlx::query(
            "INSERT INTO login_attempt (login, client_ip, attempted_at, succeeded) \
             SELECT 'l' || g, '10.0.0.1'::inet, \
                    now() - interval '48 hours' - g * interval '1 second', false \
             FROM generate_series(1, $1::bigint) AS g",
        )
        .bind(count)
        .execute(&mut self.admin)
        .await
        .unwrap();
    }

    async fn seed_recent_attempts(&mut self, count: i64) {
        sqlx::query(
            "INSERT INTO login_attempt (login, client_ip, attempted_at, succeeded) \
             SELECT 'recent' || g, '10.0.0.2'::inet, now() - interval '1 hour', false \
             FROM generate_series(1, $1::bigint) AS g",
        )
        .bind(count)
        .execute(&mut self.admin)
        .await
        .unwrap();
    }

    async fn logins(&mut self) -> Vec<String> {
        sqlx::query_scalar("SELECT login FROM login_attempt ORDER BY attempted_at")
            .fetch_all(&mut self.admin)
            .await
            .unwrap()
    }

    /// Insert `count` events of the age in days. The kind does not aggregate.
    async fn seed_events(&mut self, count: i64, age_days: i32) {
        sqlx::query(
            "INSERT INTO security_event (occurred_at, kind, login_hmac, client_ip, request_id) \
             SELECT now() - make_interval(days => $2), 'forbidden', '\\x00'::bytea, \
                    '10.0.0.1'::inet, gen_random_uuid() \
             FROM generate_series(1, $1::bigint)",
        )
        .bind(count)
        .bind(age_days)
        .execute(&mut self.admin)
        .await
        .unwrap();
    }

    /// The text of a state key, or `None` when the row is missing.
    async fn stored(&mut self, key: &str) -> Option<String> {
        sqlx::query_scalar("SELECT value FROM app_setting WHERE key = $1")
            .bind(key)
            .fetch_optional(&mut self.admin)
            .await
            .unwrap()
    }
}

fn assert_fault(error: &StorageError) {
    assert!(
        matches!(error, StorageError::Other { sqlstate, .. }
            if sqlstate.as_deref() == Some(FAULT_STATE)),
        "{error:?}"
    );
}

/// 2026-10-23 at 02:00:00 UTC with the given nanosecond, shown in the given offset.
fn instant(nanosecond: u32, offset_hours: i8) -> Timestamp {
    let utc = Date::from_calendar_date(2026, Month::October, 23)
        .unwrap()
        .with_hms_nano(2, 0, 0, nanosecond)
        .unwrap()
        .assume_utc();
    Timestamp::from(utc.to_offset(UtcOffset::from_hms(offset_hours, 0, 0).unwrap()))
}

#[tokio::test]
async fn maintenance_session_delete_removes_expired_and_too_old_sessions_only() {
    let mut f = Fixture::create().await;
    let user = fixtures::user(&mut f.admin).await;
    f.seed_session(user, 1, "1 hour", "-1 minute").await;
    f.seed_session(user, 2, "13 hours", "1 hour").await;
    f.seed_session(user, 3, "1 hour", "1 hour").await;
    f.seed_session(user, 4, "20 hours", "-2 hours").await;

    let deleted = f.store.delete_expired_sessions(ABSOLUTE_TIMEOUT).await;

    assert_eq!(deleted.unwrap(), 3);
    assert_eq!(f.session_numbers().await, vec![3]);
    assert_eq!(
        f.store
            .delete_expired_sessions(ABSOLUTE_TIMEOUT)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        f.store
            .delete_expired_sessions(SHORT_TIMEOUT)
            .await
            .unwrap(),
        1,
        "the absolute timeout is the argument of the call"
    );
    assert!(f.session_numbers().await.is_empty());
}

#[tokio::test]
async fn maintenance_a_failed_session_delete_keeps_every_session() {
    let mut f = Fixture::create().await;
    let user = fixtures::user(&mut f.admin).await;
    for number in 1..=3 {
        f.seed_session(user, number, "1 hour", "-1 minute").await;
    }
    probe::install_on(
        &mut f.admin,
        "session",
        "AFTER",
        "DELETE",
        &format!("IF OLD.id = '{:064x}' THEN {FAULT} END IF;", 3),
    )
    .await;

    let error = f
        .store
        .delete_expired_sessions(ABSOLUTE_TIMEOUT)
        .await
        .unwrap_err();

    assert_fault(&error);
    assert_eq!(f.session_numbers().await, vec![1, 2, 3]);
}

#[tokio::test]
async fn maintenance_attempt_purge_deletes_batches_of_5000_and_keeps_recent_rows() {
    let mut f = Fixture::create().await;
    f.seed_old_attempts(ROWS_OVER_ONE_BATCH).await;
    f.seed_recent_attempts(3).await;

    let first = f.store.purge_login_attempts(ATTEMPT_RETENTION, BATCH).await;
    let second = f.store.purge_login_attempts(ATTEMPT_RETENTION, BATCH).await;
    let third = f.store.purge_login_attempts(ATTEMPT_RETENTION, BATCH).await;

    assert_eq!(
        (first.unwrap(), second.unwrap(), third.unwrap()),
        (5_000, 1, 0)
    );
    assert_eq!(f.count("SELECT count(*) FROM login_attempt").await, 3);
    assert_eq!(
        f.count("SELECT count(*) FROM login_attempt WHERE login LIKE 'recent%'")
            .await,
        3
    );
}

#[tokio::test]
async fn maintenance_attempt_purge_deletes_the_oldest_rows_of_a_batch_first() {
    let mut f = Fixture::create().await;
    f.seed_old_attempts(5).await;

    let deleted = f.store.purge_login_attempts(ATTEMPT_RETENTION, 2).await;

    assert_eq!(deleted.unwrap(), 2);
    assert_eq!(f.logins().await, vec!["l3", "l2", "l1"]);
}

#[tokio::test]
async fn maintenance_a_failure_in_the_second_attempt_batch_keeps_the_first_batch_deleted() {
    let mut f = Fixture::create().await;
    f.seed_old_attempts(7).await;
    probe::install_on(
        &mut f.admin,
        "login_attempt",
        "AFTER",
        "DELETE",
        &format!("IF OLD.login = 'l1' THEN {FAULT} END IF;"),
    )
    .await;

    let first = f.store.purge_login_attempts(ATTEMPT_RETENTION, 5).await;
    let second = f.store.purge_login_attempts(ATTEMPT_RETENTION, 5).await;

    assert_eq!(first.unwrap(), 5);
    assert_fault(&second.unwrap_err());
    assert_eq!(
        f.logins().await,
        vec!["l2", "l1"],
        "the first batch stays deleted and the failed batch stays whole"
    );
}

#[tokio::test]
async fn maintenance_event_purge_deletes_batches_of_5000_and_keeps_recent_rows() {
    let mut f = Fixture::create().await;
    f.seed_events(ROWS_OVER_ONE_BATCH, 500).await;
    f.seed_events(2, 1).await;

    let first = f.store.purge_security_events().await;
    let second = f.store.purge_security_events().await;
    let third = f.store.purge_security_events().await;

    assert_eq!(
        (first.unwrap(), second.unwrap(), third.unwrap()),
        (5_000, 1, 0)
    );
    assert_eq!(f.count("SELECT count(*) FROM security_event").await, 2);
}

/// ADR-0098 (h): the condition holds the grouping `occurred_at < (now() - 400 days)`.
#[tokio::test]
async fn maintenance_event_purge_keeps_an_event_of_399_days_and_deletes_one_of_401_days() {
    let mut f = Fixture::create().await;
    f.seed_events(1, 399).await;
    f.seed_events(1, 401).await;

    let first = f.store.purge_security_events().await;
    let second = f.store.purge_security_events().await;

    assert_eq!((first.unwrap(), second.unwrap()), (1, 0));
    let remaining: Vec<bool> =
        sqlx::query_scalar("SELECT occurred_at < now() - interval '400 days' FROM security_event")
            .fetch_all(&mut f.admin)
            .await
            .unwrap();
    assert_eq!(remaining, [false], "only the event of 399 days remains");
}

#[tokio::test]
async fn maintenance_a_failure_in_the_second_event_batch_keeps_the_first_batch_deleted() {
    let mut f = Fixture::create().await;
    f.seed_events(ROWS_OVER_ONE_BATCH, 500).await;
    f.seed_events(2, 1).await;
    let last_old = f
        .count("SELECT max(id) FROM security_event WHERE occurred_at < now() - interval '400 days'")
        .await;
    probe::install_on(
        &mut f.admin,
        "security_event",
        "AFTER",
        "DELETE",
        &format!("IF OLD.id = {last_old} THEN {FAULT} END IF;"),
    )
    .await;

    let first = f.store.purge_security_events().await;
    let second = f.store.purge_security_events().await;

    assert_eq!(first.unwrap(), 5_000);
    assert_fault(&second.unwrap_err());
    assert_eq!(
        f.count("SELECT count(*) FROM security_event").await,
        3,
        "the failed batch keeps its one old row and the two recent rows"
    );
}

#[tokio::test]
async fn maintenance_state_write_stores_the_text_form_of_each_key() {
    let mut f = Fixture::create().await;
    let cases = [
        (StateValue::AuditLastCheckSeq(ChainSeq::ZERO), SEQ_KEY, "0"),
        (
            StateValue::AuditLastCheckSeq(ChainSeq::from(41)),
            SEQ_KEY,
            "41",
        ),
        (
            StateValue::AuditLastCheckSeq(ChainSeq::from(i64::MAX)),
            SEQ_KEY,
            "9223372036854775807",
        ),
        (
            StateValue::AuditLastCheckAt(instant(0, 0)),
            CHECK_AT_KEY,
            "2026-10-23T02:00:00.000000Z",
        ),
        (
            StateValue::HeartbeatLastAt(instant(123_456_789, 2)),
            HEARTBEAT_KEY,
            "2026-10-23T02:00:00.123456Z",
        ),
        (
            StateValue::HeartbeatLastAt(instant(5_000, 0)),
            HEARTBEAT_KEY,
            "2026-10-23T02:00:00.000005Z",
        ),
    ];

    for (value, key, text) in cases {
        f.store.record_state(&value).await.unwrap();

        assert_eq!(f.stored(key).await.as_deref(), Some(text), "{value:?}");
    }
}

#[tokio::test]
async fn maintenance_state_write_inserts_one_row_and_then_updates_it_with_a_null_updated_by() {
    let mut f = Fixture::create().await;
    f.store
        .record_state(&StateValue::HeartbeatLastAt(instant(0, 0)))
        .await
        .unwrap();
    let id: Uuid = sqlx::query_scalar("SELECT id FROM app_setting WHERE key = $1")
        .bind(HEARTBEAT_KEY)
        .fetch_one(&mut f.admin)
        .await
        .unwrap();
    let user = fixtures::user(&mut f.admin).await;
    sqlx::query("UPDATE app_setting SET updated_by = $1 WHERE key = $2")
        .bind(user)
        .bind(HEARTBEAT_KEY)
        .execute(&mut f.admin)
        .await
        .unwrap();

    f.store
        .record_state(&StateValue::HeartbeatLastAt(instant(1_000, 0)))
        .await
        .unwrap();

    let rows: Vec<(Uuid, String, Option<Uuid>)> =
        sqlx::query_as("SELECT id, value, updated_by FROM app_setting")
            .fetch_all(&mut f.admin)
            .await
            .unwrap();
    assert_eq!(
        rows,
        vec![(id, "2026-10-23T02:00:00.000001Z".to_owned(), None)]
    );
}

#[tokio::test]
async fn maintenance_state_write_writes_no_audit_entry() {
    let mut f = Fixture::create().await;
    let before = f.count("SELECT count(*) FROM audit_log").await;

    for value in [
        StateValue::AuditLastCheckSeq(ChainSeq::from(7)),
        StateValue::AuditLastCheckAt(instant(0, 0)),
        StateValue::HeartbeatLastAt(instant(0, 0)),
    ] {
        f.store.record_state(&value).await.unwrap();
    }

    assert_eq!(f.count("SELECT count(*) FROM app_setting").await, 3);
    assert_eq!(f.count("SELECT count(*) FROM audit_log").await, before);
}

#[tokio::test]
async fn maintenance_state_write_refuses_a_negative_chain_seq_and_writes_nothing() {
    let mut f = Fixture::create().await;

    let error = f
        .store
        .record_state(&StateValue::AuditLastCheckSeq(ChainSeq::from(-1)))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate: None, .. }),
        "{error:?}"
    );
    assert_eq!(f.stored(SEQ_KEY).await, None);
}

#[tokio::test]
async fn maintenance_a_failed_state_write_leaves_the_stored_text_unchanged() {
    let mut f = Fixture::create().await;
    f.store
        .record_state(&StateValue::HeartbeatLastAt(instant(0, 0)))
        .await
        .unwrap();
    probe::install_on(&mut f.admin, "app_setting", "BEFORE", "UPDATE", FAULT).await;

    let error = f
        .store
        .record_state(&StateValue::HeartbeatLastAt(instant(9_000, 0)))
        .await
        .unwrap_err();

    assert_fault(&error);
    assert_eq!(
        f.stored(HEARTBEAT_KEY).await.as_deref(),
        Some("2026-10-23T02:00:00.000000Z")
    );
}
