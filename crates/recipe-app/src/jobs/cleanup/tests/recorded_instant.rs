//! Test of the instant that the job `cleanup` records for its cadences (D6, ADR-0060).

use std::time::Duration;

use super::support::{MINUTE, calls, rig_stepping, run, set_time};
use crate::jobs::cleanup::CleanupSettings;
use crate::memory::MaintenanceMethod;

/// One interval of the session cadence in seconds.
const SESSION_INTERVAL: i64 = 10 * MINUTE;
/// One interval of the login attempt cadence in seconds.
const ATTEMPT_INTERVAL: i64 = 30 * MINUTE;
/// One interval of the security event cadence in seconds.
const SECURITY_INTERVAL: i64 = 60 * MINUTE;

#[tokio::test]
async fn the_recorded_instant_is_the_start_of_the_run() {
    let settings = CleanupSettings {
        session_cleanup_interval: Duration::from_secs(600),
        login_attempt_cleanup_interval: Duration::from_secs(1_800),
        ..CleanupSettings::default()
    };
    // Each read of the job clock adds one second, so the end of a run is after its start.
    let rig = rig_stepping(settings, Duration::from_secs(1));
    run(&rig).await.unwrap();
    // Exactly one interval after the START of the first run, the sessions are due again.
    set_time(&rig, SESSION_INTERVAL);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::DeleteExpiredSessions), 2);
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 1);
    // The same holds for the login attempts, which ran only in the first run.
    set_time(&rig, ATTEMPT_INTERVAL);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeLoginAttempts), 2);
}

#[tokio::test]
async fn the_security_event_purge_records_the_start_of_the_run() {
    let settings = CleanupSettings {
        security_event_purge_interval: Duration::from_secs(3_600),
        ..CleanupSettings::default()
    };
    // Each read of the job clock adds one second, so the end of a run is after its start.
    let rig = rig_stepping(settings, Duration::from_secs(1));
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 1);
    // One second before the interval from the START, the purge is not due.
    set_time(&rig, SECURITY_INTERVAL - 1);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 1);
    // Exactly one interval after the START of the first run, the purge is due again.
    set_time(&rig, SECURITY_INTERVAL);
    run(&rig).await.unwrap();
    assert_eq!(calls(&rig, MaintenanceMethod::PurgeSecurityEvents), 2);
}
