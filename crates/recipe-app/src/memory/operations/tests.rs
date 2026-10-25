//! Tests of the in-memory `OperationsReads` (task 3.10, design D6, test rows T7 and T14).

use std::time::Duration;

use recipe_domain::audit::SecurityEvent;
use recipe_domain::error_codes::ErrorCode;

use super::{MemoryOperations, OperationsMethod};
use crate::auth::Operation;
use crate::memory::testkit::{
    at, csrf_rejected, forbidden, login_failed, request_id, set_time, store,
};
use crate::memory::{Fault, JournalEntry, MemoryMaintenance, MemoryStore};
use crate::ports::{
    Isolation, MaintenanceStore as _, OperationsReads as _, ReplicationState, RoleCheck,
    SecurityEventSink as _, SnapshotWindows, StateValue, TimeStateKey,
};

fn operations() -> (MemoryStore, MemoryOperations) {
    let store = store();
    let adapter = MemoryOperations::new(store.clone());
    (store, adapter)
}

fn windows(failed_logins: u64, security: u64) -> SnapshotWindows {
    SnapshotWindows {
        failed_logins: Duration::from_secs(failed_logins),
        security: Duration::from_secs(security),
    }
}

async fn record(store: &MemoryStore, event: SecurityEvent) {
    store.sink().record(event).await.unwrap();
}

#[tokio::test]
async fn database_now_is_the_time_of_the_store_clock() {
    let (store, adapter) = operations();
    assert_eq!(adapter.database_now().await.unwrap(), at(0));
    set_time(&store, 90);
    assert_eq!(adapter.database_now().await.unwrap(), at(90));
}

#[tokio::test]
async fn the_snapshot_holds_the_database_time_and_the_defaults_of_an_empty_system() {
    let (_store, adapter) = operations();
    let snapshot = adapter.snapshot(&windows(60, 60)).await.unwrap();
    assert_eq!(snapshot.database_now, at(0));
    assert_eq!(
        snapshot.replication,
        ReplicationState {
            connected: true,
            lag_bytes: Some(0)
        }
    );
    assert_eq!(snapshot.failed_logins, 0);
    assert_eq!(snapshot.security_alerts, 0);
    assert_eq!(snapshot.audit_chain_broken_at, None);
    assert_eq!(snapshot.audit_last_check_at, None);
    assert_eq!(snapshot.first_setting_at, None);
}

#[tokio::test]
async fn the_replication_state_and_the_role_check_are_set_by_the_test() {
    let (_store, adapter) = operations();
    let down = ReplicationState {
        connected: false,
        lag_bytes: None,
    };
    adapter.set_replication(down);
    assert_eq!(
        adapter.snapshot(&windows(1, 1)).await.unwrap().replication,
        down
    );
    assert!(adapter.role_check().await.unwrap().is_allowed());
    let wrong = RoleCheck {
        current_user: "postgres".to_owned(),
        is_superuser: true,
        owns_relations: false,
    };
    adapter.set_role_check(wrong.clone());
    assert_eq!(adapter.role_check().await.unwrap(), wrong);
}

#[tokio::test]
async fn failed_logins_sum_the_events_inside_the_window() {
    let (store, adapter) = operations();
    for n in 1..=3 {
        record(&store, login_failed(n)).await;
    }
    set_time(&store, 1_000);
    for n in 4..=5 {
        record(&store, login_failed(n)).await;
    }
    record(&store, csrf_rejected(6)).await;
    assert_eq!(
        adapter
            .snapshot(&windows(500, 1))
            .await
            .unwrap()
            .failed_logins,
        2
    );
    assert_eq!(
        adapter
            .snapshot(&windows(2_000, 1))
            .await
            .unwrap()
            .failed_logins,
        5
    );
}

#[tokio::test]
async fn security_alerts_count_csrf_events_and_forbidden_events_on_user_manage() {
    let (store, adapter) = operations();
    record(&store, csrf_rejected(1)).await;
    record(&store, forbidden(2, Operation::CreateUser.name())).await;
    record(&store, forbidden(3, Operation::CreateMachine.name())).await;
    record(&store, login_failed(4)).await;
    let snapshot = adapter.snapshot(&windows(60, 60)).await.unwrap();
    assert_eq!(snapshot.security_alerts, 2);
}

#[tokio::test]
async fn security_alerts_outside_the_window_do_not_count() {
    let (store, adapter) = operations();
    record(&store, csrf_rejected(1)).await;
    set_time(&store, 1_000);
    record(&store, csrf_rejected(2)).await;
    let snapshot = adapter.snapshot(&windows(60, 500)).await.unwrap();
    assert_eq!(snapshot.security_alerts, 1);
}

#[tokio::test]
async fn the_audit_chain_broken_time_is_the_newest_event_of_that_kind() {
    let (store, adapter) = operations();
    record(&store, SecurityEvent::audit_chain_broken(request_id(1))).await;
    set_time(&store, 500);
    record(&store, SecurityEvent::audit_chain_broken(request_id(2))).await;
    set_time(&store, 900);
    record(&store, csrf_rejected(3)).await;
    let snapshot = adapter.snapshot(&windows(60, 60)).await.unwrap();
    assert_eq!(snapshot.audit_chain_broken_at, Some(at(500)));
}

#[tokio::test]
async fn the_state_keys_come_from_the_rows_that_record_state_wrote() {
    let (store, adapter) = operations();
    assert_eq!(
        adapter.state(TimeStateKey::AuditLastCheckAt).await.unwrap(),
        None
    );
    let maintenance = MemoryMaintenance::new(store.clone());
    set_time(&store, 10);
    maintenance
        .record_state(&StateValue::AuditLastCheckAt(at(7)))
        .await
        .unwrap();
    assert_eq!(
        adapter.state(TimeStateKey::AuditLastCheckAt).await.unwrap(),
        Some(at(7))
    );
    assert_eq!(
        adapter.state(TimeStateKey::HeartbeatLastAt).await.unwrap(),
        None
    );
    let snapshot = adapter.snapshot(&windows(60, 60)).await.unwrap();
    assert_eq!(snapshot.audit_last_check_at, Some(at(7)));
}

#[tokio::test]
async fn the_first_setting_time_is_the_oldest_created_at() {
    let (store, adapter) = operations();
    let maintenance = MemoryMaintenance::new(store.clone());
    set_time(&store, 10);
    maintenance
        .record_state(&StateValue::HeartbeatLastAt(at(10)))
        .await
        .unwrap();
    set_time(&store, 20);
    maintenance
        .record_state(&StateValue::AuditLastCheckAt(at(20)))
        .await
        .unwrap();
    let snapshot = adapter.snapshot(&windows(60, 60)).await.unwrap();
    assert_eq!(snapshot.first_setting_at, Some(at(10)));
}

#[tokio::test]
async fn a_bad_text_form_of_a_state_key_is_a_storage_error_and_not_none() {
    let (_store, adapter) = operations();
    let bad = [
        "",
        "not a time",
        "2026-10-23T02:00:00Z",
        "2026-10-23T02:00:00.000000+00:00",
        "2026-10-23T02:00:00.123Z",
    ];
    for text in bad {
        adapter
            .seed_state_text(TimeStateKey::AuditLastCheckAt, text)
            .unwrap();
        let error = adapter
            .state(TimeStateKey::AuditLastCheckAt)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::DB_008, "text: {text:?}");
        let error = adapter.snapshot(&windows(60, 60)).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::DB_008, "text: {text:?}");
    }
}

#[tokio::test]
async fn a_bad_heartbeat_text_does_not_break_the_snapshot() {
    let (_store, adapter) = operations();
    adapter
        .seed_state_text(TimeStateKey::HeartbeatLastAt, "bad")
        .unwrap();
    adapter.snapshot(&windows(60, 60)).await.unwrap();
    assert_eq!(
        adapter
            .state(TimeStateKey::HeartbeatLastAt)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::DB_008
    );
}

#[tokio::test]
async fn the_snapshot_reads_in_one_read_only_transaction() {
    let (store, adapter) = operations();
    adapter.snapshot(&windows(60, 60)).await.unwrap();
    assert_eq!(
        store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn a_fault_on_the_second_call_of_a_method_leaves_the_first_call_whole() {
    let (_store, adapter) = operations();
    adapter.fail_on_call(OperationsMethod::Snapshot, 2, Fault::Serialization);
    adapter.snapshot(&windows(60, 60)).await.unwrap();
    let error = adapter.snapshot(&windows(60, 60)).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    adapter.snapshot(&windows(60, 60)).await.unwrap();
}

#[tokio::test]
async fn every_method_honours_its_fault() {
    let (_store, adapter) = operations();
    for method in OperationsMethod::ALL {
        adapter.fail_from_call(method, 1, Fault::PoolTimeout);
    }
    let errors = [
        adapter.snapshot(&windows(60, 60)).await.unwrap_err(),
        adapter.database_now().await.unwrap_err(),
        adapter.role_check().await.unwrap_err(),
        adapter
            .state(TimeStateKey::HeartbeatLastAt)
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.code(), ErrorCode::DB_004);
    }
    for method in OperationsMethod::ALL {
        assert_eq!(adapter.calls(method), 1);
    }
}

#[tokio::test]
async fn clear_faults_stops_the_faults() {
    let (_store, adapter) = operations();
    adapter.fail_from_call(OperationsMethod::DatabaseNow, 1, Fault::Unavailable);
    adapter.database_now().await.unwrap_err();
    adapter.clear_faults();
    adapter.database_now().await.unwrap();
}

#[tokio::test]
async fn a_begin_fault_of_the_store_fails_the_call() {
    let (store, adapter) = operations();
    store.fail_begin(Fault::Unavailable);
    let error = adapter.database_now().await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
}
