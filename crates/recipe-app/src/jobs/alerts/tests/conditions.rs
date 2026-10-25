//! Tests of the ten conditions of `reliability.md` section 3.7 and their borders.

use tracing::Level;

use super::support::{
    DAY, HOUR, LogCapture, MINUTE, WINDOW_26H, at, earlier, rig, run, sent_by_one_run, set_time,
    space, times,
};
use crate::jobs::RunKind;
use crate::ports::{AlertCondition, ReplicationState};

/// The default `wal.max_lag_bytes`: 16 MiB.
const MAX_LAG: u64 = 16 * 1024 * 1024;

#[tokio::test]
async fn a_healthy_system_writes_no_alert_line_and_makes_no_webhook_call() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    run(&rig).await.unwrap();
    assert_eq!(logs.count(), 0);
    assert!(rig.webhook.calls().is_empty());
}

#[tokio::test]
async fn the_job_reads_one_snapshot_per_run_with_the_two_windows_of_the_settings() {
    let rig = rig();
    run(&rig).await.unwrap();
    run(&rig).await.unwrap();
    let calls = rig.operations.calls();
    assert_eq!(calls.snapshots.len(), 2);
    assert_eq!(calls.others, 0);
    let windows = calls.snapshots[0];
    assert_eq!(windows.failed_logins.as_secs(), 15 * 60);
    assert_eq!(windows.security.as_secs(), 15 * 60);
}

#[tokio::test]
async fn more_than_the_failed_login_count_in_the_window_makes_failed_logins_active() {
    let rig = rig();
    rig.operations.edit(|snapshot| snapshot.failed_logins = 50);
    assert!(sent_by_one_run(&rig).await.is_empty());
    rig.operations.edit(|snapshot| snapshot.failed_logins = 51);
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::FailedLogins]
    );
}

#[tokio::test]
async fn one_csrf_or_user_manage_denial_makes_security_events_active() {
    let rig = rig();
    rig.operations.edit(|snapshot| snapshot.security_alerts = 1);
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::SecurityEvents]
    );
}

#[tokio::test]
async fn a_lag_at_the_limit_makes_replication_active_and_a_lag_below_it_does_not() {
    let rig = rig();
    let lag = |bytes: u64| ReplicationState {
        connected: true,
        lag_bytes: Some(bytes),
    };
    rig.operations
        .edit(|snapshot| snapshot.replication = lag(MAX_LAG - 1));
    assert!(sent_by_one_run(&rig).await.is_empty());
    rig.operations
        .edit(|snapshot| snapshot.replication = lag(MAX_LAG));
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::Replication]
    );
}

#[tokio::test]
async fn a_receiver_that_is_not_connected_makes_replication_active() {
    let rig = rig();
    rig.operations.edit(|snapshot| {
        snapshot.replication = ReplicationState {
            connected: false,
            lag_bytes: None,
        };
    });
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::Replication]
    );
}

#[tokio::test]
async fn a_connected_receiver_without_a_lag_makes_replication_active() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.operations.edit(|snapshot| {
        snapshot.replication = ReplicationState {
            connected: true,
            lag_bytes: None,
        };
    });
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::Replication]
    );
    assert_eq!(logs.alert_names(), vec!["replication"]);
}

#[tokio::test]
async fn a_chain_break_stays_active_until_it_is_26_hours_old() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_chain_broken_at = Some(at(-WINDOW_26H + 1)));
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::AuditChainBroken]
    );
    rig.operations
        .edit(|snapshot| snapshot.audit_chain_broken_at = Some(at(-WINDOW_26H)));
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_chain_break_moves_out_of_the_window_with_the_database_time() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_chain_broken_at = Some(at(0)));
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::AuditChainBroken]
    );
    set_time(&rig, WINDOW_26H);
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = Some(at(WINDOW_26H)));
    rig.backups.set_newest(Some(at(WINDOW_26H)));
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_check_older_than_26_hours_makes_audit_check_age_active() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = Some(at(-WINDOW_26H)));
    assert!(sent_by_one_run(&rig).await.is_empty());
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = Some(at(-WINDOW_26H - 1)));
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::AuditCheckAge]
    );
}

#[tokio::test]
async fn a_dump_older_than_26_hours_makes_backup_age_active() {
    let rig = rig();
    rig.backups.set_newest(Some(at(-WINDOW_26H)));
    assert!(sent_by_one_run(&rig).await.is_empty());
    rig.backups.set_newest(Some(at(-WINDOW_26H - 1)));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::BackupAge]);
}

#[tokio::test]
async fn not_ready_for_more_than_one_minute_makes_not_ready_active() {
    let rig = rig();
    rig.readiness.mark_not_ready(at(-MINUTE));
    assert!(sent_by_one_run(&rig).await.is_empty());
    set_time(&rig, 1);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::NotReady]);
    rig.readiness.mark_ready();
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_new_export_hash_mismatch_is_active_in_one_run_only() {
    let rig = rig();
    rig.counters.record_export_hash_mismatch();
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::ExportHashMismatch]
    );
    assert!(sent_by_one_run(&rig).await.is_empty());
    rig.counters.record_export_hash_mismatch();
    rig.counters.record_export_hash_mismatch();
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::ExportHashMismatch]
    );
}

#[tokio::test]
async fn each_condition_writes_one_warn_line_with_its_name_in_the_field_alert() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.operations.edit(|snapshot| snapshot.security_alerts = 3);
    run(&rig).await.unwrap();
    let lines = logs.alert_lines();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].level, Level::WARN);
    assert_eq!(lines[0].field("alert"), Some("\"security_events\""));
    assert!(lines[0].field("volume").is_none());
}

#[tokio::test]
async fn all_ten_conditions_keep_the_names_of_adr_0075_in_the_log_and_in_the_call() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    make_every_condition_active(&rig);
    run(&rig).await.unwrap();
    let names = [
        "panics",
        "not_ready",
        "backup_age",
        "replication",
        "low_disk",
        "failed_logins",
        "security_events",
        "audit_chain_broken",
        "audit_check_age",
        "export_hash_mismatch",
    ];
    assert_eq!(logs.alert_names(), names);
    assert_eq!(rig.webhook.calls(), vec![AlertCondition::ALL.to_vec()]);
    let sent: Vec<&str> = AlertCondition::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(sent, names);
}

#[tokio::test]
async fn the_run_kind_has_no_effect() {
    let rig = rig();
    rig.operations.edit(|snapshot| snapshot.security_alerts = 1);
    super::support::run_kind(&rig, RunKind::CatchUp)
        .await
        .unwrap();
    super::support::run_kind(&rig, RunKind::Scheduled)
        .await
        .unwrap();
    assert_eq!(rig.webhook.calls().len(), 2);
}

/// Put the system into a state in which each of the ten conditions is active.
fn make_every_condition_active(rig: &super::support::Rig) {
    rig.operations.edit(|snapshot| {
        snapshot.replication = ReplicationState {
            connected: false,
            lag_bytes: None,
        };
        snapshot.failed_logins = 51;
        snapshot.security_alerts = 1;
        snapshot.audit_chain_broken_at = Some(at(-HOUR));
        snapshot.audit_last_check_at = Some(earlier(times(3, DAY)));
    });
    rig.backups.set_newest(Some(earlier(times(3, DAY))));
    rig.readiness.mark_not_ready(at(-HOUR));
    rig.disk.set(&super::support::backups_path(), space(100));
    rig.counters.record_export_hash_mismatch();
    for _ in 0..4 {
        rig.counters.record_panic();
    }
}
