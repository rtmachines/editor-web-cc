//! Tests of `OperationsReads` and `ReadinessProbe` on PostgreSQL (task 4.5, design D14 row T7,
//! ADR-0059, ADR-0073).
//!
//! Every test goes through the port of `recipe-app`. The adapters connect as `recipe_app`, so each
//! statement runs with the grants of that role. The tests plant rows and break the schema as the
//! superuser of the harness. Each test has its own database, so no planted case outlives its test.

use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::Instant;

use recipe_app::error::StorageError;
use recipe_app::ports::{
    NotReadyCause, OperationsReads, OperationsSnapshot, Readiness, ReadinessProbe, SnapshotWindows,
    TimeStateKey,
};
use recipe_db::settings::PoolSettings;
use recipe_db::{CloseOutcome, Database};
use recipe_domain::timestamp::Timestamp;
use sqlx::PgConnection;
use time::{Date, Month, OffsetDateTime};

use crate::support::{Role, TestDatabase, hold};

/// `alerts.failed_login_window` and `alerts.security_window` for the tests.
const WINDOWS: SnapshotWindows = SnapshotWindows {
    failed_logins: Duration::from_secs(15 * 60),
    security: Duration::from_secs(60 * 60),
};
/// `health.db_check_timeout` (`reliability.md` 7.11).
const CHECK_TIMEOUT: Duration = Duration::from_secs(2);
/// A limit that no test reaches.
const LONG_LIMIT: Duration = Duration::from_secs(10);
/// A limit that a held connection always passes.
const SHORT_LIMIT: Duration = Duration::from_millis(300);
/// A bad stored text of a state key. Each one breaks one rule of ADR-0073 (i).
const BAD_TIME_TEXTS: [&str; 7] = [
    "",
    "garbage",
    "2026-10-23 02:00:00.000000+00",
    "2026-10-23T02:00:00Z",
    "2026-10-23T02:00:00.00000Z",
    "2026-10-23T02:00:00.000000+00:00",
    "2026-10-23T04:00:00.000000+02:00",
];

/// A test database with the adapters as `recipe_app` and a session of the superuser.
struct Fixture {
    database: TestDatabase,
    app: Database,
    reads: Arc<dyn OperationsReads>,
    probe: Arc<dyn ReadinessProbe>,
    admin: PgConnection,
}

impl Fixture {
    async fn create() -> Self {
        Self::create_with(&PoolSettings::default()).await
    }

    async fn create_with(settings: &PoolSettings) -> Self {
        let database = TestDatabase::create().await;
        let app = Database::connect(&database.target(Role::App), settings)
            .await
            .unwrap();
        Self {
            reads: app.operations(),
            probe: app.readiness(),
            admin: database.connect(Role::Superuser).await,
            app,
            database,
        }
    }

    /// Run a statement of the superuser.
    async fn exec(&mut self, sql: &str) {
        sqlx::query(sql).execute(&mut self.admin).await.unwrap();
    }

    /// Plant one row of `security_event`. `age` is a text such as `10 minutes`.
    async fn event(&mut self, kind: &str, count: i32, age: &str, ip: Option<&str>, detail: &str) {
        sqlx::query(
            "INSERT INTO security_event \
                 (occurred_at, kind, login_hmac, client_ip, request_id, detail, count) \
             VALUES (now() - $1::interval, $2, '\\x00'::bytea, $3::inet, gen_random_uuid(), \
                     $4::jsonb, $5)",
        )
        .bind(age)
        .bind(kind)
        .bind(ip)
        .bind(detail)
        .bind(count)
        .execute(&mut self.admin)
        .await
        .unwrap();
    }

    /// Plant a state row or a configuration row with its text and creation time.
    async fn setting(&mut self, key: &str, value: &str, created_at: &str) {
        sqlx::query(
            "INSERT INTO app_setting (key, value, created_at) VALUES ($1, $2, $3::timestamptz)",
        )
        .bind(key)
        .bind(value)
        .bind(created_at)
        .execute(&mut self.admin)
        .await
        .unwrap();
    }

    /// The time of the database as the superuser reads it.
    async fn now(&mut self) -> OffsetDateTime {
        sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut self.admin)
            .await
            .unwrap()
    }

    /// The stored `occurred_at` of the newest row of a kind.
    async fn newest(&mut self, kind: &str) -> Timestamp {
        let at: OffsetDateTime =
            sqlx::query_scalar("SELECT max(occurred_at) FROM security_event WHERE kind = $1")
                .bind(kind)
                .fetch_one(&mut self.admin)
                .await
                .unwrap();
        Timestamp::from(at)
    }

    async fn snapshot(&self) -> Result<OperationsSnapshot, StorageError> {
        self.reads.snapshot(&WINDOWS).await
    }
}

/// An instant of 2026 at the given month, day, and clock time in UTC.
fn instant(month: Month, day: u8, hour: u8, minute: u8, microsecond: u32) -> Timestamp {
    let utc = Date::from_calendar_date(2026, month, day)
        .unwrap()
        .with_hms_micro(hour, minute, 0, microsecond)
        .unwrap()
        .assume_utc();
    Timestamp::from(utc)
}

/// Assert a defect of a stored row (`E-DB-008`) that names the table and not the stored text.
fn assert_corrupt_setting(error: &StorageError, text: &str) {
    assert!(
        matches!(error, StorageError::Other { sqlstate: None, .. }),
        "{error:?}"
    );
    let shown = format!("{error:?}");
    assert!(shown.contains("app_setting"), "{shown}");
    if !text.is_empty() {
        assert!(!shown.contains(text), "{shown}");
    }
}

#[tokio::test]
async fn snapshot_of_an_empty_database_has_zero_counts_and_no_times() {
    let mut f = Fixture::create().await;

    let before = f.now().await;
    let snapshot = f.snapshot().await.unwrap();
    let after = f.now().await;

    let database_now = *snapshot.database_now.as_offset_date_time();
    assert!(before <= database_now && database_now <= after);
    assert!(!snapshot.replication.connected);
    assert_eq!(snapshot.replication.lag_bytes, None);
    assert_eq!(snapshot.failed_logins, 0);
    assert_eq!(snapshot.security_alerts, 0);
    assert_eq!(snapshot.audit_chain_broken_at, None);
    assert_eq!(snapshot.audit_last_check_at, None);
    assert_eq!(snapshot.first_setting_at, None);
}

#[tokio::test]
async fn snapshot_sums_failed_logins_inside_the_window_only() {
    let mut f = Fixture::create().await;
    f.event("login_failed", 3, "10 minutes", Some("10.0.0.1"), "null")
        .await;
    f.event("login_failed", 2, "1 minute", Some("10.0.0.2"), "null")
        .await;
    f.event("login_failed", 9, "20 minutes", Some("10.0.0.3"), "null")
        .await;
    f.event("login_throttled", 4, "1 minute", Some("10.0.0.4"), "null")
        .await;

    let snapshot = f.snapshot().await.unwrap();

    assert_eq!(snapshot.failed_logins, 5);
    assert_eq!(snapshot.security_alerts, 0);
}

#[tokio::test]
async fn snapshot_counts_csrf_rejections_and_forbidden_user_management_only() {
    let mut f = Fixture::create().await;
    f.event("csrf_rejected", 2, "30 minutes", Some("10.0.0.1"), "null")
        .await;
    f.event("csrf_rejected", 7, "2 hours", Some("10.0.0.2"), "null")
        .await;
    f.event(
        "forbidden",
        1,
        "10 minutes",
        Some("10.0.0.3"),
        r#"{"operation":"CreateUser"}"#,
    )
    .await;
    f.event(
        "forbidden",
        1,
        "10 minutes",
        Some("10.0.0.3"),
        r#"{"operation":"ListUsers"}"#,
    )
    .await;
    f.event(
        "forbidden",
        1,
        "10 minutes",
        Some("10.0.0.3"),
        r#"{"operation":"RenameMachine"}"#,
    )
    .await;
    f.event("forbidden", 1, "10 minutes", Some("10.0.0.3"), "null")
        .await;
    f.event(
        "forbidden",
        1,
        "3 hours",
        Some("10.0.0.3"),
        r#"{"operation":"CreateUser"}"#,
    )
    .await;

    let snapshot = f.snapshot().await.unwrap();

    assert_eq!(snapshot.security_alerts, 4);
    assert_eq!(snapshot.failed_logins, 0);
}

#[tokio::test]
async fn snapshot_returns_the_newest_chain_break_and_the_stored_times() {
    let mut f = Fixture::create().await;
    f.event("audit_chain_broken", 1, "3 days", None, "null")
        .await;
    f.event("audit_chain_broken", 1, "1 day", None, "null")
        .await;
    f.setting(
        "audit.last_check_at",
        "2026-10-23T02:00:00.123456Z",
        "2026-02-01 00:00:00+00",
    )
    .await;
    f.setting(
        "heartbeat.last_at",
        "2026-10-23T03:00:00.000000Z",
        "2026-01-02 03:04:05+00",
    )
    .await;
    let newest = f.newest("audit_chain_broken").await;

    let snapshot = f.snapshot().await.unwrap();

    assert_eq!(snapshot.audit_chain_broken_at, Some(newest));
    assert_eq!(
        snapshot.audit_last_check_at,
        Some(instant(Month::October, 23, 2, 0, 123_456))
    );
    assert_eq!(
        snapshot.first_setting_at,
        Some(Timestamp::from(
            Date::from_calendar_date(2026, Month::January, 2)
                .unwrap()
                .with_hms(3, 4, 5)
                .unwrap()
                .assume_utc()
        ))
    );
}

#[tokio::test]
async fn snapshot_fails_for_a_bad_text_of_audit_last_check_at() {
    for text in BAD_TIME_TEXTS {
        let mut f = Fixture::create().await;
        f.setting("audit.last_check_at", text, "2026-02-01 00:00:00+00")
            .await;

        let error = f.snapshot().await.unwrap_err();

        assert_corrupt_setting(&error, text);
    }
}

#[tokio::test]
async fn snapshot_ignores_a_bad_text_of_the_other_state_keys() {
    let mut f = Fixture::create().await;
    f.setting("heartbeat.last_at", "garbage", "2026-02-01 00:00:00+00")
        .await;
    f.setting("audit.last_check_seq", "007", "2026-02-02 00:00:00+00")
        .await;

    let snapshot = f.snapshot().await.unwrap();

    assert_eq!(snapshot.audit_last_check_at, None);
    assert!(snapshot.first_setting_at.is_some());
}

#[tokio::test]
async fn snapshot_describes_one_moment_while_rows_arrive() {
    // Each transaction of the writer adds one failed login and one CSRF rejection. A snapshot
    // that reads in two statements could see only one of the two rows. One statement cannot.
    let mut f = Fixture::create().await;
    let reads = Arc::clone(&f.reads);
    let reader = tokio::spawn(async move {
        let mut seen = Vec::new();
        for _ in 0..150 {
            let snapshot = reads.snapshot(&WINDOWS).await.unwrap();
            seen.push((snapshot.failed_logins, snapshot.security_alerts));
        }
        seen
    });

    for number in 0..150_u32 {
        let ip = format!("10.1.{}.{}", number / 200, number % 200 + 1);
        sqlx::query(
            "WITH a AS (INSERT INTO security_event (kind, login_hmac, client_ip, request_id) \
                        VALUES ('login_failed', '\\x00'::bytea, $1::inet, gen_random_uuid())) \
             INSERT INTO security_event (kind, login_hmac, client_ip, request_id) \
             VALUES ('csrf_rejected', '\\x00'::bytea, $1::inet, gen_random_uuid())",
        )
        .bind(ip)
        .execute(&mut f.admin)
        .await
        .unwrap();
        tokio::task::yield_now().await;
    }

    let seen = reader.await.unwrap();
    assert!(
        seen.iter().all(|(logins, alerts)| logins == alerts),
        "{seen:?}"
    );
}

fn moving_image(s: &OperationsSnapshot) -> [Option<Timestamp>; 3] {
    [
        s.audit_chain_broken_at,
        s.audit_last_check_at,
        s.first_setting_at,
    ]
}

#[tokio::test]
async fn snapshot_shows_the_image_of_its_start_although_all_its_tables_change_meanwhile() {
    // The planted gate holds the statement after its data snapshot, so a second read would differ.
    let mut f = Fixture::create_with(&holding_settings()).await;
    f.exec(&hold::plant_snapshot_gate()).await;
    f.event("login_failed", 1, "1 minute", Some("10.0.0.1"), "null")
        .await;
    f.setting(
        "audit.last_check_at",
        "2026-10-23T02:00:00.000000Z",
        "2026-02-01 00:00:00+00",
    )
    .await;
    let mut gate = f.database.connect(Role::Superuser).await;
    hold::close_snapshot_gate(&mut gate).await;
    let reads = Arc::clone(&f.reads);
    let reader = tokio::spawn(async move { reads.snapshot(&WINDOWS).await });
    hold::wait_until_app_session_waits(&mut f.admin).await;

    for statement in hold::LATER_IMAGE {
        f.exec(statement).await;
    }
    hold::open_snapshot_gate(&mut gate).await;
    let earlier = hold::bounded(reader).await.unwrap().unwrap();
    let later = f.snapshot().await.unwrap();

    assert_eq!((earlier.failed_logins, earlier.security_alerts), (1, 0));
    assert_eq!(
        moving_image(&earlier),
        [
            None,
            Some(instant(Month::October, 23, 2, 0, 0)),
            Some(instant(Month::February, 1, 0, 0, 0)),
        ]
    );
    assert_eq!((later.failed_logins, later.security_alerts), (2, 1));
    assert_eq!(
        moving_image(&later),
        [
            Some(f.newest("audit_chain_broken").await),
            Some(instant(Month::October, 23, 3, 0, 0)),
            Some(instant(Month::January, 1, 0, 0, 0)),
        ]
    );
}

/// The SQLSTATE that the planted `replication_state()` raises for a wrong transaction mode.
const WRONG_MODE_SQLSTATE: &str = "TX001";

/// Replace `replication_state()` with a function that accepts only one transaction mode.
///
/// The adapter gives no access to the settings of its transaction. The planted function reads
/// them inside the snapshot statement and raises `TX001` for any other mode. The owner, the
/// signature, `SECURITY DEFINER`, the `search_path`, and the grants stay as in migration 0006.
async fn plant_mode_check(f: &mut Fixture, read_only: &str, isolation: &str) {
    f.exec(&format!(
        "CREATE OR REPLACE FUNCTION replication_state(OUT connected BOOLEAN, OUT lag_bytes NUMERIC) \
         LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$ \
         BEGIN \
             IF pg_catalog.current_setting('transaction_read_only') <> '{read_only}' \
                OR pg_catalog.current_setting('transaction_isolation') <> '{isolation}' THEN \
                 RAISE EXCEPTION 'wrong transaction mode' USING ERRCODE = '{WRONG_MODE_SQLSTATE}'; \
             END IF; \
             connected := false; \
             lag_bytes := NULL; \
         END $$"
    ))
    .await;
}

fn is_wrong_mode(error: &StorageError) -> bool {
    matches!(error, StorageError::Other { sqlstate, .. }
        if sqlstate.as_deref() == Some(WRONG_MODE_SQLSTATE))
}

#[tokio::test]
async fn snapshot_runs_in_a_read_only_read_committed_transaction() {
    let mut f = Fixture::create().await;
    plant_mode_check(&mut f, "on", "read committed").await;

    let snapshot = f.snapshot().await;

    assert!(snapshot.is_ok(), "{snapshot:?}");
}

#[tokio::test]
async fn the_planted_mode_check_fails_the_snapshot_for_another_mode() {
    // This proves that the planted function sees the settings, so the test above is not vacuous.
    for (read_only, isolation) in [("off", "read committed"), ("on", "repeatable read")] {
        let mut f = Fixture::create().await;
        plant_mode_check(&mut f, read_only, isolation).await;

        let error = f.snapshot().await.unwrap_err();

        assert!(is_wrong_mode(&error), "{read_only} {isolation}: {error:?}");
    }
}

#[tokio::test]
async fn snapshot_fails_as_a_whole_when_replication_state_is_not_granted() {
    let mut f = Fixture::create().await;
    f.exec("REVOKE EXECUTE ON FUNCTION replication_state() FROM recipe_app")
        .await;
    f.event("login_failed", 3, "1 minute", Some("10.0.0.1"), "null")
        .await;

    let error = f.snapshot().await.unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate, .. } if sqlstate.as_deref() == Some("42501")),
        "{error:?}"
    );
    // The other methods do not call the function.
    assert!(f.reads.database_now().await.is_ok());
    assert_eq!(
        f.reads.state(TimeStateKey::AuditLastCheckAt).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn snapshot_fails_for_a_role_without_the_grant_of_replication_state() {
    // The backup role reads the tables for `pg_dump`, but only `recipe_app` may run the function.
    let f = Fixture::create().await;
    let backup = Database::connect(&f.database.target(Role::Backup), &PoolSettings::default())
        .await
        .unwrap();

    let error = backup.operations().snapshot(&WINDOWS).await.unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate, .. } if sqlstate.as_deref() == Some("42501")),
        "{error:?}"
    );
}

#[tokio::test]
async fn database_now_is_the_time_of_the_database() {
    let mut f = Fixture::create().await;

    let before = f.now().await;
    let read = f.reads.database_now().await.unwrap();
    let after = f.now().await;

    assert!(before <= *read.as_offset_date_time() && *read.as_offset_date_time() <= after);
}

#[tokio::test]
async fn role_check_allows_only_the_application_role() {
    let f = Fixture::create().await;
    let allowed = f.reads.role_check().await.unwrap();
    assert_eq!(allowed.current_user, "recipe_app");
    assert!(!allowed.is_superuser && !allowed.owns_relations);
    assert!(allowed.is_allowed());

    let migrator = role_check_of(&f.database, Role::Migrator).await;
    assert_eq!(migrator.current_user, "recipe_migrator");
    assert!(migrator.owns_relations && !migrator.is_superuser);
    assert!(!migrator.is_allowed());

    let superuser = role_check_of(&f.database, Role::Superuser).await;
    assert!(superuser.is_superuser);
    assert!(!superuser.is_allowed());
}

/// Run the role check over a pool of another role.
async fn role_check_of(database: &TestDatabase, role: Role) -> recipe_app::ports::RoleCheck {
    let other = Database::connect(&database.target(role), &PoolSettings::default())
        .await
        .unwrap();
    other.operations().role_check().await.unwrap()
}

#[tokio::test]
async fn state_reads_a_time_or_nothing() {
    let mut f = Fixture::create().await;
    assert_eq!(
        f.reads.state(TimeStateKey::HeartbeatLastAt).await.unwrap(),
        None
    );
    f.setting(
        "heartbeat.last_at",
        "2026-10-23T02:00:00.000001Z",
        "2026-02-01 00:00:00+00",
    )
    .await;

    let read = f.reads.state(TimeStateKey::HeartbeatLastAt).await.unwrap();

    assert_eq!(read, Some(instant(Month::October, 23, 2, 0, 1)));
    assert_eq!(
        f.reads.state(TimeStateKey::AuditLastCheckAt).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn state_fails_for_a_bad_text_of_each_time_key() {
    for key in [
        TimeStateKey::AuditLastCheckAt,
        TimeStateKey::HeartbeatLastAt,
    ] {
        for text in BAD_TIME_TEXTS {
            let mut f = Fixture::create().await;
            f.setting(key.key(), text, "2026-02-01 00:00:00+00").await;

            let error = f.reads.state(key).await.unwrap_err();

            assert_corrupt_setting(&error, text);
        }
    }
}

#[tokio::test]
async fn readiness_is_ready_on_a_migrated_database() {
    let f = Fixture::create().await;

    assert_eq!(f.probe.check(CHECK_TIMEOUT).await, Readiness::Ready);
}

#[tokio::test]
async fn readiness_stays_ready_with_a_newer_migration() {
    let mut f = Fixture::create().await;
    f.exec(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
         VALUES (999999, 'newer', true, '\\x00'::bytea, 0)",
    )
    .await;

    assert_eq!(f.probe.check(CHECK_TIMEOUT).await, Readiness::Ready);
}

#[tokio::test]
async fn readiness_is_not_ready_with_a_changed_checksum() {
    let mut f = Fixture::create().await;
    f.exec("UPDATE _sqlx_migrations SET checksum = '\\x00'::bytea WHERE version = 1")
        .await;

    assert_eq!(
        f.probe.check(CHECK_TIMEOUT).await,
        Readiness::NotReady(NotReadyCause::SchemaMismatch)
    );
}

#[tokio::test]
async fn readiness_is_not_ready_with_a_missing_migration() {
    let mut f = Fixture::create().await;
    f.exec(
        "DELETE FROM _sqlx_migrations WHERE version = (SELECT max(version) FROM _sqlx_migrations)",
    )
    .await;

    assert_eq!(
        f.probe.check(CHECK_TIMEOUT).await,
        Readiness::NotReady(NotReadyCause::SchemaMismatch)
    );
}

#[tokio::test]
async fn readiness_is_not_ready_with_a_failed_row() {
    for statement in [
        "UPDATE _sqlx_migrations SET success = false WHERE version = 1",
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
         VALUES (999999, 'failed', false, '\\x00'::bytea, 0)",
    ] {
        let mut f = Fixture::create().await;
        f.exec(statement).await;

        assert_eq!(
            f.probe.check(CHECK_TIMEOUT).await,
            Readiness::NotReady(NotReadyCause::SchemaMismatch),
            "{statement}"
        );
    }
}

#[tokio::test]
async fn readiness_is_not_ready_without_the_migration_table() {
    let mut f = Fixture::create().await;
    f.exec("DROP TABLE _sqlx_migrations").await;

    assert_eq!(
        f.probe.check(CHECK_TIMEOUT).await,
        Readiness::NotReady(NotReadyCause::SchemaMismatch)
    );
}

#[tokio::test]
async fn readiness_times_out_while_a_lock_blocks_the_read_and_recovers_after() {
    let mut f = Fixture::create().await;
    f.exec("BEGIN").await;
    f.exec("LOCK TABLE _sqlx_migrations IN ACCESS EXCLUSIVE MODE")
        .await;

    let blocked = f.probe.check(Duration::from_millis(300)).await;
    f.exec("ROLLBACK").await;

    assert_eq!(blocked, Readiness::NotReady(NotReadyCause::TimedOut));
    assert_eq!(f.probe.check(CHECK_TIMEOUT).await, Readiness::Ready);
}

#[tokio::test]
async fn readiness_is_not_ready_when_the_database_refuses_connections() {
    // `DROP DATABASE` forces a checkpoint, which is slow while many tests run. A closed database
    // and ended sessions give the same fault without it.
    let settings = PoolSettings {
        acquire_timeout: Duration::from_millis(500),
        ..PoolSettings::default()
    };
    let f = Fixture::create_with(&settings).await;
    assert_eq!(f.probe.check(CHECK_TIMEOUT).await, Readiness::Ready);
    // PostgreSQL refuses to disallow connections for the current database.
    let mut other =
        crate::support::connect(crate::support::server(), Role::Superuser, "postgres").await;
    sqlx::query(&format!(
        "ALTER DATABASE \"{}\" WITH ALLOW_CONNECTIONS false",
        f.database.name()
    ))
    .execute(&mut other)
    .await
    .unwrap();
    sqlx::query(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
         WHERE datname = $1 AND usename = 'recipe_app'",
    )
    .bind(f.database.name())
    .execute(&mut other)
    .await
    .unwrap();

    let readiness = f.probe.check(Duration::from_secs(5)).await;

    assert_eq!(readiness, Readiness::NotReady(NotReadyCause::Unreachable));
}

#[tokio::test]
async fn close_ends_closed_and_later_calls_fail_as_unavailable() {
    let f = Fixture::create().await;
    assert!(f.reads.database_now().await.is_ok());

    let outcome = f.app.close(LONG_LIMIT).await;

    assert_eq!(outcome, CloseOutcome::Closed);
    let error = f.reads.database_now().await.unwrap_err();
    assert!(matches!(error, StorageError::Unavailable), "{error:?}");
    assert_eq!(f.app.close(LONG_LIMIT).await, CloseOutcome::Closed);
}

/// The result of the read that a test holds in use.
type HeldRead = JoinHandle<Result<Option<Timestamp>, StorageError>>;

/// A pool whose timeouts are longer than any test, so a held read never ends by itself.
fn holding_settings() -> PoolSettings {
    PoolSettings {
        lock_timeout: Duration::from_secs(60),
        statement_timeout: Duration::from_secs(60),
        ..PoolSettings::default()
    }
}

/// Block a read of the pool on a lock of the superuser and wait until it holds a connection.
///
/// The read ends after the test sends `ROLLBACK` through `f.exec`.
async fn hold_a_connection(f: &mut Fixture) -> HeldRead {
    f.exec("BEGIN").await;
    f.exec("LOCK TABLE app_setting IN ACCESS EXCLUSIVE MODE")
        .await;
    let reads = Arc::clone(&f.reads);
    let held = tokio::spawn(async move { reads.state(TimeStateKey::AuditLastCheckAt).await });
    hold::wait_until_app_session_waits(&mut f.admin).await;
    held
}

/// Poll until the pool refuses new work, which proves that `close` started.
async fn wait_until_refused(reads: &Arc<dyn OperationsReads>) {
    hold::bounded(async {
        while !matches!(reads.database_now().await, Err(StorageError::Unavailable)) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
}

#[tokio::test]
async fn close_stays_pending_until_its_limit_and_then_refuses_new_work() {
    let mut f = Fixture::create_with(&holding_settings()).await;
    let held = hold_a_connection(&mut f).await;

    let started = Instant::now();
    let cut = hold::bounded(f.app.close(SHORT_LIMIT)).await;
    let waited = started.elapsed();
    let refused_time = f.reads.database_now().await;
    let refused_state = f.reads.state(TimeStateKey::HeartbeatLastAt).await;
    f.exec("ROLLBACK").await;
    let finished = hold::bounded(held).await.unwrap();
    let complete = hold::bounded(f.app.close(LONG_LIMIT)).await;

    assert_eq!(cut, CloseOutcome::TimedOut);
    assert!(waited >= SHORT_LIMIT, "{waited:?}");
    assert!(matches!(refused_time, Err(StorageError::Unavailable)));
    assert!(matches!(refused_state, Err(StorageError::Unavailable)));
    assert_eq!(finished.unwrap(), None);
    assert_eq!(complete, CloseOutcome::Closed);
}

#[tokio::test]
async fn close_ends_closed_once_the_used_connection_returns() {
    let mut f = Fixture::create_with(&holding_settings()).await;
    let held = hold_a_connection(&mut f).await;
    let app = f.app.clone();
    let closer = tokio::spawn(async move { app.close(LONG_LIMIT).await });
    wait_until_refused(&f.reads).await;

    let pending_while_used = !closer.is_finished();
    f.exec("ROLLBACK").await;
    let finished = hold::bounded(held).await.unwrap();
    let outcome = hold::bounded(closer).await.unwrap();

    assert!(
        pending_while_used,
        "close ended while a connection was in use"
    );
    assert_eq!(finished.unwrap(), None);
    assert_eq!(outcome, CloseOutcome::Closed);
}

#[tokio::test]
async fn close_counts_the_connection_of_an_aborted_holder_until_it_returns() {
    // The aborted task is gone, but its connection returns to the pool only after the server
    // ends the statement. Until then `close` must still wait for it.
    let mut f = Fixture::create_with(&holding_settings()).await;
    let held = hold_a_connection(&mut f).await;

    held.abort();
    let joined = hold::bounded(held).await;
    let cut = hold::bounded(f.app.close(SHORT_LIMIT)).await;
    f.exec("ROLLBACK").await;
    let complete = hold::bounded(f.app.close(LONG_LIMIT)).await;

    assert!(joined.unwrap_err().is_cancelled());
    assert_eq!(cut, CloseOutcome::TimedOut);
    assert_eq!(complete, CloseOutcome::Closed);
}
