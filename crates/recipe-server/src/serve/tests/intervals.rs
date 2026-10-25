//! The interval of each loop comes from the plan (`reliability.md` section 7.11).
//!
//! Each test sets one prime interval for one job, lets virtual time pass, and counts the reads or
//! writes of that job. A prime number keeps the runs of two jobs apart.

use std::time::Duration;

use recipe_app::memory::{MaintenanceMethod, OperationsMethod};

use super::{advance, quiet_plan, running_with};
use crate::serve::test_support::stub::{Event, StubHost};

const SECONDS: fn(u64) -> Duration = Duration::from_secs;

#[test]
fn the_disk_check_runs_again_every_interval_of_the_plan() {
    let host = StubHost::new();
    let mut plan = quiet_plan();
    plan.settings.disk_check_interval = SECONDS(7);
    let server = running_with(&host, plan);
    assert_eq!(host.disk.calls(), 1, "the first run of the start");
    advance(&server, 30);
    assert_eq!(host.disk.calls(), 5);
    let _report = server.close();
}

#[test]
fn the_heartbeat_runs_every_interval_of_the_plan() {
    let host = StubHost::new();
    let mut plan = quiet_plan();
    plan.settings.heartbeat_interval = SECONDS(11);
    let server = running_with(&host, plan);
    advance(&server, 30);
    assert_eq!(host.maintenance.calls(MaintenanceMethod::RecordState), 2);
    let _report = server.close();
}

#[test]
fn the_clock_check_runs_at_once_and_then_every_interval_of_the_plan() {
    let host = StubHost::new();
    let mut plan = quiet_plan();
    plan.settings.clock_check_interval = SECONDS(13);
    let server = running_with(&host, plan);
    assert_eq!(host.log.count(&Event::ClockRead), 1, "the run at once");
    advance(&server, 30);
    assert_eq!(host.operations.calls(OperationsMethod::DatabaseNow), 3);
    let _report = server.close();
}

#[test]
fn the_alerts_job_runs_every_interval_of_the_plan() {
    let host = StubHost::new();
    let mut plan = quiet_plan();
    plan.settings.alerts.interval = SECONDS(17);
    let server = running_with(&host, plan);
    assert_eq!(host.log.count(&Event::Snapshot), 0, "no run at the start");
    advance(&server, 40);
    assert_eq!(host.log.count(&Event::Snapshot), 2);
    let _report = server.close();
}

#[test]
fn the_cleanup_loop_uses_the_shorter_of_its_two_intervals() {
    let host = StubHost::new();
    let mut plan = quiet_plan();
    plan.settings.cleanup.session_cleanup_interval = SECONDS(19);
    plan.settings.cleanup.login_attempt_cleanup_interval = SECONDS(23);
    let server = running_with(&host, plan);
    advance(&server, 21);
    assert_eq!(
        host.maintenance
            .calls(MaintenanceMethod::DeleteExpiredSessions),
        1
    );
    let _report = server.close();
}

#[test]
fn the_cleanup_job_gets_the_absolute_timeout_of_the_session() {
    let host = StubHost::new();
    let mut plan = quiet_plan();
    plan.settings.cleanup.session_cleanup_interval = SECONDS(19);
    plan.settings.identity.session_absolute_timeout = SECONDS(3_600);
    let server = running_with(&host, plan);
    advance(&server, 21);
    assert_eq!(
        host.log.count(&Event::SessionPurge(SECONDS(3_600))),
        1,
        "{:?}",
        host.log.events()
    );
    let _report = server.close();
}
