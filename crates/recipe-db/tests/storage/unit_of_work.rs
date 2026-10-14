//! Tests of `PgUnitOfWork` and `PgTransaction` (task 8.5, design D3).
//!
//! Every test goes through the port `dyn UnitOfWork` of `recipe-app`, as a use case does. The
//! adapter connects as `recipe_app`. The tests read the audit tables as the superuser.

use std::error::Error as _;
use std::sync::Arc;

use recipe_app::error::{AppError, StorageError};
use recipe_app::ports::{Isolation, Transaction, UnitOfWork};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::error_codes::ErrorCode;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::audit::{audit_count, entries, entry, head_seq, install_trigger, stored_rows};
use crate::support::fixtures;
use crate::support::{Role, TestDatabase};

/// The constraint that the actor of an audit row must satisfy.
const ACTOR_FOREIGN_KEY: &str = "audit_log_actor_user_id_fkey";
/// The SQLSTATE of a write in a read-only transaction.
const READ_ONLY_SQL_TRANSACTION: &str = "25006";
/// The number of entries in the batch test (design D3).
const BATCH_SIZE: usize = 100;

/// A test database, a user in `app_user`, the unit of work, and a session of the superuser.
struct Fixture {
    database: TestDatabase,
    user: Uuid,
    unit_of_work: Arc<dyn UnitOfWork>,
    admin: PgConnection,
}

impl Fixture {
    async fn create() -> Self {
        let database = TestDatabase::create().await;
        let mut migrator = database.connect(Role::Migrator).await;
        let user = fixtures::user(&mut migrator).await;
        let connected = Database::connect(&database.target(Role::App), &PoolSettings::default())
            .await
            .unwrap();
        let admin = database.connect(Role::Superuser).await;
        Self {
            database,
            user,
            unit_of_work: connected.unit_of_work(),
            admin,
        }
    }

    async fn begin(&self, isolation: Isolation) -> Box<dyn Transaction> {
        self.unit_of_work.begin(isolation).await.unwrap()
    }
}

#[tokio::test]
async fn a_commit_keeps_100_entries_in_the_order_of_the_list() {
    let mut f = Fixture::create().await;
    let (list, machines) = entries(Some(f.user), BATCH_SIZE);
    let tx = f.begin(Isolation::ReadCommitted).await;

    f.unit_of_work.commit(tx, list).await.unwrap();

    let rows = stored_rows(&mut f.admin).await;
    let sequences: Vec<i64> = rows.iter().map(|row| row.chain_seq).collect();
    let stored_machines: Vec<Uuid> = rows.iter().map(|row| row.entity_id).collect();
    let expected: Vec<i64> = (1..=i64::try_from(BATCH_SIZE).unwrap()).collect();
    assert_eq!(sequences, expected);
    assert_eq!(stored_machines, machines);
}

#[tokio::test]
async fn a_commit_stores_every_column_of_an_entry() {
    let mut f = Fixture::create().await;
    let machine = Uuid::new_v4();
    let source = entry(Some(f.user), machine);
    let tx = f.begin(Isolation::ReadCommitted).await;

    f.unit_of_work
        .commit(tx, recipe_app::ports::NonEmpty::new(source.clone()))
        .await
        .unwrap();

    let rows = stored_rows(&mut f.admin).await;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.actor_user_id, Some(f.user));
    assert_eq!(row.action, "MachineCreated");
    assert_eq!(row.entity_type, "machine");
    assert_eq!(row.entity_id, machine);
    assert_eq!(row.aggregate_type, "machine");
    assert_eq!(row.aggregate_id, machine);
    assert_eq!(row.before_state, None);
    assert_eq!(row.after_state.as_ref(), source.after());
    assert_eq!(row.request_id, *source.request_id().as_uuid());
    assert_eq!(row.client_ip.as_deref(), Some("10.0.0.7"));
    assert_eq!(row.session_ref.as_deref(), Some("0123456789abcdef"));
}

#[tokio::test]
async fn an_entry_without_actor_address_and_session_is_stored_with_nulls() {
    let mut f = Fixture::create().await;
    let bare = bare_entry();
    let tx = f.begin(Isolation::ReadCommitted).await;

    f.unit_of_work
        .commit(tx, recipe_app::ports::NonEmpty::new(bare))
        .await
        .unwrap();

    let rows = stored_rows(&mut f.admin).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].actor_user_id, None);
    assert_eq!(rows[0].client_ip, None);
    assert_eq!(rows[0].session_ref, None);
}

/// An entry without actor, address, and session reference.
fn bare_entry() -> recipe_domain::audit::AuditEntry {
    use recipe_domain::audit::{
        AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
    };
    use recipe_domain::plant::MachineId;
    AuditEntry::row(
        RowAction::MachineCreated,
        AuditTarget::machine(MachineId::from(Uuid::new_v4())),
        RowChange::Create {
            after: serde_json::json!({}),
        },
        AuditContext {
            actor: None,
            request_id: RequestId::from(Uuid::new_v4()),
            client_ip: None,
            session_ref: None,
        },
    )
}

#[tokio::test]
async fn a_drop_without_finish_keeps_nothing() {
    let mut f = Fixture::create().await;
    let (list, _) = entries(Some(f.user), 3);
    let mut tx = f.begin(Isolation::ReadCommitted).await;
    tx.append_audit(&list).await.unwrap();

    drop(tx);

    assert_eq!(audit_count(&mut f.admin).await, 0);
    assert_eq!(head_seq(&mut f.admin).await, 0);
}

#[tokio::test]
async fn a_read_only_transaction_rejects_the_audit_insert() {
    let f = Fixture::create().await;
    let (list, _) = entries(Some(f.user), 1);
    let mut tx = f.begin(Isolation::ReadOnly).await;

    let error = tx.append_audit(&list).await.unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate: Some(state), .. }
            if state == READ_ONLY_SQL_TRANSACTION),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_008);
}

#[tokio::test]
async fn begin_sets_the_isolation_level_and_the_access_mode() {
    let f = Fixture::create().await;
    install_trigger(
        &f.database,
        "BEFORE",
        "RAISE EXCEPTION 'level=% read_only=%', \
             current_setting('transaction_isolation'), current_setting('transaction_read_only');",
    )
    .await;
    let (list, _) = entries(Some(f.user), 1);
    let expected = [
        (
            Isolation::ReadCommitted,
            "level=read committed read_only=off",
        ),
        (Isolation::Serializable, "level=serializable read_only=off"),
    ];
    for (isolation, text) in expected {
        let mut tx = f.begin(isolation).await;

        let error = tx.append_audit(&list).await.unwrap_err();

        let cause = error.source().unwrap().to_string();
        assert!(cause.contains(text), "{isolation:?}: {cause}");
    }
}

#[tokio::test]
async fn a_failed_audit_insert_gives_audit_write_failed_and_keeps_nothing() {
    let mut f = Fixture::create().await;
    // The trigger fires after the hash trigger moved the head, so the rollback must undo both.
    install_trigger(
        &f.database,
        "AFTER",
        "IF NEW.chain_seq = 2 THEN RAISE EXCEPTION 'test failure of the audit insert'; END IF;",
    )
    .await;
    let (list, _) = entries(Some(f.user), 3);
    let tx = f.begin(Isolation::ReadCommitted).await;

    let error = f.unit_of_work.commit(tx, list).await.unwrap_err();

    assert!(
        matches!(&error, AppError::AuditWriteFailed { .. }),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::AUD_001);
    assert_eq!(audit_count(&mut f.admin).await, 0);
    assert_eq!(head_seq(&mut f.admin).await, 0);
}

#[tokio::test]
async fn an_actor_that_does_not_exist_fails_and_keeps_nothing() {
    let mut f = Fixture::create().await;
    let (list, _) = entries(Some(Uuid::new_v4()), 2);
    let tx = f.begin(Isolation::ReadCommitted).await;

    let error = f.unit_of_work.commit(tx, list).await.unwrap_err();

    assert!(
        matches!(&error, AppError::AuditWriteFailed {
            cause: StorageError::MissingReference { constraint } } if constraint == ACTOR_FOREIGN_KEY),
        "{error:?}"
    );
    assert_eq!(audit_count(&mut f.admin).await, 0);
    assert_eq!(head_seq(&mut f.admin).await, 0);
}

#[tokio::test]
async fn two_commits_continue_the_chain_without_a_gap() {
    let mut f = Fixture::create().await;
    for _ in 0..2 {
        let (list, _) = entries(Some(f.user), 2);
        let tx = f.begin(Isolation::ReadCommitted).await;
        f.unit_of_work.commit(tx, list).await.unwrap();
    }

    let sequences: Vec<i64> = stored_rows(&mut f.admin)
        .await
        .iter()
        .map(|row| row.chain_seq)
        .collect();

    assert_eq!(sequences, vec![1, 2, 3, 4]);
}
