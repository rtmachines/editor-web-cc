//! Tests of the faults of one run: a failed snapshot, a backup fault, a clock that jumps, and a
//! panic of the webhook adapter (ADR-0089 (d), ADR-0073 (c), ADR-0060 (x), (aa)).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use uuid::Uuid;

use super::support::{
    BACKUP_TEXT, HOUR, LogCapture, Rig, WINDOW_26H, at, backups_path, pgdata, rig, run,
    sent_by_one_run, walarchive,
};
use crate::jobs::RunKind;
use crate::jobs::alerts::{AlertPorts, AlertSettings, AlertSources, AlertVolumes, Alerts};
use crate::memory::MemoryWebhook;
use crate::ports::{AlertCondition, AlertWebhook, WebhookFault};

/// Count `count` panics.
fn panic_times(rig: &Rig, count: u32) {
    for _ in 0..count {
        rig.counters.record_panic();
    }
}

#[tokio::test]
async fn a_failed_snapshot_returns_the_error_and_writes_no_line() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    panic_times(&rig, 10);
    rig.counters.record_export_hash_mismatch();
    rig.operations.set_failing(true);
    let error = run(&rig).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert_eq!(logs.count(), 0);
}

#[tokio::test]
async fn a_failed_snapshot_sends_no_call_and_reads_nothing_else() {
    let rig = rig();
    rig.operations.set_failing(true);
    run(&rig).await.unwrap_err();
    assert!(rig.webhook.calls().is_empty());
    assert!(rig.disk.queries().is_empty());
    assert_eq!(rig.backups.calls(), 0);
    assert_eq!(rig.operations.calls().snapshots.len(), 1);
}

#[tokio::test]
async fn a_failed_snapshot_does_not_consume_the_counters() {
    let rig = rig();
    panic_times(&rig, 4);
    rig.counters.record_export_hash_mismatch();
    rig.operations.set_failing(true);
    run(&rig).await.unwrap_err();
    rig.operations.set_failing(false);
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::Panics, AlertCondition::ExportHashMismatch]
    );
}

#[tokio::test]
async fn the_next_run_after_a_failed_snapshot_works() {
    let rig = rig();
    rig.operations.set_failing(true);
    run(&rig).await.unwrap_err();
    rig.operations.set_failing(false);
    run(&rig).await.unwrap();
    assert!(rig.webhook.calls().is_empty());
}

#[tokio::test]
async fn a_backup_fault_writes_one_fault_line_in_each_run_that_gets_it() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.backups.set_unreadable();
    run(&rig).await.unwrap();
    run(&rig).await.unwrap();
    let faults = logs
        .events()
        .into_iter()
        .filter(|event| event.message == BACKUP_TEXT)
        .count();
    assert_eq!(faults, 2);
    assert_eq!(rig.webhook.calls().len(), 2);
}

#[tokio::test]
async fn a_single_backup_fault_does_not_hold_the_condition() {
    let rig = rig();
    rig.backups.fail_on_call(1);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::BackupAge]);
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_chain_break_in_the_future_beyond_the_interval_stays_active() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_chain_broken_at = Some(at(WINDOW_26H * 2)));
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::AuditChainBroken]
    );
}

#[tokio::test]
async fn a_chain_break_in_the_future_within_the_interval_is_a_fresh_break() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_chain_broken_at = Some(at(30)));
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::AuditChainBroken]
    );
}

#[tokio::test]
async fn a_check_time_in_the_future_beyond_the_interval_makes_the_check_age_active() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = Some(at(HOUR)));
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::AuditCheckAge]
    );
}

#[tokio::test]
async fn a_check_time_in_the_future_within_the_interval_counts_as_age_zero() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = Some(at(30)));
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_dump_time_in_the_future_beyond_the_interval_makes_the_backup_age_active() {
    let rig = rig();
    rig.backups.set_newest(Some(at(HOUR)));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::BackupAge]);
}

#[tokio::test]
async fn a_dump_time_in_the_future_within_the_interval_counts_as_age_zero() {
    let rig = rig();
    rig.backups.set_newest(Some(at(30)));
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn the_job_reads_the_three_volumes_and_the_backup_directory_once_in_each_run() {
    let rig = rig();
    run(&rig).await.unwrap();
    assert_eq!(rig.backups.calls(), 1);
    assert_eq!(
        rig.disk.queries(),
        vec![pgdata(), backups_path(), walarchive()]
    );
}

/// A webhook adapter that panics while its switch is on.
///
/// It solves one problem: a test must prove that a panic of the adapter loses no alert. The
/// supervisor catches the panic, and the next run must send the same alert again.
struct PanickingWebhook {
    /// True while the adapter must panic.
    panics: Arc<AtomicBool>,
    /// The adapter that records the calls when the switch is off.
    inner: MemoryWebhook,
}

#[async_trait]
impl AlertWebhook for PanickingWebhook {
    async fn send_all(&self, conditions: &[AlertCondition]) -> Vec<Result<(), WebhookFault>> {
        assert!(!self.panics.load(Ordering::Relaxed), "injected panic");
        self.inner.send_all(conditions).await
    }
}

#[tokio::test]
async fn a_panic_of_the_webhook_adapter_loses_no_export_alert() {
    let rig = rig();
    let panics = Arc::new(AtomicBool::new(true));
    let recorded = MemoryWebhook::default();
    let job = Arc::new(Alerts::new(
        AlertPorts {
            operations: Arc::new(rig.operations.clone()),
            webhook: Arc::new(PanickingWebhook {
                panics: Arc::clone(&panics),
                inner: recorded.clone(),
            }),
            disk: Arc::new(rig.disk.clone()),
            backups: Arc::new(rig.backups.clone()),
        },
        AlertSources {
            counters: rig.counters.clone(),
            readiness: rig.readiness.clone(),
        },
        AlertVolumes {
            pgdata: pgdata(),
            backups: backups_path(),
            walarchive: walarchive(),
        },
        AlertSettings::default(),
    ));
    rig.counters.record_export_hash_mismatch();
    let first = Arc::clone(&job);
    let joined = tokio::spawn(async move {
        first
            .run_once(RequestId::from(Uuid::from_u128(1)), RunKind::Scheduled)
            .await
    })
    .await;
    assert!(joined.unwrap_err().is_panic());
    panics.store(false, Ordering::Relaxed);
    job.run_once(RequestId::from(Uuid::from_u128(2)), RunKind::Scheduled)
        .await
        .unwrap();
    assert_eq!(
        recorded.calls(),
        vec![vec![AlertCondition::ExportHashMismatch]]
    );
}
