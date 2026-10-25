//! Tests of the strict 26 hour borders at full precision (finding CRX-A8-1).
//!
//! The database time of each run has a fraction of a second. Each border is tested at 1 ns before,
//! exactly at, and 1 ns after the age of 26 hours, so a compare in whole seconds fails.

use recipe_domain::timestamp::Timestamp;
use time::Duration as TimeDuration;

use super::support::{Rig, WINDOW_26H, at, rig, sent_by_one_run};
use crate::ports::AlertCondition;

/// The fraction of the database time of each run: half a second.
const FRACTION_NANOS: i64 = 500_000_000;

/// The ages around the border, as nanoseconds added to 26 hours, with the expected result of a
/// condition that is active below the border (`audit_chain_broken`, grace).
const BELOW_BORDER: [(i64, bool); 3] = [(-1, true), (0, false), (1, false)];

/// The same ages, with the expected result of a condition that is active above the border.
const ABOVE_BORDER: [(i64, bool); 3] = [(-1, false), (0, false), (1, true)];

/// Move the database time of the snapshot to the start of the test plus half a second.
///
/// Return that time.
fn set_fractional_now(rig: &Rig) -> Timestamp {
    let now = Timestamp::from(
        at(0)
            .as_offset_date_time()
            .checked_add(TimeDuration::nanoseconds(FRACTION_NANOS))
            .unwrap(),
    );
    rig.operations.edit(|snapshot| snapshot.database_now = now);
    now
}

/// Return the instant that is 26 hours and `extra_nanos` before `now`.
fn older_by(now: Timestamp, extra_nanos: i64) -> Timestamp {
    let then = now
        .as_offset_date_time()
        .checked_sub(TimeDuration::seconds(WINDOW_26H))
        .unwrap()
        .checked_sub(TimeDuration::nanoseconds(extra_nanos))
        .unwrap();
    Timestamp::from(then)
}

/// Run the job once and return true when it sent `condition`.
async fn is_sent(rig: &Rig, condition: AlertCondition) -> bool {
    sent_by_one_run(rig).await.contains(&condition)
}

#[tokio::test]
async fn the_chain_break_border_is_strict_at_full_precision() {
    for (extra, expected) in BELOW_BORDER {
        let rig = rig();
        let now = set_fractional_now(&rig);
        rig.operations
            .edit(|snapshot| snapshot.audit_chain_broken_at = Some(older_by(now, extra)));
        assert_eq!(
            is_sent(&rig, AlertCondition::AuditChainBroken).await,
            expected,
            "extra nanoseconds: {extra}"
        );
    }
}

#[tokio::test]
async fn the_backup_age_border_is_strict_at_full_precision() {
    for (extra, expected) in ABOVE_BORDER {
        let rig = rig();
        let now = set_fractional_now(&rig);
        rig.backups.set_newest(Some(older_by(now, extra)));
        assert_eq!(
            is_sent(&rig, AlertCondition::BackupAge).await,
            expected,
            "extra nanoseconds: {extra}"
        );
    }
}

#[tokio::test]
async fn the_audit_check_age_border_is_strict_at_full_precision() {
    for (extra, expected) in ABOVE_BORDER {
        let rig = rig();
        let now = set_fractional_now(&rig);
        rig.operations
            .edit(|snapshot| snapshot.audit_last_check_at = Some(older_by(now, extra)));
        assert_eq!(
            is_sent(&rig, AlertCondition::AuditCheckAge).await,
            expected,
            "extra nanoseconds: {extra}"
        );
    }
}

#[tokio::test]
async fn the_grace_of_a_missing_dump_ends_at_full_precision() {
    for (extra, in_grace) in BELOW_BORDER {
        let rig = rig();
        let now = set_fractional_now(&rig);
        rig.backups.set_newest(None);
        rig.operations
            .edit(|snapshot| snapshot.first_setting_at = Some(older_by(now, extra)));
        assert_eq!(
            is_sent(&rig, AlertCondition::BackupAge).await,
            !in_grace,
            "extra nanoseconds: {extra}"
        );
    }
}

#[tokio::test]
async fn the_grace_of_a_missing_check_ends_at_full_precision() {
    for (extra, in_grace) in BELOW_BORDER {
        let rig = rig();
        let now = set_fractional_now(&rig);
        rig.operations.edit(|snapshot| {
            snapshot.audit_last_check_at = None;
            snapshot.first_setting_at = Some(older_by(now, extra));
        });
        assert_eq!(
            is_sent(&rig, AlertCondition::AuditCheckAge).await,
            !in_grace,
            "extra nanoseconds: {extra}"
        );
    }
}
