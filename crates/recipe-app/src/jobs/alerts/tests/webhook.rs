//! Tests of the webhook calls of one run (ADR-0063 (l) to (n), ADR-0075 (c), (e)).

use tracing::Level;

use super::support::{LogCapture, rig, run};
use crate::ports::{AlertCondition, WebhookFault};

/// Make three conditions active: failed logins, security events, and an export mismatch.
fn three_active(rig: &super::support::Rig) {
    rig.operations.edit(|snapshot| {
        snapshot.failed_logins = 99;
        snapshot.security_alerts = 2;
    });
    rig.counters.record_export_hash_mismatch();
}

#[tokio::test]
async fn one_call_carries_every_active_condition_in_the_order_of_the_table() {
    let rig = rig();
    three_active(&rig);
    run(&rig).await.unwrap();
    assert_eq!(
        rig.webhook.calls(),
        vec![vec![
            AlertCondition::FailedLogins,
            AlertCondition::SecurityEvents,
            AlertCondition::ExportHashMismatch,
        ]]
    );
}

#[tokio::test]
async fn with_no_active_condition_the_job_does_not_call_the_port() {
    let rig = rig();
    run(&rig).await.unwrap();
    run(&rig).await.unwrap();
    assert!(rig.webhook.calls().is_empty());
}

#[tokio::test]
async fn the_log_lines_come_before_the_call_and_match_the_call() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    three_active(&rig);
    run(&rig).await.unwrap();
    assert_eq!(
        logs.alert_names(),
        ["failed_logins", "security_events", "export_hash_mismatch"]
    );
}

#[tokio::test]
async fn a_failed_call_writes_one_warn_line_and_the_run_continues() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    three_active(&rig);
    rig.webhook
        .fail_condition(AlertCondition::SecurityEvents, WebhookFault::Timeout);
    run(&rig).await.unwrap();
    let faults: Vec<_> = logs
        .events()
        .into_iter()
        .filter(|event| event.field("alert").is_none())
        .collect();
    assert_eq!(faults.len(), 1);
    assert_eq!(faults[0].level, Level::WARN);
    assert_eq!(faults[0].message, WebhookFault::Timeout.to_string());
    assert_eq!(faults[0].field("condition"), Some("\"security_events\""));
    assert_eq!(rig.webhook.sent(AlertCondition::FailedLogins), 1);
    assert_eq!(rig.webhook.sent(AlertCondition::ExportHashMismatch), 1);
}

#[tokio::test]
async fn every_failed_call_gets_its_own_line_with_its_own_reason() {
    let (logs, _guard) = LogCapture::install();
    let rig = rig();
    three_active(&rig);
    rig.webhook
        .fail_condition(AlertCondition::FailedLogins, WebhookFault::Connection);
    rig.webhook.fail_condition(
        AlertCondition::ExportHashMismatch,
        WebhookFault::Status { code: 500 },
    );
    run(&rig).await.unwrap();
    let messages: Vec<_> = logs
        .events()
        .into_iter()
        .filter(|event| event.field("alert").is_none())
        .map(|event| event.message)
        .collect();
    assert_eq!(
        messages,
        [
            "The connection to the webhook host failed.",
            "The webhook host answered with the status 500."
        ]
    );
}

#[tokio::test]
async fn a_failed_call_does_not_stop_the_next_run_from_sending_again() {
    let rig = rig();
    three_active(&rig);
    rig.webhook
        .fail_on_call(AlertCondition::FailedLogins, 1, WebhookFault::Tls);
    run(&rig).await.unwrap();
    run(&rig).await.unwrap();
    assert_eq!(rig.webhook.sent(AlertCondition::FailedLogins), 2);
}
