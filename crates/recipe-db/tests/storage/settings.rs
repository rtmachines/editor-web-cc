//! Tests of `SettingRepository` on the transaction (task 3.2, `data-model.md` sections 3.17 and 6).
//!
//! Every test goes through the port `dyn UnitOfWork` of `recipe-app`, as `CompareSettings` does.
//! The adapter connects as `recipe_app`, so each statement runs with the grants of that role.
//! The tests seed and read the table as the superuser of the harness.

use std::sync::Arc;
use std::time::Duration;

use recipe_app::error::{AppError, StorageError};
use recipe_app::ports::{Isolation, NonEmpty, STATE_SETTING_KEYS, Transaction, UnitOfWork};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::audit::AuditEntry;
use recipe_domain::setting::{SettingKey, SettingValue};
use serde_json::{Value, json};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

use crate::support::audit::{entries, install_trigger};
use crate::support::fault::FAULT;
use crate::support::gate;
use crate::support::{Role, TestDatabase, fixtures, probe};

const SEPARATION: &str = "approval.separation_of_duties";
const DB_HOST: &str = "db.host";
const DB_NAME: &str = "db.name";
const PUBLIC_PORT: &str = "http.public_port";
/// The SQLSTATE of a write in a read-only transaction.
const READ_ONLY_SQL_TRANSACTION: &str = "25006";
/// The SQLSTATE of `NOWAIT` on a locked row.
const LOCK_NOT_AVAILABLE: &str = "55P03";
/// A probe of the lock mode `FOR NO KEY UPDATE` on every row.
const NO_KEY_UPDATE_PROBE: &str = "SELECT 1 FROM app_setting FOR NO KEY UPDATE NOWAIT";
/// A probe of the lock mode `FOR KEY SHARE` on every row.
const KEY_SHARE_PROBE: &str = "SELECT 1 FROM app_setting FOR KEY SHARE NOWAIT";
/// A probe of the lock mode `FOR SHARE` on every row.
const SHARE_PROBE: &str = "SELECT 1 FROM app_setting FOR SHARE NOWAIT";
/// The smaller of the two identifiers of the lock order test.
const LOW_ID: &str = "00000000-0000-0000-0000-000000000001";
/// The larger of the two identifiers of the lock order test.
const HIGH_ID: &str = "00000000-0000-0000-0000-000000000002";
/// A probe of the lock mode `FOR NO KEY UPDATE` on the row with the smaller identifier.
const LOW_ID_PROBE: &str = "SELECT 1 FROM app_setting WHERE id = '00000000-0000-0000-0000-000000000001' \
     FOR NO KEY UPDATE NOWAIT";
/// The pause between two looks at the waiting sessions.
const POLL_PAUSE: Duration = Duration::from_millis(20);
/// The number of looks before the case gives up. The bound is about 5 s.
const POLL_LIMIT: u32 = 250;

/// A test database, the unit of work of `recipe_app`, and a session of the superuser.
struct Fixture {
    database: TestDatabase,
    unit_of_work: Arc<dyn UnitOfWork>,
    admin: PgConnection,
}

/// One stored row as the superuser reads it.
type StoredSetting = (Uuid, String, String, Option<Uuid>);

impl Fixture {
    async fn create() -> Self {
        Self::create_with(&PoolSettings::default()).await
    }

    /// Create the fixture with the pool settings of the adapter session.
    async fn create_with(pool: &PoolSettings) -> Self {
        let database = TestDatabase::create().await;
        let connected = Database::connect(&database.target(Role::App), pool)
            .await
            .unwrap();
        let admin = database.connect(Role::Superuser).await;
        Self {
            database,
            unit_of_work: connected.unit_of_work(),
            admin,
        }
    }

    async fn begin(&self, isolation: Isolation) -> Box<dyn Transaction> {
        self.unit_of_work.begin(isolation).await.unwrap()
    }

    /// Insert rows as the superuser, with `updated_by = NULL`.
    async fn seed(&mut self, rows: &[(&str, &str)]) {
        for (name, text) in rows {
            sqlx::query("INSERT INTO app_setting (key, value) VALUES ($1, $2)")
                .bind(*name)
                .bind(*text)
                .execute(&mut self.admin)
                .await
                .unwrap();
        }
    }

    /// Every row of the table, in the order of the keys.
    async fn stored(&mut self) -> Vec<StoredSetting> {
        sqlx::query_as(
            "SELECT id, key, value, updated_by FROM app_setting ORDER BY key COLLATE \"C\"",
        )
        .fetch_all(&mut self.admin)
        .await
        .unwrap()
    }

    /// The `(key, value)` pairs of the table, in the order of the keys.
    async fn stored_pairs(&mut self) -> Vec<(String, String)> {
        self.stored()
            .await
            .into_iter()
            .map(|(_, name, text, _)| (name, text))
            .collect()
    }

    /// One audit entry list that a commit accepts.
    fn audit() -> NonEmpty<AuditEntry> {
        entries(None, 1).0
    }
}

fn key(text: &str) -> SettingKey {
    SettingKey::parse(text).unwrap()
}

fn value(text: &str) -> SettingValue {
    SettingValue::parse(text).unwrap()
}

fn pair(name: &str, text: &str) -> (String, String) {
    (name.to_owned(), text.to_owned())
}

#[tokio::test]
async fn an_upsert_inserts_a_missing_row_and_the_commit_keeps_it() {
    let mut f = Fixture::create().await;
    let mut tx = f.begin(Isolation::ReadCommitted).await;

    let written = tx
        .settings()
        .upsert(&key(SEPARATION), &value("true"))
        .await
        .unwrap();
    f.unit_of_work.commit(tx, Fixture::audit()).await.unwrap();

    let stored = f.stored().await;
    assert_eq!(stored.len(), 1);
    let (id, name, text, updated_by) = stored.first().unwrap();
    assert_eq!(id, written.id.as_uuid());
    assert_eq!((name.as_str(), text.as_str()), (SEPARATION, "true"));
    assert_eq!(*updated_by, None);
    let image = written.after.as_value();
    assert_eq!(image["id"], json!(id.to_string()));
    assert_eq!(image["key"], json!(SEPARATION));
    assert_eq!(image["value"], json!("true"));
    assert_eq!(image["updated_by"], Value::Null);
}

#[tokio::test]
async fn an_upsert_of_an_existing_row_keeps_its_id_and_clears_updated_by() {
    let mut f = Fixture::create().await;
    f.seed(&[(SEPARATION, "true")]).await;
    let user = fixtures::user(&mut f.admin).await;
    sqlx::query("UPDATE app_setting SET updated_by = $1, updated_at = now() - interval '1 day'")
        .bind(user)
        .execute(&mut f.admin)
        .await
        .unwrap();
    let (old_id, ..) = f.stored().await.remove(0);
    let mut tx = f.begin(Isolation::ReadCommitted).await;

    let written = tx
        .settings()
        .upsert(&key(SEPARATION), &value("false"))
        .await
        .unwrap();
    f.unit_of_work.commit(tx, Fixture::audit()).await.unwrap();

    assert_eq!(written.id.as_uuid(), &old_id);
    assert_eq!(
        f.stored().await,
        vec![(old_id, SEPARATION.to_owned(), "false".to_owned(), None)]
    );
    let fresh: bool =
        sqlx::query_scalar("SELECT updated_at > now() - interval '1 hour' FROM app_setting")
            .fetch_one(&mut f.admin)
            .await
            .unwrap();
    assert!(fresh, "the trigger sets updated_at");
    assert_eq!(written.after.as_value()["value"], json!("false"));
}

#[tokio::test]
async fn the_rows_come_in_key_order_with_their_images_and_skip_the_state_keys() {
    let mut f = Fixture::create().await;
    f.seed(&[
        (PUBLIC_PORT, "8080"),
        ("heartbeat.last_at", "2026-10-23T02:00:00.000000Z"),
        (DB_HOST, "db"),
        ("audit.last_check_seq", "41"),
        ("audit.last_check_at", "2026-10-23T02:00:00.000000Z"),
    ])
    .await;
    let mut tx = f.begin(Isolation::ReadCommitted).await;

    let rows = tx.settings().lock_rows().await.unwrap();

    let found: Vec<(&str, &str)> = rows
        .iter()
        .map(|locked| (locked.state.key.as_str(), locked.state.value.as_str()))
        .collect();
    assert_eq!(found, vec![(DB_HOST, "db"), (PUBLIC_PORT, "8080")]);
    let first = rows.first().unwrap();
    let image = first.before.as_value();
    assert_eq!(image["id"], json!(first.state.id.to_string()));
    assert_eq!(image["key"], json!(DB_HOST));
    assert_eq!(image["value"], json!("db"));
    assert_eq!(image["updated_by"], Value::Null);
}

#[tokio::test]
async fn an_empty_table_gives_no_row() {
    let f = Fixture::create().await;
    let mut tx = f.begin(Isolation::ReadCommitted).await;

    assert!(tx.settings().lock_rows().await.unwrap().is_empty());
}

/// Return the SQLSTATE that `NOWAIT` gives for a row with a conflicting lock, or `None`.
async fn nowait_state(admin: &mut PgConnection, probe: &str) -> Option<String> {
    let error = sqlx::query(probe).execute(admin).await.err()?;
    error
        .as_database_error()
        .and_then(|e| e.code())
        .map(Into::into)
}

#[tokio::test]
async fn the_rows_are_locked_for_no_key_update_until_the_commit() {
    let mut f = Fixture::create().await;
    f.seed(&[(DB_HOST, "db")]).await;
    let mut tx = f.begin(Isolation::ReadCommitted).await;
    tx.settings().lock_rows().await.unwrap();

    let no_key_update = nowait_state(&mut f.admin, NO_KEY_UPDATE_PROBE).await;
    let key_share = nowait_state(&mut f.admin, KEY_SHARE_PROBE).await;
    let share = nowait_state(&mut f.admin, SHARE_PROBE).await;

    // The three probes together pass only for `FOR NO KEY UPDATE` (PostgreSQL 16, Table 13.3).
    // `FOR KEY SHARE` conflicts with `FOR UPDATE` only. `FOR NO KEY UPDATE` conflicts with
    // `FOR SHARE`, `FOR NO KEY UPDATE`, and `FOR UPDATE`. `FOR SHARE` conflicts with
    // `FOR NO KEY UPDATE` and `FOR UPDATE`, but not with `FOR SHARE` or `FOR KEY SHARE`.
    assert_eq!(no_key_update.as_deref(), Some(LOCK_NOT_AVAILABLE));
    assert_eq!(key_share, None, "FOR KEY SHARE must not conflict");
    assert_eq!(share.as_deref(), Some(LOCK_NOT_AVAILABLE));
    f.unit_of_work.commit(tx, Fixture::audit()).await.unwrap();
    assert_eq!(nowait_state(&mut f.admin, NO_KEY_UPDATE_PROBE).await, None);
}

/// Count the sessions of this database that wait for a lock.
async fn waiting_sessions(admin: &mut PgConnection) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_stat_activity \
         WHERE datname = current_database() AND wait_event_type = 'Lock'",
    )
    .fetch_one(admin)
    .await
    .unwrap()
}

/// Poll until one session waits for a lock, and fail after the bound.
async fn wait_for_a_waiting_session(admin: &mut PgConnection) {
    for _ in 0..POLL_LIMIT {
        if waiting_sessions(admin).await > 0 {
            return;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }
    assert!(
        waiting_sessions(admin).await > 0,
        "no session waits for a lock"
    );
}

#[tokio::test]
async fn the_rows_are_locked_in_the_order_of_the_id_and_not_of_the_key() {
    // A race test waits for a lock, so the pool has the long `lock_timeout` of the race tests.
    let mut f = Fixture::create_with(&gate::pool()).await;
    // The key order is the reverse of the id order: `db.host` has the larger `id`.
    // The row of the larger id goes in first. The heap order then differs from the id order,
    // so a missing `ORDER BY id` and an `ORDER BY key` both lock the larger id first.
    for (id, name) in [(HIGH_ID, DB_HOST), (LOW_ID, PUBLIC_PORT)] {
        sqlx::query("INSERT INTO app_setting (id, key, value) VALUES ($1::uuid, $2, 'x')")
            .bind(id)
            .bind(name)
            .execute(&mut f.admin)
            .await
            .unwrap();
    }
    let mut holder = f.database.connect(Role::Superuser).await;
    let mut hold = holder.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM app_setting WHERE id = $1::uuid FOR UPDATE")
        .bind(HIGH_ID)
        .execute(&mut *hold)
        .await
        .unwrap();
    let mut tx = f.begin(Isolation::ReadCommitted).await;
    let task = tokio::spawn(async move {
        let rows = tx.settings().lock_rows().await;
        (tx, rows)
    });

    wait_for_a_waiting_session(&mut f.admin).await;
    let low_row = nowait_state(&mut f.admin, LOW_ID_PROBE).await;
    hold.rollback().await.unwrap();
    let (_tx, rows) = task.await.unwrap();

    assert_eq!(
        low_row.as_deref(),
        Some(LOCK_NOT_AVAILABLE),
        "lock_rows holds the row with the smaller id while it waits for the larger id"
    );
    let keys: Vec<String> = rows
        .unwrap()
        .iter()
        .map(|locked| locked.state.key.as_str().to_owned())
        .collect();
    assert_eq!(
        keys,
        vec![DB_HOST, PUBLIC_PORT],
        "the result is in key order"
    );
}

#[tokio::test]
async fn a_later_transaction_reads_the_committed_upserts() {
    let f = Fixture::create().await;
    let mut tx = f.begin(Isolation::ReadCommitted).await;
    tx.settings()
        .upsert(&key(DB_HOST), &value("db"))
        .await
        .unwrap();
    f.unit_of_work.commit(tx, Fixture::audit()).await.unwrap();
    let mut later = f.begin(Isolation::ReadCommitted).await;

    let rows = later.settings().lock_rows().await.unwrap();

    let found: Vec<&str> = rows.iter().map(|l| l.state.value.as_str()).collect();
    assert_eq!(found, vec!["db"]);
}

#[tokio::test]
async fn a_failed_statement_midway_keeps_every_earlier_write_out() {
    let mut f = Fixture::create().await;
    f.seed(&[(SEPARATION, "true")]).await;
    probe::install_on(
        &mut f.admin,
        "app_setting",
        "BEFORE",
        "INSERT",
        &format!("IF NEW.key = '{DB_NAME}' THEN {FAULT} END IF;"),
    )
    .await;
    let mut tx = f.begin(Isolation::ReadCommitted).await;
    tx.settings()
        .upsert(&key(SEPARATION), &value("false"))
        .await
        .unwrap();
    tx.settings()
        .upsert(&key(DB_HOST), &value("db"))
        .await
        .unwrap();

    let error = tx
        .settings()
        .upsert(&key(DB_NAME), &value("recipe"))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate, .. } if sqlstate.as_deref() == Some("P0001")),
        "{error:?}"
    );
    drop(tx);
    assert_eq!(f.stored_pairs().await, vec![pair(SEPARATION, "true")]);
}

#[tokio::test]
async fn a_failed_audit_insert_keeps_the_upserts_out() {
    let mut f = Fixture::create().await;
    f.seed(&[(SEPARATION, "true")]).await;
    install_trigger(&f.database, "BEFORE", FAULT).await;
    let mut tx = f.begin(Isolation::ReadCommitted).await;
    tx.settings()
        .upsert(&key(SEPARATION), &value("false"))
        .await
        .unwrap();
    tx.settings()
        .upsert(&key(DB_HOST), &value("db"))
        .await
        .unwrap();

    let error = f
        .unit_of_work
        .commit(tx, Fixture::audit())
        .await
        .unwrap_err();

    assert!(
        matches!(error, AppError::AuditWriteFailed { .. }),
        "{error:?}"
    );
    assert_eq!(f.stored_pairs().await, vec![pair(SEPARATION, "true")]);
}

#[tokio::test]
async fn an_upsert_of_a_state_key_is_refused_and_writes_no_row() {
    let mut f = Fixture::create().await;
    for name in STATE_SETTING_KEYS {
        let mut tx = f.begin(Isolation::ReadCommitted).await;

        let error = tx
            .settings()
            .upsert(&key(name), &value("x"))
            .await
            .unwrap_err();

        assert!(
            matches!(&error, StorageError::Other { sqlstate: None, source }
                if source.to_string()
                    == format!("the upsert of the table app_setting names the state key {name}")),
            "key: {name}, error: {error:?}"
        );
        f.unit_of_work.commit(tx, Fixture::audit()).await.unwrap();
        assert!(f.stored().await.is_empty(), "key: {name}");
    }
}

#[tokio::test]
async fn a_read_only_transaction_rejects_the_upsert() {
    let mut f = Fixture::create().await;
    let mut tx = f.begin(Isolation::ReadOnly).await;

    let error = tx
        .settings()
        .upsert(&key(DB_HOST), &value("db"))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate: Some(state), .. }
            if state == READ_ONLY_SQL_TRANSACTION),
        "{error:?}"
    );
    assert!(f.stored().await.is_empty());
}
