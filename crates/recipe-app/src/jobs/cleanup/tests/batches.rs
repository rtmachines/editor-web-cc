//! Tests of the batches of the job `cleanup` (`data-model.md` section 6, row `cleanup`).

use std::time::Duration;

use super::support::{
    HOUR, MINUTE, calls, rig, rig_with_timeout, run, seed_attempt_aged, seed_old_attempts,
    seed_session_aged,
};
use crate::jobs::cleanup::CleanupSettings;
use crate::memory::MaintenanceMethod;

fn small_batch(batch: u32) -> CleanupSettings {
    CleanupSettings {
        login_attempt_purge_batch: batch,
        ..CleanupSettings::default()
    }
}

#[tokio::test]
async fn the_attempts_go_in_batches_until_a_short_batch() {
    let rig = rig(small_batch(2));
    seed_old_attempts(&rig, 5, 0);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 3);
    assert!(rig.store.state().attempts.is_empty());
}

#[tokio::test]
async fn a_last_batch_that_is_exactly_full_gets_one_more_call() {
    let rig = rig(small_batch(2));
    seed_old_attempts(&rig, 4, 0);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 3);
    assert!(rig.store.state().attempts.is_empty());
}

#[tokio::test]
async fn the_default_batch_of_the_attempts_is_5000_rows() {
    let rig = rig(CleanupSettings::default());
    seed_old_attempts(&rig, 5_001, 0);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 2);
    assert!(rig.store.state().attempts.is_empty());
}

#[tokio::test]
async fn a_batch_size_of_zero_ends_the_run_after_one_call() {
    let rig = rig(small_batch(0));
    seed_old_attempts(&rig, 2, 0);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 1);
    assert_eq!(rig.store.state().attempts.len(), 2);
}

#[tokio::test]
async fn the_security_events_go_in_batches_of_5000_until_a_short_batch() {
    let rig = rig(CleanupSettings::default());
    rig.maintenance.seed_expired_security_events(12_000);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 3);
    assert_eq!(rig.maintenance.expired_security_events(), 0);
}

#[tokio::test]
async fn full_security_batches_get_one_more_call_after_the_last_full_batch() {
    let rig = rig(CleanupSettings::default());
    rig.maintenance.seed_expired_security_events(10_000);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 3);
    assert_eq!(rig.maintenance.expired_security_events(), 0);
}

#[tokio::test]
async fn the_sessions_go_in_one_statement() {
    let rig = rig(CleanupSettings::default());
    for n in 1..=3 {
        super::support::seed_expired_session(&rig, n, 0);
    }
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 1);
    assert!(rig.store.state().sessions.is_empty());
}

#[tokio::test]
async fn the_absolute_timeout_of_the_constructor_reaches_the_store() {
    let rig = rig_with_timeout(CleanupSettings::default(), Duration::from_secs(600));
    seed_session_aged(&rig, 1, 0, 11 * MINUTE);
    seed_session_aged(&rig, 2, 0, 5 * MINUTE);
    run(&rig).await.unwrap();
    assert_eq!(rig.store.state().sessions.len(), 1);
}

#[tokio::test]
async fn the_retention_of_the_settings_reaches_the_store() {
    let settings = CleanupSettings {
        login_attempt_retention: Duration::from_secs(3_600),
        ..CleanupSettings::default()
    };
    let rig = rig(settings);
    seed_attempt_aged(&rig, 0, 2 * HOUR);
    seed_attempt_aged(&rig, 0, 30 * MINUTE);
    run(&rig).await.unwrap();
    assert_eq!(rig.store.state().attempts.len(), 1);
}
