//! Tests of the border of a stored time in the future (ADR-0093 (a), finding CR-A8-2).
//!
//! A stored time later than `database_now` by at most `alerts.interval` has the age zero. A later
//! time is untrusted, and its condition stays active. Each border is tested at 1 ns before, exactly
//! at, and 1 ns after the interval, and at 1 s after it.

use recipe_domain::timestamp::Timestamp;
use time::Duration as TimeDuration;

use super::support::{Rig, at, rig, rig_with, sent_by_one_run};
use crate::jobs::alerts::AlertSettings;
use crate::ports::AlertCondition;
use std::time::Duration;

/// The default of `alerts.interval` in nanoseconds: 60 s.
const INTERVAL_NANOS: i64 = 60_000_000_000;

/// The distances in nanoseconds ahead of `database_now`, with the expected trust of the time.
const DEFAULT_CASES: [(i64, bool); 4] = [
    (INTERVAL_NANOS - 1, true),
    (INTERVAL_NANOS, true),
    (INTERVAL_NANOS + 1, false),
    (61_000_000_000, false),
];

/// Return the instant that is `nanos` after `database_now` of the rig, which is `at(0)`.
fn ahead(nanos: i64) -> Timestamp {
    Timestamp::from(
        at(0)
            .as_offset_date_time()
            .checked_add(TimeDuration::nanoseconds(nanos))
            .unwrap(),
    )
}

/// Run the job once and return true when it sent `condition`.
async fn is_sent(rig: &Rig, condition: AlertCondition) -> bool {
    sent_by_one_run(rig).await.contains(&condition)
}

#[tokio::test]
async fn a_dump_time_ahead_is_trusted_up_to_the_interval_and_not_beyond_it() {
    for (nanos, trusted) in DEFAULT_CASES {
        let rig = rig();
        rig.backups.set_newest(Some(ahead(nanos)));
        assert_eq!(
            is_sent(&rig, AlertCondition::BackupAge).await,
            !trusted,
            "nanoseconds ahead: {nanos}"
        );
    }
}

#[tokio::test]
async fn a_check_time_ahead_is_trusted_up_to_the_interval_and_not_beyond_it() {
    for (nanos, trusted) in DEFAULT_CASES {
        let rig = rig();
        rig.operations
            .edit(|snapshot| snapshot.audit_last_check_at = Some(ahead(nanos)));
        assert_eq!(
            is_sent(&rig, AlertCondition::AuditCheckAge).await,
            !trusted,
            "nanoseconds ahead: {nanos}"
        );
    }
}

#[tokio::test]
async fn a_first_start_ahead_gives_the_grace_of_a_missing_dump_only_up_to_the_interval() {
    for (nanos, trusted) in DEFAULT_CASES {
        let rig = rig();
        rig.backups.set_newest(None);
        rig.operations
            .edit(|snapshot| snapshot.first_setting_at = Some(ahead(nanos)));
        assert_eq!(
            is_sent(&rig, AlertCondition::BackupAge).await,
            !trusted,
            "nanoseconds ahead: {nanos}"
        );
    }
}

#[tokio::test]
async fn a_first_start_ahead_gives_the_grace_of_a_missing_check_only_up_to_the_interval() {
    for (nanos, trusted) in DEFAULT_CASES {
        let rig = rig();
        rig.operations.edit(|snapshot| {
            snapshot.audit_last_check_at = None;
            snapshot.first_setting_at = Some(ahead(nanos));
        });
        assert_eq!(
            is_sent(&rig, AlertCondition::AuditCheckAge).await,
            !trusted,
            "nanoseconds ahead: {nanos}"
        );
    }
}

#[tokio::test]
async fn a_chain_break_time_ahead_is_active_at_every_distance() {
    for (nanos, _) in DEFAULT_CASES {
        let rig = rig();
        rig.operations
            .edit(|snapshot| snapshot.audit_chain_broken_at = Some(ahead(nanos)));
        assert!(
            is_sent(&rig, AlertCondition::AuditChainBroken).await,
            "nanoseconds ahead: {nanos}"
        );
    }
}

#[tokio::test]
async fn the_trusted_skew_is_the_interval_of_the_settings() {
    let settings = AlertSettings {
        interval: Duration::from_secs(5),
        ..AlertSettings::default()
    };
    let cases = [
        (4_999_999_999, true),
        (5_000_000_000, true),
        (5_000_000_001, false),
        (INTERVAL_NANOS, false),
    ];
    for (nanos, trusted) in cases {
        let rig = rig_with(settings);
        rig.backups.set_newest(Some(ahead(nanos)));
        assert_eq!(
            is_sent(&rig, AlertCondition::BackupAge).await,
            !trusted,
            "nanoseconds ahead: {nanos}"
        );
    }
}
