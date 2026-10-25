//! Tests of the settings and of the three cadences of the job `cleanup` (ADR-0088 (c)).

use std::time::Duration;

use super::support::{
    DAY, FIVE_HOURS, HOUR, MINUTE, TEN_MINUTES, after, before, calls, rig, run, run_kind,
    seed_expired_session, seed_old_attempts, set_time,
};
use crate::jobs::RunKind;
use crate::jobs::cleanup::{CleanupSettings, SECURITY_EVENT_PURGE_BATCH};
use crate::memory::{MaintenanceMethod, SECURITY_PURGE_BATCH};

#[test]
fn the_defaults_equal_section_7_11() {
    let settings = CleanupSettings::default();
    assert_eq!(settings.session_cleanup_interval, Duration::from_secs(600));
    assert_eq!(
        settings.login_attempt_cleanup_interval,
        Duration::from_secs(600)
    );
    assert_eq!(
        settings.login_attempt_retention,
        Duration::from_secs(86_400)
    );
    assert_eq!(
        settings.security_event_purge_interval,
        Duration::from_secs(86_400)
    );
    assert_eq!(settings.login_attempt_purge_batch, 5_000);
}

#[test]
fn the_security_event_batch_equals_the_batch_of_the_function() {
    assert_eq!(SECURITY_EVENT_PURGE_BATCH, 5_000);
    assert_eq!(SECURITY_EVENT_PURGE_BATCH, SECURITY_PURGE_BATCH);
}

#[test]
fn the_loop_runs_at_the_shorter_of_the_two_intervals() {
    let shorter_sessions = CleanupSettings {
        session_cleanup_interval: Duration::from_secs(300),
        login_attempt_cleanup_interval: Duration::from_secs(900),
        ..CleanupSettings::default()
    };
    let shorter_attempts = CleanupSettings {
        session_cleanup_interval: Duration::from_secs(900),
        login_attempt_cleanup_interval: Duration::from_secs(300),
        ..CleanupSettings::default()
    };
    assert_eq!(shorter_sessions.loop_interval(), Duration::from_secs(300));
    assert_eq!(shorter_attempts.loop_interval(), Duration::from_secs(300));
    assert_eq!(
        CleanupSettings::default().loop_interval(),
        Duration::from_secs(600)
    );
}

#[tokio::test]
async fn the_first_run_purges_sessions_attempts_and_security_events() {
    let rig = rig(CleanupSettings::default());
    seed_expired_session(&rig, 1, 0);
    seed_old_attempts(&rig, 3, 0);
    rig.maintenance.seed_expired_security_events(7);
    run(&rig).await.unwrap();
    let state = rig.store.state();
    assert!(state.sessions.is_empty());
    assert!(state.attempts.is_empty());
    assert_eq!(rig.maintenance.expired_security_events(), 0);
}

#[tokio::test]
async fn a_catch_up_run_purges_like_a_scheduled_run() {
    let rig = rig(CleanupSettings::default());
    seed_expired_session(&rig, 1, 0);
    run_kind(&rig, RunKind::CatchUp).await.unwrap();
    assert!(rig.store.state().sessions.is_empty());
}

#[tokio::test]
async fn the_sessions_wait_one_interval_after_their_last_run() {
    let rig = rig(CleanupSettings::default());
    run(&rig).await.unwrap();
    let just_before = before(TEN_MINUTES, 1);
    seed_expired_session(&rig, 2, just_before);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 1);
    assert_eq!(rig.store.state().sessions.len(), 1);
    set_time(&rig, TEN_MINUTES);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 2);
    assert!(rig.store.state().sessions.is_empty());
}

#[tokio::test]
async fn the_two_cadences_keep_their_own_due_times_in_one_loop() {
    let settings = CleanupSettings {
        session_cleanup_interval: Duration::from_secs(600),
        login_attempt_cleanup_interval: Duration::from_secs(1_800),
        ..CleanupSettings::default()
    };
    let rig = rig(settings);
    let mut seen = Vec::new();
    for minutes in [0_i64, 10, 20, 30, 40] {
        set_time(&rig, minutes.checked_mul(MINUTE).unwrap());
        run(&rig).await.unwrap();
        seen.push((
            calls(&rig, MaintenanceMethod::DeleteExpiredSessions),
            calls(&rig, MaintenanceMethod::PurgeLoginAttempts),
            calls(&rig, MaintenanceMethod::PurgeSecurityEvents),
        ));
    }
    // Runs at minutes 0, 10, 20, 30, and 40 of one loop at the shorter interval.
    assert_eq!(
        seen,
        [(1, 1, 1), (2, 1, 1), (3, 1, 1), (4, 2, 1), (5, 2, 1)]
    );
}

#[tokio::test]
async fn the_security_event_purge_waits_for_its_interval() {
    let rig = rig(CleanupSettings::default());
    run(&rig).await.unwrap();
    rig.maintenance.seed_expired_security_events(3);
    set_time(&rig, before(DAY, 1));
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 1);
    assert_eq!(rig.maintenance.expired_security_events(), 3);
    set_time(&rig, DAY);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 2);
    assert_eq!(rig.maintenance.expired_security_events(), 0);
}

#[tokio::test]
async fn a_clock_that_moved_back_keeps_every_cadence_waiting() {
    let rig = rig(CleanupSettings::default());
    set_time(&rig, FIVE_HOURS);
    run(&rig).await.unwrap();
    set_time(&rig, HOUR);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 1);
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 1);
    // A re-anchor at HOUR makes HOUR + 10 minutes due (interval 10 minutes).
    // The anchor stays FIVE_HOURS, so the elapsed time is negative and nothing is due.
    set_time(&rig, after(HOUR, TEN_MINUTES));
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 1);
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 1);
    set_time(&rig, after(FIVE_HOURS, TEN_MINUTES));
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 2);
}

#[tokio::test]
async fn the_job_writes_no_audit_entry_and_no_state_key() {
    let rig = rig(CleanupSettings::default());
    seed_expired_session(&rig, 1, 0);
    seed_old_attempts(&rig, 2, 0);
    rig.maintenance.seed_expired_security_events(2);
    run(&rig).await.unwrap();
    assert!(rig.store.audit_entries().is_empty());
    assert!(rig.store.state().settings.is_empty());
    assert_eq!(calls(&rig, MaintenanceMethod::RecordState), 0);
}
