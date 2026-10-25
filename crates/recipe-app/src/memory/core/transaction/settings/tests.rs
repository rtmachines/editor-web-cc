//! Tests of the in-memory `SettingRepository` (task 3.2, `data-model.md` section 3.17).

use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::MachineId;
use recipe_domain::setting::{SettingId, SettingKey, SettingValue};
use recipe_domain::timestamp::Timestamp;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::{AppError, StorageError};
use crate::memory::core::state::StoredSetting;
use crate::memory::{Fault, MemoryStore};
use crate::ports::{Isolation, NonEmpty, STATE_SETTING_KEYS, Transaction, UnitOfWork};

const SEPARATION: &str = "approval.separation_of_duties";
const DB_HOST: &str = "db.host";
const PUBLIC_PORT: &str = "http.public_port";

fn key(text: &str) -> SettingKey {
    SettingKey::parse(text).unwrap()
}

fn value(text: &str) -> SettingValue {
    SettingValue::parse(text).unwrap()
}

fn entries() -> NonEmpty<AuditEntry> {
    let entry = AuditEntry::row(
        RowAction::MachineCreated,
        AuditTarget::machine(MachineId::from(Uuid::from_u128(1))),
        RowChange::Create {
            after: json!({ "n": "1" }),
        },
        AuditContext {
            actor: None,
            request_id: RequestId::from(Uuid::from_u128(1)),
            client_ip: None,
            session_ref: None,
        },
    );
    NonEmpty::new(entry)
}

/// Upsert `pairs` in one transaction and commit it.
async fn commit_pairs(store: &MemoryStore, pairs: &[(&str, &str)]) {
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    for (name, text) in pairs {
        tx.settings()
            .upsert(&key(name), &value(text))
            .await
            .unwrap();
    }
    let unit_of_work: &dyn UnitOfWork = store;
    unit_of_work.commit(Box::new(tx), entries()).await.unwrap();
}

/// Read the rows in a new transaction and return `(key, value)` pairs in the order of the port.
async fn stored_pairs(store: &MemoryStore) -> Vec<(String, String)> {
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let rows = tx.settings().lock_rows().await.unwrap();
    rows.into_iter()
        .map(|locked| {
            (
                locked.state.key.as_str().to_owned(),
                locked.state.value.as_str().to_owned(),
            )
        })
        .collect()
}

fn pair(name: &str, text: &str) -> (String, String) {
    (name.to_owned(), text.to_owned())
}

#[tokio::test]
async fn a_new_store_has_no_configuration_row() {
    let store = MemoryStore::default();
    assert!(stored_pairs(&store).await.is_empty());
}

#[tokio::test]
async fn an_upsert_inserts_a_missing_row_and_returns_the_image_after() {
    let store = MemoryStore::default();
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();

    let written = tx
        .settings()
        .upsert(&key(SEPARATION), &value("true"))
        .await
        .unwrap();

    let image = written.after.as_value();
    assert_eq!(image["id"], Value::String(written.id.to_string()));
    assert_eq!(image["key"], json!(SEPARATION));
    assert_eq!(image["value"], json!("true"));
    assert_eq!(image["updated_by"], Value::Null);
    assert!(image["created_at"].is_string());
    assert!(image["updated_at"].is_string());
}

#[tokio::test]
async fn an_upsert_of_an_existing_row_keeps_its_id_and_the_image_before_shows_the_old_value() {
    let store = MemoryStore::default();
    commit_pairs(&store, &[(SEPARATION, "true")]).await;
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let mut rows = tx.settings().lock_rows().await.unwrap();
    let locked = rows.pop().unwrap();

    let written = tx
        .settings()
        .upsert(&key(SEPARATION), &value("false"))
        .await
        .unwrap();

    assert_eq!(written.id, locked.state.id);
    assert_eq!(locked.before.as_value()["value"], json!("true"));
    assert_eq!(written.after.as_value()["value"], json!("false"));
    assert_eq!(
        written.after.as_value()["id"],
        locked.before.as_value()["id"]
    );
}

#[tokio::test]
async fn two_new_rows_get_two_different_ids() {
    let store = MemoryStore::default();
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();

    let first = tx
        .settings()
        .upsert(&key(DB_HOST), &value("db"))
        .await
        .unwrap();
    let second = tx
        .settings()
        .upsert(&key(PUBLIC_PORT), &value("8080"))
        .await
        .unwrap();

    assert_ne!(first.id, second.id);
}

#[tokio::test]
async fn the_rows_come_in_the_byte_order_of_the_keys() {
    let store = MemoryStore::default();
    commit_pairs(&store, &[(PUBLIC_PORT, "8080"), (DB_HOST, "db")]).await;

    let pairs = stored_pairs(&store).await;

    assert_eq!(pairs, vec![pair(DB_HOST, "db"), pair(PUBLIC_PORT, "8080")]);
}

/// Put the three state rows into the table and commit, as the memory `MaintenanceStore` does.
/// The write goes through `state_mut()`, because `upsert` refuses a state key.
async fn seed_state_rows(store: &MemoryStore) {
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let now = tx.now();
    for (offset, name) in STATE_SETTING_KEYS.iter().enumerate() {
        let serial = u128::try_from(offset).unwrap().checked_add(1000).unwrap();
        let row = StoredSetting {
            id: SettingId::from(Uuid::from_u128(serial)),
            value: value("x"),
            created_at: now,
            updated_at: now,
        };
        tx.state_mut().settings.insert(key(name), row);
    }
    let unit_of_work: &dyn UnitOfWork = store;
    unit_of_work.commit(Box::new(tx), entries()).await.unwrap();
}

#[tokio::test]
async fn the_rows_skip_the_state_keys() {
    let store = MemoryStore::default();
    seed_state_rows(&store).await;
    commit_pairs(&store, &[(DB_HOST, "db")]).await;

    let pairs = stored_pairs(&store).await;

    assert_eq!(pairs, vec![pair(DB_HOST, "db")]);
}

#[tokio::test]
async fn an_upsert_of_a_state_key_is_refused_and_writes_no_row() {
    for name in STATE_SETTING_KEYS {
        let store = MemoryStore::default();
        let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();

        let error = tx
            .settings()
            .upsert(&key(name), &value("x"))
            .await
            .unwrap_err();

        assert!(
            matches!(&error, StorageError::Other { sqlstate: None, .. }),
            "key: {name}, error: {error:?}"
        );
        assert_eq!(error.code(), ErrorCode::DB_008, "key: {name}");
        assert!(tx.state().settings.is_empty(), "key: {name}");
        assert_eq!(tx.state().last_setting_serial, 0, "key: {name}");
    }
}

#[tokio::test]
async fn the_lock_of_the_rows_goes_into_the_journal_in_the_order_of_the_ids() {
    use crate::memory::{JournalEntry, LockMode};
    let store = MemoryStore::default();
    // The first insert takes the id 1 and the second takes the id 2. The key order is the reverse.
    commit_pairs(&store, &[(PUBLIC_PORT, "8080"), (DB_HOST, "db")]).await;
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();

    let rows = tx.settings().lock_rows().await.unwrap();

    let locks: Vec<(String, LockMode)> = store
        .journal()
        .into_iter()
        .filter_map(|line| match line {
            JournalEntry::Lock(lock) if lock.table == "app_setting" => Some((lock.key, lock.mode)),
            _ => None,
        })
        .collect();
    let id = |serial: u128| Uuid::from_u128(serial).to_string();
    assert_eq!(
        locks,
        vec![
            (id(1), LockMode::ForNoKeyUpdate),
            (id(2), LockMode::ForNoKeyUpdate)
        ]
    );
    let keys: Vec<&str> = rows.iter().map(|l| l.state.key.as_str()).collect();
    assert_eq!(
        keys,
        vec![DB_HOST, PUBLIC_PORT],
        "the result stays in key order"
    );
}

#[tokio::test]
async fn a_committed_upsert_is_visible_to_the_next_transaction() {
    let store = MemoryStore::default();
    commit_pairs(&store, &[(SEPARATION, "true")]).await;
    commit_pairs(&store, &[(SEPARATION, "false")]).await;

    assert_eq!(stored_pairs(&store).await, vec![pair(SEPARATION, "false")]);
}

#[tokio::test]
async fn the_state_upsert_and_the_repository_share_the_row_rule_and_the_serial() {
    let store = MemoryStore::default();
    commit_pairs(&store, &[(DB_HOST, "db")]).await;
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    let before = tx.state().settings.get(&key(DB_HOST)).unwrap().clone();
    let later = Timestamp::from(
        tx.now()
            .as_offset_date_time()
            .checked_add(time::Duration::seconds(60))
            .unwrap(),
    );

    tx.state_mut()
        .upsert_setting(key(DB_HOST), value("other"), later)
        .unwrap();
    tx.state_mut()
        .upsert_setting(key(PUBLIC_PORT), value("8080"), later)
        .unwrap();

    let changed = tx.state().settings.get(&key(DB_HOST)).unwrap().clone();
    assert_eq!(changed.id, before.id);
    assert_eq!(changed.created_at, before.created_at);
    assert_eq!(changed.updated_at, later);
    assert_eq!(changed.value, value("other"));
    let inserted = tx.state().settings.get(&key(PUBLIC_PORT)).unwrap().clone();
    assert_eq!(inserted.id, SettingId::from(Uuid::from_u128(2)));
    assert_eq!((inserted.created_at, inserted.updated_at), (later, later));
    let written = tx
        .settings()
        .upsert(&key(PUBLIC_PORT), &value("9090"))
        .await
        .unwrap();
    assert_eq!(written.id, inserted.id);
    let third = tx
        .settings()
        .upsert(&key(SEPARATION), &value("true"))
        .await
        .unwrap();
    assert_eq!(third.id, SettingId::from(Uuid::from_u128(3)));
}

#[tokio::test]
async fn a_dropped_transaction_keeps_no_row() {
    let store = MemoryStore::default();
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    tx.settings()
        .upsert(&key(SEPARATION), &value("true"))
        .await
        .unwrap();

    drop(tx);

    assert!(stored_pairs(&store).await.is_empty());
}

#[tokio::test]
async fn a_failed_audit_write_keeps_no_row() {
    let store = MemoryStore::default();
    commit_pairs(&store, &[(SEPARATION, "true")]).await;
    store.fail_append_audit(Fault::Unavailable);
    let mut tx = store.begin_memory(Isolation::ReadCommitted).unwrap();
    tx.settings()
        .upsert(&key(SEPARATION), &value("false"))
        .await
        .unwrap();
    tx.settings()
        .upsert(&key(DB_HOST), &value("db"))
        .await
        .unwrap();

    let unit_of_work: &dyn UnitOfWork = &store;
    let error = unit_of_work
        .commit(Box::new(tx), entries())
        .await
        .unwrap_err();

    assert!(
        matches!(error, AppError::AuditWriteFailed { .. }),
        "{error:?}"
    );
    assert_eq!(stored_pairs(&store).await, vec![pair(SEPARATION, "true")]);
}
