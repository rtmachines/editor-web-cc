//! Tests of the in-memory `MaintenanceStore` (task 3.10, design D6, test rows T5 and T14).

use std::time::Duration;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::setting::SettingKey;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::{MaintenanceMethod, MemoryMaintenance, SECURITY_PURGE_BATCH};
use crate::memory::testkit::{IDLE_SECONDS, at, ip, new_session, set_time, store};
use crate::memory::{Fault, JournalEntry, MemoryStore};
use crate::ports::{ChainSeq, Isolation, MaintenanceStore as _, StateValue};

const HALF_DAY: Duration = Duration::from_secs(12 * 60 * 60);

fn maintenance() -> (MemoryStore, MemoryMaintenance) {
    let store = store();
    let adapter = MemoryMaintenance::new(store.clone());
    (store, adapter)
}

fn seed_old_attempts(store: &MemoryStore, count: u32) {
    let login = recipe_domain::identity::TypedLogin::parse("anna").unwrap();
    for _ in 0..count {
        store.seed_attempt(&login, ip(), at(0), false);
    }
}

fn setting_text(store: &MemoryStore, key: &str) -> Option<String> {
    let key = SettingKey::parse(key).unwrap();
    store
        .state()
        .settings
        .get(&key)
        .map(|row| row.value.to_string())
}

#[test]
fn the_batch_of_the_security_event_purge_equals_section_7_11() {
    assert_eq!(SECURITY_PURGE_BATCH, 5_000);
}

#[tokio::test]
async fn an_idle_expired_session_is_deleted_and_a_live_session_stays() {
    let (store, adapter) = maintenance();
    store.seed_session(&new_session(1));
    set_time(&store, 1_000);
    store.seed_session(&new_session(2));
    set_time(&store, 2_000);
    let deleted = adapter.delete_expired_sessions(HALF_DAY).await.unwrap();
    assert_eq!(deleted, 1);
    assert_eq!(store.state().sessions.len(), 1);
}

#[tokio::test]
async fn a_session_older_than_the_absolute_timeout_is_deleted_although_it_is_not_idle() {
    let (store, adapter) = maintenance();
    store.seed_session(&new_session(1));
    set_time(&store, 1_000);
    let deleted = adapter
        .delete_expired_sessions(Duration::from_secs(600))
        .await
        .unwrap();
    assert_eq!(deleted, 1);
    assert!(store.state().sessions.is_empty());
}

/// Move the clock of `store` to `seconds` after the start of the test, plus one microsecond.
fn set_time_plus_one_microsecond(store: &MemoryStore, seconds: i64) {
    let instant = at(seconds).as_offset_date_time().to_owned();
    let later = instant
        .checked_add(time::Duration::microseconds(1))
        .unwrap();
    store.clock().set(Timestamp::from(later));
}

#[tokio::test]
async fn a_session_that_expires_exactly_now_stays() {
    let (store, adapter) = maintenance();
    store.seed_session(&new_session(1));
    let idle = i64::try_from(IDLE_SECONDS).unwrap();
    set_time(&store, idle);
    assert_eq!(adapter.delete_expired_sessions(HALF_DAY).await.unwrap(), 0);
    assert_eq!(store.state().sessions.len(), 1);
}

#[tokio::test]
async fn a_session_created_exactly_one_absolute_timeout_ago_stays() {
    let (store, adapter) = maintenance();
    store.seed_session(&new_session(1));
    set_time(&store, 600);
    let deleted = adapter
        .delete_expired_sessions(Duration::from_secs(600))
        .await
        .unwrap();
    assert_eq!(deleted, 0);
    assert_eq!(store.state().sessions.len(), 1);
}

#[tokio::test]
async fn a_session_one_microsecond_past_its_idle_expiry_is_deleted() {
    let (store, adapter) = maintenance();
    store.seed_session(&new_session(1));
    set_time_plus_one_microsecond(&store, i64::try_from(IDLE_SECONDS).unwrap());
    assert_eq!(adapter.delete_expired_sessions(HALF_DAY).await.unwrap(), 1);
    assert!(store.state().sessions.is_empty());
}

#[tokio::test]
async fn a_session_one_microsecond_past_its_absolute_timeout_is_deleted() {
    let (store, adapter) = maintenance();
    store.seed_session(&new_session(1));
    set_time_plus_one_microsecond(&store, 600);
    let deleted = adapter
        .delete_expired_sessions(Duration::from_secs(600))
        .await
        .unwrap();
    assert_eq!(deleted, 1);
    assert!(store.state().sessions.is_empty());
}

#[tokio::test]
async fn deleting_with_no_expired_session_returns_zero() {
    let (store, adapter) = maintenance();
    store.seed_session(&new_session(1));
    assert_eq!(adapter.delete_expired_sessions(HALF_DAY).await.unwrap(), 0);
    assert_eq!(store.state().sessions.len(), 1);
}

#[tokio::test]
async fn login_attempts_older_than_the_retention_go_in_batches_of_the_argument() {
    let (store, adapter) = maintenance();
    seed_old_attempts(&store, 5);
    let login = recipe_domain::identity::TypedLogin::parse("bruno").unwrap();
    store.seed_attempt(&login, ip(), at(100_000), false);
    set_time(&store, 100_000);
    let retention = Duration::from_secs(50_000);
    let mut batches = Vec::new();
    for _ in 0..4 {
        batches.push(adapter.purge_login_attempts(retention, 2).await.unwrap());
    }
    assert_eq!(batches, [2, 2, 1, 0]);
    assert_eq!(store.state().attempts.len(), 1);
}

#[tokio::test]
async fn the_oldest_login_attempts_go_first() {
    let (store, adapter) = maintenance();
    let login = recipe_domain::identity::TypedLogin::parse("anna").unwrap();
    let oldest = store.seed_attempt(&login, ip(), at(0), false);
    let newer = store.seed_attempt(&login, ip(), at(10), false);
    set_time(&store, 100_000);
    let retention = Duration::from_secs(50_000);
    assert_eq!(adapter.purge_login_attempts(retention, 1).await.unwrap(), 1);
    let state = store.state();
    assert!(!state.attempts.contains_key(&oldest));
    assert!(state.attempts.contains_key(&newer));
}

#[tokio::test]
async fn the_security_event_purge_deletes_batches_of_5000() {
    let (_store, adapter) = maintenance();
    adapter.seed_expired_security_events(12_000);
    let mut batches = Vec::new();
    for _ in 0..4 {
        batches.push(adapter.purge_security_events().await.unwrap());
    }
    assert_eq!(batches, [5_000, 5_000, 2_000, 0]);
    assert_eq!(adapter.expired_security_events(), 0);
}

#[tokio::test]
async fn record_state_writes_the_text_form_of_each_key() {
    let (store, adapter) = maintenance();
    let instant = Timestamp::from(OffsetDateTime::from_unix_timestamp(1_000_000_000).unwrap());
    adapter
        .record_state(&StateValue::AuditLastCheckSeq(ChainSeq::from(12_345)))
        .await
        .unwrap();
    adapter
        .record_state(&StateValue::AuditLastCheckAt(instant))
        .await
        .unwrap();
    adapter
        .record_state(&StateValue::HeartbeatLastAt(instant))
        .await
        .unwrap();
    let text = |key| setting_text(&store, key).unwrap();
    assert_eq!(text("audit.last_check_seq"), "12345");
    assert_eq!(text("audit.last_check_at"), "2001-09-09T01:46:40.000000Z");
    assert_eq!(text("heartbeat.last_at"), "2001-09-09T01:46:40.000000Z");
}

#[tokio::test]
async fn record_state_writes_no_audit_entry() {
    let (store, adapter) = maintenance();
    adapter
        .record_state(&StateValue::HeartbeatLastAt(at(0)))
        .await
        .unwrap();
    assert!(store.audit_entries().is_empty());
    assert_eq!(
        store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadCommitted)]
    );
}

#[tokio::test]
async fn a_second_record_state_updates_the_row_of_the_first() {
    let (store, adapter) = maintenance();
    adapter
        .record_state(&StateValue::HeartbeatLastAt(at(0)))
        .await
        .unwrap();
    let key = SettingKey::parse("heartbeat.last_at").unwrap();
    let first = store.state().settings.get(&key).unwrap().clone();
    set_time(&store, 60);
    adapter
        .record_state(&StateValue::HeartbeatLastAt(at(60)))
        .await
        .unwrap();
    let state = store.state();
    let second = state.settings.get(&key).unwrap();
    assert_eq!(state.settings.len(), 1);
    assert_eq!(second.id, first.id);
    assert_eq!(second.created_at, at(0));
    assert_eq!(second.updated_at, at(60));
    assert_ne!(second.value, first.value);
}

#[tokio::test]
async fn the_three_state_keys_get_three_rows() {
    let (store, adapter) = maintenance();
    adapter
        .record_state(&StateValue::AuditLastCheckSeq(ChainSeq::ZERO))
        .await
        .unwrap();
    adapter
        .record_state(&StateValue::AuditLastCheckAt(at(0)))
        .await
        .unwrap();
    adapter
        .record_state(&StateValue::HeartbeatLastAt(at(0)))
        .await
        .unwrap();
    assert_eq!(store.state().settings.len(), 3);
}

#[tokio::test]
async fn a_negative_sequence_is_a_storage_error_and_writes_nothing() {
    let (store, adapter) = maintenance();
    let error = adapter
        .record_state(&StateValue::AuditLastCheckSeq(ChainSeq::from(-1)))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(store.state().settings.is_empty());
}

#[tokio::test]
async fn a_time_without_a_utc_form_is_a_storage_error_and_writes_nothing() {
    let (store, adapter) = maintenance();
    let offset = time::UtcOffset::from_hms(-2, 0, 0).unwrap();
    let last = time::PrimitiveDateTime::new(time::Date::MAX, time::Time::MAX).assume_offset(offset);
    let error = adapter
        .record_state(&StateValue::HeartbeatLastAt(Timestamp::from(last)))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(store.state().settings.is_empty());
}

#[tokio::test]
async fn a_fault_on_the_second_batch_keeps_the_first_batch_deleted() {
    let (store, adapter) = maintenance();
    seed_old_attempts(&store, 3);
    set_time(&store, 100_000);
    adapter.fail_on_call(MaintenanceMethod::PurgeLoginAttempts, 2, Fault::Unavailable);
    let retention = Duration::from_secs(50_000);
    assert_eq!(adapter.purge_login_attempts(retention, 1).await.unwrap(), 1);
    let error = adapter
        .purge_login_attempts(retention, 1)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert_eq!(store.state().attempts.len(), 2);
    assert_eq!(adapter.purge_login_attempts(retention, 1).await.unwrap(), 1);
    assert_eq!(store.state().attempts.len(), 1);
}

#[tokio::test]
async fn fail_from_call_fails_every_later_call() {
    let (store, adapter) = maintenance();
    seed_old_attempts(&store, 3);
    set_time(&store, 100_000);
    adapter.fail_from_call(MaintenanceMethod::PurgeLoginAttempts, 2, Fault::Deadlock);
    let retention = Duration::from_secs(50_000);
    adapter.purge_login_attempts(retention, 1).await.unwrap();
    for _ in 0..2 {
        let error = adapter
            .purge_login_attempts(retention, 1)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::DB_001);
    }
}

#[tokio::test]
async fn every_method_honours_its_fault_and_changes_nothing() {
    let (store, adapter) = maintenance();
    store.seed_session(&new_session(1));
    seed_old_attempts(&store, 1);
    adapter.seed_expired_security_events(10);
    set_time(&store, 100_000);
    for method in MaintenanceMethod::ALL {
        adapter.fail_from_call(method, 1, Fault::StatementTimeout);
    }
    let retention = Duration::from_secs(50_000);
    let errors = [
        adapter.delete_expired_sessions(HALF_DAY).await.unwrap_err(),
        adapter
            .purge_login_attempts(retention, 5)
            .await
            .unwrap_err(),
        adapter.purge_security_events().await.unwrap_err(),
        adapter
            .record_state(&StateValue::HeartbeatLastAt(at(0)))
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.code(), ErrorCode::DB_002);
    }
    let state = store.state();
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(state.attempts.len(), 1);
    assert!(state.settings.is_empty());
    assert_eq!(adapter.expired_security_events(), 10);
}

#[tokio::test]
async fn each_method_counts_its_own_calls() {
    let (_store, adapter) = maintenance();
    adapter.delete_expired_sessions(HALF_DAY).await.unwrap();
    adapter.delete_expired_sessions(HALF_DAY).await.unwrap();
    adapter.purge_security_events().await.unwrap();
    assert_eq!(adapter.calls(MaintenanceMethod::DeleteExpiredSessions), 2);
    assert_eq!(adapter.calls(MaintenanceMethod::PurgeSecurityEvents), 1);
    assert_eq!(adapter.calls(MaintenanceMethod::RecordState), 0);
}

#[tokio::test]
async fn clear_faults_stops_the_faults() {
    let (_store, adapter) = maintenance();
    adapter.fail_from_call(
        MaintenanceMethod::PurgeSecurityEvents,
        1,
        Fault::Unavailable,
    );
    adapter.purge_security_events().await.unwrap_err();
    adapter.clear_faults();
    assert_eq!(adapter.purge_security_events().await.unwrap(), 0);
}

#[tokio::test]
async fn a_begin_fault_of_the_store_fails_the_call() {
    let (store, adapter) = maintenance();
    store.fail_begin(Fault::Unavailable);
    let error = adapter.delete_expired_sessions(HALF_DAY).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
}

#[tokio::test]
async fn a_clone_shares_the_faults_and_the_counters() {
    let (_store, adapter) = maintenance();
    let copy = adapter.clone();
    copy.fail_from_call(
        MaintenanceMethod::PurgeSecurityEvents,
        1,
        Fault::Unavailable,
    );
    adapter.purge_security_events().await.unwrap_err();
    assert_eq!(copy.calls(MaintenanceMethod::PurgeSecurityEvents), 1);
}
