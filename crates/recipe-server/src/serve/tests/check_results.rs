//! A failed result of a scheduled backup check reaches the webhook (task 5.2, ADR-0151 (h)).
//!
//! The job `alerts` reads the newest result of the restore test and of the base backup. A result
//! `failed` makes the condition active at once, so the webhook gets one body for each check.
//! The test names the conditions only as text, so it builds before the job reads the results.

use recipe_app::ports::{CheckOutcome, CheckResult, ScheduledCheck};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::advance;
use crate::serve::test_support::stub::{Event, StubHost};

const ALERT_INTERVAL_SECONDS: u64 = 61;

/// Return how often the webhook got the alert `name`, over all calls.
///
/// The memory webhook records the conditions of each call. The real sender turns a condition into
/// the body `{"alert":"<name>"}`, so the name stands for the body.
fn sent_by_name(host: &StubHost, name: &str) -> usize {
    host.webhook
        .calls()
        .iter()
        .flatten()
        .filter(|condition| condition.name() == name)
        .count()
}

/// A result `failed` that finished at the epoch.
fn failed_result() -> CheckResult {
    CheckResult {
        outcome: CheckOutcome::Failed,
        finished_at: Timestamp::from(OffsetDateTime::UNIX_EPOCH),
    }
}

#[test]
fn a_failed_result_of_each_check_reaches_the_webhook_once() {
    let host = StubHost::new();
    host.check_results
        .set_result(ScheduledCheck::RestoreTest, Some(failed_result()));
    host.check_results
        .set_result(ScheduledCheck::BaseBackup, Some(failed_result()));
    let server = super::expect_running(&host);
    advance(&server, ALERT_INTERVAL_SECONDS);
    assert_eq!(host.log.count(&Event::Snapshot), 1, "the job ran once");
    assert_eq!(sent_by_name(&host, "restore_test"), 1);
    assert_eq!(sent_by_name(&host, "base_backup"), 1);
    let _report = server.close();
}
