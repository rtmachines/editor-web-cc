//! Tests of `CompareSettings` (task 3.9, design D14 row T15, ADR-0079, `reliability.md` 7.12).

use std::sync::Arc;
use std::time::Duration;

use recipe_domain::audit::{AuditAction, AuditEntry, RequestId};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::setting::{SettingKey, SettingValue};
use serde_json::Value;
use uuid::Uuid;

use super::{CompareSettings, ConfiguredSettings, WebhookUrlText};
use crate::error::AppError;
use crate::memory::{Fault, FaultPoint, JournalEntry, LockMode, MemoryStore};
use crate::ports::{Isolation, STATE_SETTING_KEYS};

const URL: &str = "https://hooks.example.test/services/T0/B0/very-secret-token";
const URL_KEY: &str = "alerts.webhook_url";
const SEPARATION: &str = "approval.separation_of_duties";

/// The table of design D7 for `sample()`, written by hand. The webhook row is checked apart.
const EXPECTED: [(&str, &str); 17] = [
    ("db.host", "db"),
    ("db.port", "5432"),
    ("db.name", "recipe"),
    ("db.app_user", "recipe_app"),
    ("secrets.app_password_file", "/run/secrets/app_password"),
    (
        "security.login_hmac_key_file",
        "/run/secrets/login_hmac_key",
    ),
    (
        "security.csrf_signing_key_file",
        "/run/secrets/csrf_signing_key",
    ),
    ("http.trusted_proxies", "172.28.0.2/32,10.0.0.0/8"),
    ("http.public_port", "8080"),
    ("http.internal_port", "8081"),
    (SEPARATION, "true"),
    ("alerts.webhook_url_file", "/run/secrets/alerts_webhook_url"),
    ("alerts.webhook_ca_file", ""),
    ("session.secure", "true"),
    ("session.idle_timeout", "1800s"),
    ("session.absolute_timeout", "28800s"),
    ("i18n.translation_langs", "en,de"),
];

fn sample() -> ConfiguredSettings {
    ConfiguredSettings {
        db_host: "db".to_owned(),
        db_port: 5432,
        db_name: "recipe".to_owned(),
        db_app_user: "recipe_app".to_owned(),
        app_password_file: "/run/secrets/app_password".to_owned(),
        login_hmac_key_file: "/run/secrets/login_hmac_key".to_owned(),
        csrf_signing_key_file: "/run/secrets/csrf_signing_key".to_owned(),
        trusted_proxies: vec!["172.28.0.2/32".to_owned(), "10.0.0.0/8".to_owned()],
        public_port: 8080,
        internal_port: 8081,
        separation_of_duties: true,
        webhook_url_file: Some("/run/secrets/alerts_webhook_url".to_owned()),
        webhook_ca_file: None,
        webhook_url: Some(WebhookUrlText::from(URL.to_owned())),
        session_secure: true,
        idle_timeout: Duration::from_secs(1800),
        absolute_timeout: Duration::from_secs(28_800),
        translation_langs: vec!["en".to_owned(), "de".to_owned()],
    }
}

fn request_id() -> RequestId {
    RequestId::from(Uuid::from_u128(0x51))
}

fn key(text: &str) -> SettingKey {
    SettingKey::parse(text).unwrap()
}

fn value(text: &str) -> SettingValue {
    SettingValue::parse(text).unwrap()
}

async fn run(store: &MemoryStore, configured: &ConfiguredSettings) -> Result<usize, AppError> {
    CompareSettings::new(Arc::new(store.clone()))
        .run(configured, request_id())
        .await
}

/// The stored `(key, value)` pairs in the byte order of the keys.
fn stored(store: &MemoryStore) -> Vec<(String, String)> {
    store
        .state()
        .settings
        .iter()
        .map(|(name, row)| (name.to_string(), row.value.to_string()))
        .collect()
}

fn stored_value(store: &MemoryStore, name: &str) -> Option<String> {
    stored(store)
        .into_iter()
        .find(|(stored_name, _)| stored_name == name)
        .map(|(_, text)| text)
}

fn setting_entries(store: &MemoryStore) -> Vec<AuditEntry> {
    store
        .audit_entries()
        .into_iter()
        .filter(|entry| entry.action() == AuditAction::SettingChanged)
        .collect()
}

fn text_of<'a>(image: &'a Value, field: &str) -> &'a str {
    image.get(field).and_then(Value::as_str).unwrap()
}

/// Run `sample()` once, so that every key has a row.
async fn started(store: &MemoryStore) {
    assert_eq!(run(store, &sample()).await.unwrap(), 18);
}

#[test]
fn the_canonical_text_of_the_18_keys_equals_the_table_of_d7() {
    let fingerprint = SettingValue::fingerprint(URL);
    assert!(fingerprint.as_str().starts_with("sha256:"));
    assert_eq!(fingerprint.as_str().len(), 71);
    let mut expected: Vec<(String, String)> = EXPECTED
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect();
    expected.insert(13, (URL_KEY.to_owned(), fingerprint.to_string()));
    let pairs: Vec<(String, String)> = sample()
        .canonical()
        .unwrap()
        .into_iter()
        .map(|(name, text)| (name.to_string(), text.to_string()))
        .collect();
    assert_eq!(pairs, expected);
}

#[test]
fn an_absent_file_an_absent_webhook_and_an_empty_list_give_the_empty_text() {
    let configured = ConfiguredSettings {
        webhook_url_file: None,
        webhook_url: None,
        trusted_proxies: Vec::new(),
        translation_langs: Vec::new(),
        ..sample()
    };
    let pairs = configured.canonical().unwrap();
    for name in [
        "alerts.webhook_url_file",
        "alerts.webhook_ca_file",
        URL_KEY,
        "http.trusted_proxies",
        "i18n.translation_langs",
    ] {
        let found = pairs.iter().find(|(pair_key, _)| pair_key.as_str() == name);
        assert_eq!(found.map(|(_, text)| text.as_str()), Some(""), "{name}");
    }
}

#[test]
fn the_debug_text_of_the_settings_holds_no_part_of_the_url() {
    let text = format!("{:?}", sample());
    assert!(!text.contains("hooks.example"));
    assert!(!text.contains("secret-token"));
    assert!(!format!("{:?}", WebhookUrlText::from(URL.to_owned())).contains("example"));
}

#[tokio::test]
async fn a_value_over_the_limit_is_refused_before_a_transaction_opens() {
    let store = MemoryStore::default();
    let configured = ConfiguredSettings {
        trusted_proxies: vec!["x".repeat(1001)],
        ..sample()
    };
    let error = run(&store, &configured).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(store.journal().is_empty());
    assert!(stored(&store).is_empty());
}

#[tokio::test]
async fn on_an_empty_table_every_key_gets_a_row_and_an_entry_without_old_state() {
    let store = MemoryStore::default();
    started(&store).await;
    assert_eq!(stored(&store).len(), 18);
    let entries = setting_entries(&store);
    assert_eq!(entries.len(), 18);
    for entry in &entries {
        assert!(entry.before().is_none());
        assert!(entry.actor().is_none());
        assert_eq!(entry.request_id(), request_id());
        assert!(entry.client_ip().is_none());
        assert!(entry.session_ref().is_none());
    }
    assert_eq!(stored_value(&store, "db.port").as_deref(), Some("5432"));
}

#[tokio::test]
async fn unchanged_values_write_no_row_and_no_entry() {
    let store = MemoryStore::default();
    started(&store).await;
    let rows = store.state().settings;
    let entries = store.audit_entries();
    store.clock().advance(time::Duration::hours(1));
    assert_eq!(run(&store, &sample()).await.unwrap(), 0);
    assert_eq!(store.state().settings, rows);
    assert_eq!(store.audit_entries(), entries);
}

#[tokio::test]
async fn one_changed_value_writes_one_row_and_one_entry_with_both_full_rows() {
    let store = MemoryStore::default();
    started(&store).await;
    let configured = ConfiguredSettings {
        separation_of_duties: false,
        ..sample()
    };
    assert_eq!(run(&store, &configured).await.unwrap(), 1);
    assert_eq!(stored_value(&store, SEPARATION).as_deref(), Some("false"));
    let entries = setting_entries(&store);
    assert_eq!(entries.len(), 19);
    let entry = entries.last().unwrap();
    let (before, after) = (entry.before().unwrap(), entry.after().unwrap());
    assert_eq!(text_of(before, "key"), SEPARATION);
    assert_eq!(text_of(before, "value"), "true");
    assert_eq!(text_of(after, "value"), "false");
    assert_eq!(text_of(before, "id"), text_of(after, "id"));
    for image in [before, after] {
        for column in [
            "id",
            "key",
            "value",
            "updated_by",
            "created_at",
            "updated_at",
        ] {
            assert!(image.get(column).is_some(), "{column}");
        }
    }
    assert_eq!(after.get("updated_by"), Some(&Value::Null));
}

#[tokio::test]
async fn a_missing_row_gives_an_entry_without_old_state_and_the_row_returns() {
    let store = MemoryStore::default();
    started(&store).await;
    store.update_state(|state, _| state.settings.remove(&key("db.name")));
    assert_eq!(stored(&store).len(), 17);
    assert_eq!(run(&store, &sample()).await.unwrap(), 1);
    assert_eq!(stored_value(&store, "db.name").as_deref(), Some("recipe"));
    let entries = setting_entries(&store);
    let entry = entries.last().unwrap();
    assert_eq!(entries.len(), 19);
    assert!(entry.before().is_none());
    assert_eq!(text_of(entry.after().unwrap(), "key"), "db.name");
    assert_eq!(text_of(entry.after().unwrap(), "value"), "recipe");
}

#[tokio::test]
async fn the_comparison_takes_no_read_only_input_and_commits_a_writable_transaction() {
    // `CompareSettings` has no switch and no flag as input (ADR-0061 (e)). Read-only mode can
    // reach it only as a transaction that refuses to commit, so this test needs a writable one.
    let store = MemoryStore::default();
    started(&store).await;
    let configured = ConfiguredSettings {
        public_port: 9090,
        ..sample()
    };
    let journal_before = store.journal().len();
    assert_eq!(run(&store, &configured).await.unwrap(), 1);
    assert_eq!(
        stored_value(&store, "http.public_port").as_deref(),
        Some("9090")
    );
    assert_eq!(setting_entries(&store).len(), 19);
    let journal = store.journal();
    let begins: Vec<_> = journal
        .get(journal_before..)
        .unwrap()
        .iter()
        .filter(|line| matches!(line, JournalEntry::Begin(_)))
        .collect();
    assert_eq!(begins, [&JournalEntry::Begin(Isolation::ReadCommitted)]);
}

#[tokio::test]
async fn the_table_and_the_entries_hold_the_fingerprint_and_never_the_url() {
    let store = MemoryStore::default();
    started(&store).await;
    let fingerprint = SettingValue::fingerprint(URL);
    assert_eq!(
        stored_value(&store, URL_KEY).as_deref(),
        Some(fingerprint.as_str())
    );
    let dump = format!("{:?}{:?}", store.state().settings, store.audit_entries());
    assert!(!dump.contains("hooks.example"));
    assert!(!dump.contains("secret-token"));
    assert!(dump.contains(fingerprint.as_str()));
}

#[tokio::test]
async fn a_changed_url_changes_the_fingerprint_and_gives_one_entry() {
    let store = MemoryStore::default();
    started(&store).await;
    let other = "https://hooks.example.test/services/T1/B1/other-token";
    let configured = ConfiguredSettings {
        webhook_url: Some(WebhookUrlText::from(other.to_owned())),
        ..sample()
    };
    assert_eq!(run(&store, &configured).await.unwrap(), 1);
    let entries = setting_entries(&store);
    let entry = entries.last().unwrap();
    let old = SettingValue::fingerprint(URL);
    let new = SettingValue::fingerprint(other);
    assert_ne!(old, new);
    assert_eq!(text_of(entry.before().unwrap(), "value"), old.as_str());
    assert_eq!(text_of(entry.after().unwrap(), "value"), new.as_str());
    assert!(!format!("{entries:?}").contains("other-token"));
}

#[tokio::test]
async fn a_removed_webhook_stores_the_empty_text() {
    let store = MemoryStore::default();
    started(&store).await;
    let configured = ConfiguredSettings {
        webhook_url: None,
        ..sample()
    };
    assert_eq!(run(&store, &configured).await.unwrap(), 1);
    assert_eq!(stored_value(&store, URL_KEY).as_deref(), Some(""));
}

#[tokio::test]
async fn an_empty_url_text_stores_the_empty_text() {
    let store = MemoryStore::default();
    started(&store).await;
    let configured = ConfiguredSettings {
        webhook_url: Some(WebhookUrlText::from(String::new())),
        ..sample()
    };
    assert_eq!(run(&store, &configured).await.unwrap(), 1);
    assert_eq!(stored_value(&store, URL_KEY).as_deref(), Some(""));
}

#[tokio::test]
async fn an_empty_url_text_against_a_stored_empty_row_writes_nothing() {
    let store = MemoryStore::default();
    started(&store).await;
    let removed = ConfiguredSettings {
        webhook_url: None,
        ..sample()
    };
    assert_eq!(run(&store, &removed).await.unwrap(), 1);
    let rows = store.state().settings;
    let entries = store.audit_entries();
    let empty_text = ConfiguredSettings {
        webhook_url: Some(WebhookUrlText::from(String::new())),
        ..sample()
    };
    assert_eq!(run(&store, &empty_text).await.unwrap(), 0);
    assert_eq!(store.state().settings, rows);
    assert_eq!(store.audit_entries(), entries);
}

#[tokio::test]
async fn a_url_text_of_only_spaces_keeps_its_fingerprint() {
    let store = MemoryStore::default();
    let configured = ConfiguredSettings {
        webhook_url: Some(WebhookUrlText::from("  ".to_owned())),
        ..sample()
    };
    assert_eq!(run(&store, &configured).await.unwrap(), 18);
    let fingerprint = SettingValue::fingerprint("  ");
    assert_eq!(
        stored_value(&store, URL_KEY).as_deref(),
        Some(fingerprint.as_str())
    );
}

#[tokio::test]
async fn a_failed_commit_on_an_empty_table_writes_no_row_and_no_entry() {
    let store = MemoryStore::default();
    store.fail_next(FaultPoint::Finish, Fault::Unavailable, 1);
    let error = run(&store, &sample()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert!(stored(&store).is_empty());
    assert!(store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_failed_commit_keeps_the_old_values_and_the_old_entries() {
    let store = MemoryStore::default();
    started(&store).await;
    let before = store.state().settings;
    let entries = store.audit_entries();
    store.fail_next(FaultPoint::Finish, Fault::Unavailable, 1);
    let configured = ConfiguredSettings {
        separation_of_duties: false,
        db_port: 5433,
        ..sample()
    };
    assert!(run(&store, &configured).await.is_err());
    assert_eq!(store.state().settings, before);
    assert_eq!(store.audit_entries(), entries);
    assert_eq!(run(&store, &configured).await.unwrap(), 2);
}

#[tokio::test]
async fn a_failed_audit_insert_gives_e_aud_001_and_writes_nothing() {
    let store = MemoryStore::default();
    store.fail_next(FaultPoint::AppendAudit, Fault::Unavailable, 1);
    let error = run(&store, &sample()).await.unwrap_err();
    assert!(matches!(error, AppError::AuditWriteFailed { .. }));
    assert_eq!(error.code(), ErrorCode::AUD_001);
    assert!(stored(&store).is_empty());
    assert!(store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_state_keys_stay_unchanged_and_get_no_entry() {
    let store = MemoryStore::default();
    let state_key = key(STATE_SETTING_KEYS.first().unwrap());
    store
        .update_state(|state, now| state.put_setting(&state_key, &value("42"), now))
        .unwrap();
    started(&store).await;
    assert_eq!(
        stored_value(&store, state_key.as_str()).as_deref(),
        Some("42")
    );
    assert_eq!(stored(&store).len(), 19);
    assert!(
        setting_entries(&store)
            .iter()
            .all(|entry| text_of(entry.after().unwrap(), "key") != state_key.as_str())
    );
}

#[tokio::test]
async fn one_read_committed_transaction_locks_the_rows_before_any_write() {
    let store = MemoryStore::default();
    started(&store).await;
    let configured = ConfiguredSettings {
        db_port: 5433,
        ..sample()
    };
    let journal_before = store.journal().len();
    assert_eq!(run(&store, &configured).await.unwrap(), 1);
    let journal = store.journal();
    let this_run = journal.get(journal_before..).unwrap();
    assert_eq!(
        this_run.first(),
        Some(&JournalEntry::Begin(Isolation::ReadCommitted))
    );
    let locks: Vec<_> = this_run
        .iter()
        .filter_map(|line| match line {
            JournalEntry::Lock(lock) => Some(lock),
            JournalEntry::Begin(_) => None,
        })
        .collect();
    assert_eq!(locks.len(), 18);
    assert!(
        locks
            .iter()
            .all(|lock| lock.table == "app_setting" && lock.mode == LockMode::ForNoKeyUpdate)
    );
    assert_eq!(this_run.len(), 19);
}
