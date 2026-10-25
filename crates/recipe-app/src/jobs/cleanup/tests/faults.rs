//! Tests of an error of the store in the job `cleanup` (ADR-0088 (c), ADR-0089 (d)).
//!
//! A failed batch keeps the batches before it. The next run continues and repeats nothing that
//! already ended without an error.

use recipe_domain::error_codes::ErrorCode;

use super::support::{calls, rig, run, seed_expired_session, seed_old_attempts, set_time};
use crate::error::AppError;
use crate::jobs::cleanup::CleanupSettings;
use crate::memory::{Fault, MaintenanceMethod};

fn batch_of_one() -> CleanupSettings {
    CleanupSettings {
        login_attempt_purge_batch: 1,
        ..CleanupSettings::default()
    }
}

#[tokio::test]
async fn a_failed_second_attempt_batch_keeps_the_first_and_the_next_run_continues() {
    let rig = rig(batch_of_one());
    seed_old_attempts(&rig, 3, 0);
    rig.maintenance
        .fail_on_call(MaintenanceMethod::PurgeLoginAttempts, 2, Fault::Unavailable);
    let error = run(&rig).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)));
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert_eq!(rig.store.state().attempts.len(), 2);
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 0);
    set_time(&rig, 1);
    run(&rig).await.unwrap();
    assert!(rig.store.state().attempts.is_empty());
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 1);
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 1);
}

#[tokio::test]
async fn a_failed_session_delete_stops_the_run_and_the_next_run_does_everything() {
    let rig = rig(CleanupSettings::default());
    seed_expired_session(&rig, 1, 0);
    seed_old_attempts(&rig, 2, 0);
    rig.maintenance
        .fail_on_call(MaintenanceMethod::DeleteExpiredSessions, 1, Fault::Deadlock);
    let error = run(&rig).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 0);
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 0);
    set_time(&rig, 1);
    run(&rig).await.unwrap();
    assert!(rig.store.state().sessions.is_empty());
    assert!(rig.store.state().attempts.is_empty());
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 1);
}

#[tokio::test]
async fn a_failed_second_security_batch_keeps_the_first_and_repeats_nothing_else() {
    let rig = rig(CleanupSettings::default());
    rig.maintenance.seed_expired_security_events(12_000);
    rig.maintenance.fail_on_call(
        MaintenanceMethod::PurgeSecurityEvents,
        2,
        Fault::StatementTimeout,
    );
    let error = run(&rig).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_002);
    assert_eq!(rig.maintenance.expired_security_events(), 7_000);
    set_time(&rig, 1);
    run(&rig).await.unwrap();
    assert_eq!(rig.maintenance.expired_security_events(), 0);
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 1);
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 1);
}

#[tokio::test]
async fn a_failed_run_changes_no_row_outside_the_purged_tables() {
    let rig = rig(CleanupSettings::default());
    rig.maintenance.fail_from_call(
        MaintenanceMethod::DeleteExpiredSessions,
        1,
        Fault::Unavailable,
    );
    run(&rig).await.unwrap_err();
    assert!(rig.store.audit_entries().is_empty());
    assert!(rig.store.state().settings.is_empty());
}
