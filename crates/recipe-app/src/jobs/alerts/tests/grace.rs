//! Tests of the start grace (design D1, `reliability.md` 3.7, ADR-0060 (x), (y), ADR-0073 (c)).

use super::support::{
    BACKUP_TEXT, HOUR, LogCapture, WINDOW_26H, at, earlier, rig, run, sent_by_one_run, set_time,
    times,
};
use crate::ports::AlertCondition;

/// Make the installation start `age` seconds before the database time.
fn started_ago(rig: &super::support::Rig, age: i64) {
    rig.operations
        .edit(|snapshot| snapshot.first_setting_at = Some(earlier(age)));
}

#[tokio::test]
async fn a_missing_dump_raises_no_alert_inside_the_grace() {
    let rig = rig();
    rig.backups.set_newest(None);
    started_ago(&rig, WINDOW_26H - 1);
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_missing_dump_raises_the_alert_when_the_grace_has_ended() {
    let rig = rig();
    rig.backups.set_newest(None);
    started_ago(&rig, WINDOW_26H);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::BackupAge]);
}

#[tokio::test]
async fn the_grace_of_the_dump_ends_with_the_database_time() {
    let rig = rig();
    rig.backups.set_newest(None);
    started_ago(&rig, HOUR);
    assert!(sent_by_one_run(&rig).await.is_empty());
    set_time(&rig, WINDOW_26H);
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = Some(at(WINDOW_26H)));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::BackupAge]);
}

#[tokio::test]
async fn a_missing_check_time_raises_no_alert_inside_the_grace() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = None);
    started_ago(&rig, WINDOW_26H - 1);
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn a_missing_check_time_raises_the_alert_when_the_grace_has_ended() {
    let rig = rig();
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = None);
    started_ago(&rig, WINDOW_26H);
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::AuditCheckAge]
    );
}

#[tokio::test]
async fn an_unknown_first_start_gives_no_grace() {
    let rig = rig();
    rig.backups.set_newest(None);
    rig.operations.edit(|snapshot| {
        snapshot.audit_last_check_at = None;
        snapshot.first_setting_at = None;
    });
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::BackupAge, AlertCondition::AuditCheckAge]
    );
}

#[tokio::test]
async fn an_unreadable_backup_directory_raises_the_alert_inside_the_grace() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.backups.set_unreadable();
    started_ago(&rig, HOUR);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::BackupAge]);
    let events = logs.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].message, BACKUP_TEXT);
    assert!(events[0].field("alert").is_none());
    assert_eq!(events[1].field("alert"), Some("\"backup_age\""));
}

#[tokio::test]
async fn an_old_dump_raises_the_alert_inside_the_grace() {
    let rig = rig();
    started_ago(&rig, times(2, HOUR));
    rig.backups.set_newest(Some(at(-WINDOW_26H - 1)));
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::BackupAge]);
}

#[tokio::test]
async fn the_grace_does_not_apply_to_other_conditions() {
    let rig = rig();
    started_ago(&rig, HOUR);
    rig.operations.edit(|snapshot| snapshot.failed_logins = 51);
    run(&rig).await.unwrap();
    assert_eq!(
        rig.webhook.calls(),
        vec![vec![AlertCondition::FailedLogins]]
    );
}
