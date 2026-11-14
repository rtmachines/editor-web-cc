//! Tests of the conditions `restore_test` and `base_backup` (ADR-0151 (h) to (j2a), design D5 and
//! D11, delta requirements of the capability `operations`).
//!
//! Each scenario runs for both checks. The tests use the paused clock of `tokio`, so the time of a
//! run between the snapshot and the end of a read is known to the nanosecond.

use std::io;
use std::time::Duration;

use recipe_domain::error_codes::ErrorCode;
use tracing::Level;

use super::support::{
    DAY, HOUR, LogCapture, Rig, at, check_result, earlier, rig, run, sent_by_one_run, times,
};
use crate::ports::{AlertCondition, CheckOutcome, CheckResultFault, ScheduledCheck};

/// The limit of both checks: `restore_test.max_age_alert` and `basebackup.max_age_alert`, 8 days.
const LIMIT: i64 = 8 * DAY;
/// One second inside the limit.
const JUST_INSIDE: i64 = LIMIT - 1;
/// One second over the limit.
const JUST_OVER: i64 = LIMIT + 1;
/// The default of `alerts.interval` in seconds.
const INTERVAL: i64 = 60;
/// One second over `alerts.interval`.
const OVER_INTERVAL: i64 = INTERVAL + 1;
/// The time that each read of a result takes in the tests of the comparison moment.
const READ_SECONDS: u64 = 50;

/// One check, its condition, and the time that the reads up to its own read take.
#[derive(Clone, Copy, Debug)]
struct Case {
    /// The check under test.
    check: ScheduledCheck,
    /// The condition of the check.
    condition: AlertCondition,
    /// The name of the condition in the log and in the call.
    name: &'static str,
    /// The seconds from the snapshot to the end of the read of this check, when each read of a
    /// result takes [`READ_SECONDS`]. The job reads the restore test first.
    elapsed: i64,
}

/// The two checks in the order of `AlertCondition::ALL`.
const CASES: [Case; 2] = [
    Case {
        check: ScheduledCheck::RestoreTest,
        condition: AlertCondition::RestoreTest,
        name: "restore_test",
        elapsed: 50,
    },
    Case {
        check: ScheduledCheck::BaseBackup,
        condition: AlertCondition::BaseBackup,
        name: "base_backup",
        elapsed: 100,
    },
];

/// Give `case` the result `outcome` that ended `age` seconds before the database time.
fn ended_ago(rig: &Rig, case: Case, outcome: CheckOutcome, age: i64) {
    rig.checks
        .set_result(case.check, Some(check_result(outcome, earlier(age))));
}

/// Make the installation start `age` seconds before the database time.
fn started_ago(rig: &Rig, age: i64) {
    rig.operations
        .edit(|snapshot| snapshot.first_setting_at = Some(earlier(age)));
}

/// Return the `Debug` text of the field `alert` for `case`.
fn quoted(case: Case) -> String {
    format!("{:?}", case.name)
}

/// Return the text of the log line of a read fault of `case`.
fn fault_text(case: Case) -> String {
    CheckResultFault::Unreadable {
        check: case.check,
        source: io::Error::other("injected"),
    }
    .to_string()
}

/// Run the job once and return true when it sent the condition of `case`.
async fn is_sent(rig: &Rig, case: Case) -> bool {
    sent_by_one_run(rig).await.contains(&case.condition)
}

#[tokio::test(start_paused = true)]
async fn a_passed_result_inside_the_limit_is_inactive_and_writes_no_line() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        ended_ago(&rig, case, CheckOutcome::Passed, times(2, DAY));
        assert!(sent_by_one_run(&rig).await.is_empty(), "{case:?}");
        assert_eq!(logs.count(), 0, "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_passed_result_1_second_inside_the_limit_is_inactive() {
    for case in CASES {
        let rig = rig();
        ended_ago(&rig, case, CheckOutcome::Passed, JUST_INSIDE);
        assert!(sent_by_one_run(&rig).await.is_empty(), "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_passed_result_1_second_over_the_limit_is_active() {
    for case in CASES {
        let rig = rig();
        ended_ago(&rig, case, CheckOutcome::Passed, JUST_OVER);
        assert_eq!(
            sent_by_one_run(&rig).await,
            vec![case.condition],
            "{case:?}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn a_passed_result_9_days_old_is_active_with_one_warn_line() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        ended_ago(&rig, case, CheckOutcome::Passed, times(9, DAY));
        run(&rig).await.unwrap();
        let events = logs.events();
        assert_eq!(events.len(), 1, "{case:?}");
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].field("alert"), Some(quoted(case).as_str()));
    }
}

#[tokio::test(start_paused = true)]
async fn a_failed_result_is_active_with_one_warn_line_and_one_call() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        ended_ago(&rig, case, CheckOutcome::Failed, HOUR);
        run(&rig).await.unwrap();
        let events = logs.events();
        assert_eq!(events.len(), 1, "{case:?}");
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].field("alert"), Some(quoted(case).as_str()));
        assert_eq!(rig.webhook.calls(), vec![vec![case.condition]], "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_failed_result_is_active_also_inside_the_start_grace() {
    for case in CASES {
        let rig = rig();
        started_ago(&rig, HOUR);
        ended_ago(&rig, case, CheckOutcome::Failed, 10);
        assert_eq!(
            sent_by_one_run(&rig).await,
            vec![case.condition],
            "{case:?}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn a_missing_result_after_the_start_grace_is_active_with_one_warn_line() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        started_ago(&rig, times(30, DAY));
        rig.checks.set_result(case.check, None);
        run(&rig).await.unwrap();
        assert_eq!(logs.alert_names(), vec![case.name], "{case:?}");
        assert_eq!(logs.count(), 1, "{case:?}");
        assert_eq!(rig.webhook.calls(), vec![vec![case.condition]], "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_missing_result_inside_the_start_grace_is_inactive_and_writes_no_line() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        started_ago(&rig, DAY);
        rig.checks.set_result(case.check, None);
        assert!(sent_by_one_run(&rig).await.is_empty(), "{case:?}");
        assert_eq!(logs.count(), 0, "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_start_grace_of_a_check_is_its_limit_of_8_days() {
    for case in CASES {
        let rig = rig();
        rig.checks.set_result(case.check, None);
        started_ago(&rig, times(2, DAY));
        assert!(!is_sent(&rig, case).await, "2 days: {case:?}");
        started_ago(&rig, JUST_INSIDE);
        assert!(!is_sent(&rig, case).await, "1 s inside: {case:?}");
        started_ago(&rig, LIMIT);
        assert!(is_sent(&rig, case).await, "at the limit: {case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn an_unknown_first_start_gives_no_grace_to_a_missing_result() {
    for case in CASES {
        let rig = rig();
        rig.checks.set_result(case.check, None);
        rig.operations
            .edit(|snapshot| snapshot.first_setting_at = None);
        assert_eq!(
            sent_by_one_run(&rig).await,
            vec![case.condition],
            "{case:?}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn an_unreadable_result_is_active_with_one_fault_line_and_one_alert_line() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        rig.checks.set_unreadable(case.check);
        run(&rig).await.unwrap();
        let events = logs.events();
        assert_eq!(events.len(), 2, "{case:?}");
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].message, fault_text(case));
        assert!(events[0].field("alert").is_none());
        assert_eq!(events[1].level, Level::WARN);
        assert_eq!(events[1].field("alert"), Some(quoted(case).as_str()));
        assert_eq!(rig.webhook.calls(), vec![vec![case.condition]], "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn alerts_warn_line_of_a_check_fault_holds_error_code() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        rig.checks.set_unreadable(case.check);
        run(&rig).await.unwrap();
        let events = logs.events();
        assert_eq!(events[0].message, fault_text(case), "{case:?}");
        assert_eq!(
            events[0].field("error_code"),
            Some(ErrorCode::WEB_016.as_str()),
            "{case:?}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn a_read_fault_inside_the_start_grace_is_active_with_one_fault_line() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        started_ago(&rig, DAY);
        rig.checks.set_unreadable(case.check);
        run(&rig).await.unwrap();
        let events = logs.events();
        assert_eq!(events.len(), 2, "{case:?}");
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].message, fault_text(case));
        assert!(events[0].field("alert").is_none());
        assert_eq!(events[1].field("alert"), Some(quoted(case).as_str()));
        assert_eq!(rig.webhook.calls(), vec![vec![case.condition]], "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_fault_line_names_its_check_and_holds_no_path() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    rig.checks.set_unreadable(ScheduledCheck::RestoreTest);
    rig.checks.set_unreadable(ScheduledCheck::BaseBackup);
    run(&rig).await.unwrap();
    let faults: Vec<String> = logs
        .events()
        .into_iter()
        .filter(|event| event.field("alert").is_none())
        .map(|event| event.message)
        .collect();
    assert_eq!(faults, [fault_text(CASES[0]), fault_text(CASES[1])]);
    assert_ne!(faults[0], faults[1]);
}

#[tokio::test(start_paused = true)]
async fn a_single_read_fault_does_not_hold_the_condition() {
    for case in CASES {
        let rig = rig();
        rig.checks.fail_on_call(case.check, 1);
        assert_eq!(
            sent_by_one_run(&rig).await,
            vec![case.condition],
            "{case:?}"
        );
        assert!(sent_by_one_run(&rig).await.is_empty(), "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn each_run_reads_each_check_once_also_after_a_fault_of_the_other() {
    let rig = rig();
    rig.checks.set_unreadable(ScheduledCheck::RestoreTest);
    run(&rig).await.unwrap();
    assert_eq!(rig.checks.calls(ScheduledCheck::RestoreTest), 1);
    assert_eq!(rig.checks.calls(ScheduledCheck::BaseBackup), 1);
    run(&rig).await.unwrap();
    assert_eq!(rig.checks.calls(ScheduledCheck::RestoreTest), 2);
    assert_eq!(rig.checks.calls(ScheduledCheck::BaseBackup), 2);
}

#[tokio::test(start_paused = true)]
async fn a_result_ahead_of_the_comparison_moment_is_trusted_up_to_the_interval() {
    for case in CASES {
        let rig = rig();
        rig.checks.set_result(
            case.check,
            Some(check_result(CheckOutcome::Passed, at(INTERVAL))),
        );
        assert!(!is_sent(&rig, case).await, "at the interval: {case:?}");
        rig.checks.set_result(
            case.check,
            Some(check_result(CheckOutcome::Passed, at(OVER_INTERVAL))),
        );
        assert!(is_sent(&rig, case).await, "1 s over the interval: {case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn one_run_with_both_conditions_active_sends_both_names_in_the_order_of_all() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    for case in CASES {
        ended_ago(&rig, case, CheckOutcome::Failed, HOUR);
    }
    run(&rig).await.unwrap();
    assert_eq!(
        rig.webhook.calls(),
        vec![vec![
            AlertCondition::RestoreTest,
            AlertCondition::BaseBackup
        ]]
    );
    assert_eq!(logs.alert_names(), ["restore_test", "base_backup"]);
}

#[tokio::test(start_paused = true)]
async fn the_two_check_conditions_sit_between_backup_age_and_replication_in_one_call() {
    let rig = rig();
    rig.backups.set_newest(Some(earlier(times(3, DAY))));
    rig.operations.edit(|snapshot| {
        snapshot.replication = crate::ports::ReplicationState {
            connected: false,
            lag_bytes: None,
        };
    });
    for case in CASES {
        ended_ago(&rig, case, CheckOutcome::Failed, HOUR);
    }
    run(&rig).await.unwrap();
    assert_eq!(
        rig.webhook.calls(),
        vec![vec![
            AlertCondition::BackupAge,
            AlertCondition::RestoreTest,
            AlertCondition::BaseBackup,
            AlertCondition::Replication,
        ]]
    );
}

#[tokio::test(start_paused = true)]
async fn a_result_written_during_the_reads_of_the_run_is_inactive() {
    for case in CASES {
        let (logs, _guard) = LogCapture::install();
        let rig = rig();
        rig.checks.set_delay(Duration::from_secs(READ_SECONDS));
        rig.checks.set_result(
            case.check,
            Some(check_result(CheckOutcome::Passed, at(100))),
        );
        assert!(sent_by_one_run(&rig).await.is_empty(), "{case:?}");
        assert_eq!(logs.count(), 0, "{case:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_comparison_moment_follows_the_time_of_the_reads_up_to_the_end_of_the_read() {
    for case in CASES {
        let rig = rig();
        rig.checks.set_delay(Duration::from_secs(READ_SECONDS));
        let border = case.elapsed.checked_add(INTERVAL).unwrap();
        let over = border.checked_add(1).unwrap();
        rig.checks.set_result(
            case.check,
            Some(check_result(CheckOutcome::Passed, at(border))),
        );
        assert!(!is_sent(&rig, case).await, "at the border: {case:?}");
        rig.checks.set_result(
            case.check,
            Some(check_result(CheckOutcome::Passed, at(over))),
        );
        assert!(is_sent(&rig, case).await, "1 s over the border: {case:?}");
    }
}
