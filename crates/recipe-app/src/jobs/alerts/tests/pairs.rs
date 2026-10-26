//! Tests of the pairs of settings that share a default.
//!
//! Three pairs have equal defaults: `interval` and `not_ready_for` (60 s), `failed_login_window`
//! and `security_window` (15 min), and `backup_max_age_alert` and `audit_max_check_age` (26 h).
//! A swap of two fields in the job passes a test that uses the defaults. Each test here gives the
//! two fields of a pair different values, in both directions, and tests the border of each.

use std::time::Duration;

use recipe_domain::timestamp::Timestamp;

use super::support::{HOUR, Rig, at, earlier, rig_with, sent_by_one_run};
use crate::jobs::alerts::AlertSettings;
use crate::ports::{AlertCondition, SnapshotWindows};

/// The pairs (`interval`, `not_ready_for`) in seconds. Each value is the smaller one in one pair.
const TIMING_PAIRS: [(u64, u64); 2] = [(5, 120), (120, 5)];

/// The pairs (`failed_login_window`, `security_window`) in minutes.
const WINDOW_PAIRS: [(u64, u64); 2] = [(5, 20), (20, 5)];

/// The pairs (`backup_max_age_alert`, `audit_max_check_age`) in hours.
const AGE_PAIRS: [(u64, u64); 2] = [(10, 30), (30, 10)];

/// Return `value` plus one.
fn after(value: i64) -> i64 {
    value.checked_add(1).unwrap()
}

/// Return `value` minus one.
fn before(value: i64) -> i64 {
    value.checked_sub(1).unwrap()
}

/// Return the signed value of a count.
fn signed(value: u64) -> i64 {
    i64::try_from(value).unwrap()
}

/// Return the duration of `count` units of `unit` seconds.
fn span(count: u64, unit: u64) -> Duration {
    Duration::from_secs(count.checked_mul(unit).unwrap())
}

/// Return the settings with the given `interval` and `not_ready_for` in seconds.
fn timing(interval: u64, not_ready_for: u64) -> AlertSettings {
    AlertSettings {
        interval: Duration::from_secs(interval),
        not_ready_for: Duration::from_secs(not_ready_for),
        ..AlertSettings::default()
    }
}

#[tokio::test]
async fn not_ready_follows_not_ready_for_and_not_the_interval() {
    for (interval, not_ready_for) in TIMING_PAIRS {
        let limit = signed(not_ready_for);
        for (elapsed, active) in [(limit, false), (after(limit), true)] {
            let rig = rig_with(timing(interval, not_ready_for));
            rig.readiness.mark_not_ready(earlier(elapsed));
            let expected = if active {
                vec![AlertCondition::NotReady]
            } else {
                Vec::new()
            };
            assert_eq!(
                sent_by_one_run(&rig).await,
                expected,
                "interval {interval} s, not_ready_for {not_ready_for} s, not ready for {elapsed} s"
            );
        }
    }
}

#[tokio::test]
async fn the_trusted_skew_follows_the_interval_and_not_not_ready_for() {
    for (interval, not_ready_for) in TIMING_PAIRS {
        let limit = signed(interval);
        for (ahead, trusted) in [(limit, true), (after(limit), false)] {
            let rig = rig_with(timing(interval, not_ready_for));
            rig.backups.set_newest(Some(at(ahead)));
            let expected = if trusted {
                Vec::new()
            } else {
                vec![AlertCondition::BackupAge]
            };
            assert_eq!(
                sent_by_one_run(&rig).await,
                expected,
                "interval {interval} s, not_ready_for {not_ready_for} s, ahead {ahead} s"
            );
        }
    }
}

#[tokio::test]
async fn each_window_goes_to_its_own_field_of_the_snapshot_call() {
    for (failed, security) in WINDOW_PAIRS {
        let rig = rig_with(AlertSettings {
            failed_login_window: span(failed, 60),
            security_window: span(security, 60),
            ..AlertSettings::default()
        });
        sent_by_one_run(&rig).await;
        let expected = SnapshotWindows {
            failed_logins: span(failed, 60),
            security: span(security, 60),
        };
        assert_eq!(
            rig.operations.calls().snapshots,
            vec![expected],
            "failed_login_window {failed} min, security_window {security} min"
        );
    }
}

/// The side of its window on which a condition becomes active.
#[derive(Clone, Copy)]
enum Border {
    /// Active when the stored time is older than the window.
    OlderThan,
    /// Active when the stored time is younger than the window.
    YoungerThan,
    /// Active when the first start is at least the window ago and the stored time is missing.
    Grace,
}

impl Border {
    /// Return the two ages around the border of `window` with the expectation of each.
    fn cases(self, window: i64) -> [(i64, bool); 2] {
        match self {
            Self::OlderThan => [(window, false), (after(window), true)],
            Self::YoungerThan => [(before(window), true), (window, false)],
            Self::Grace => [(before(window), false), (window, true)],
        }
    }
}

/// One condition with the window that drives it and the way to put its time into the snapshot.
struct Probe {
    /// True when `backup_max_age_alert` drives the condition, false for `audit_max_check_age`.
    backup: bool,
    /// The side of the window on which the condition is active.
    border: Border,
    /// The condition under test.
    condition: AlertCondition,
    /// Put the instant `then` into the source of the condition.
    setup: fn(&Rig, Timestamp),
}

/// Set the newest dump to `then`.
fn set_dump(rig: &Rig, then: Timestamp) {
    rig.backups.set_newest(Some(then));
}

/// Remove the dump and set the first start to `then`.
fn set_missing_dump(rig: &Rig, then: Timestamp) {
    rig.backups.set_newest(None);
    rig.operations
        .edit(|snapshot| snapshot.first_setting_at = Some(then));
}

/// Set the last audit check to `then`.
fn set_check(rig: &Rig, then: Timestamp) {
    rig.operations
        .edit(|snapshot| snapshot.audit_last_check_at = Some(then));
}

/// Remove the last audit check and set the first start to `then`.
fn set_missing_check(rig: &Rig, then: Timestamp) {
    rig.operations.edit(|snapshot| {
        snapshot.audit_last_check_at = None;
        snapshot.first_setting_at = Some(then);
    });
}

/// Set the time of the chain break to `then`.
fn set_chain_break(rig: &Rig, then: Timestamp) {
    rig.operations
        .edit(|snapshot| snapshot.audit_chain_broken_at = Some(then));
}

/// Test the borders of `probe` with a window of its own and the other window different.
///
/// The run must send exactly the condition of the probe, or nothing. A swapped window moves the
/// border of the condition or raises another condition.
async fn check_probe(probe: &Probe) {
    for (backup_hours, audit_hours) in AGE_PAIRS {
        let settings = AlertSettings {
            backup_max_age_alert: span(backup_hours, 3600),
            audit_max_check_age: span(audit_hours, 3600),
            ..AlertSettings::default()
        };
        let hours = if probe.backup {
            backup_hours
        } else {
            audit_hours
        };
        let window = signed(hours).checked_mul(HOUR).unwrap();
        for (age, active) in probe.border.cases(window) {
            let rig = rig_with(settings);
            (probe.setup)(&rig, earlier(age));
            let expected = if active {
                vec![probe.condition]
            } else {
                Vec::new()
            };
            assert_eq!(
                sent_by_one_run(&rig).await,
                expected,
                "backup {backup_hours} h, audit {audit_hours} h, age {age} s"
            );
        }
    }
}

#[tokio::test]
async fn backup_age_follows_backup_max_age_alert() {
    check_probe(&Probe {
        backup: true,
        border: Border::OlderThan,
        condition: AlertCondition::BackupAge,
        setup: set_dump,
    })
    .await;
}

#[tokio::test]
async fn the_grace_of_a_missing_dump_follows_backup_max_age_alert() {
    check_probe(&Probe {
        backup: true,
        border: Border::Grace,
        condition: AlertCondition::BackupAge,
        setup: set_missing_dump,
    })
    .await;
}

#[tokio::test]
async fn audit_check_age_follows_audit_max_check_age() {
    check_probe(&Probe {
        backup: false,
        border: Border::OlderThan,
        condition: AlertCondition::AuditCheckAge,
        setup: set_check,
    })
    .await;
}

#[tokio::test]
async fn the_grace_of_a_missing_check_follows_audit_max_check_age() {
    check_probe(&Probe {
        backup: false,
        border: Border::Grace,
        condition: AlertCondition::AuditCheckAge,
        setup: set_missing_check,
    })
    .await;
}

#[tokio::test]
async fn the_chain_break_window_follows_audit_max_check_age() {
    check_probe(&Probe {
        backup: false,
        border: Border::YoungerThan,
        condition: AlertCondition::AuditChainBroken,
        setup: set_chain_break,
    })
    .await;
}
