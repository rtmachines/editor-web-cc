//! Tests of the in-memory `AlertWebhook` (task 3.10, design D6, test row T14).

use super::MemoryWebhook;
use crate::ports::{AlertCondition, AlertWebhook as _, WebhookFault};

#[tokio::test]
async fn every_condition_succeeds_by_default_in_the_order_of_the_argument() {
    let webhook = MemoryWebhook::default();
    let results = webhook
        .send_all(&[AlertCondition::LowDisk, AlertCondition::Panics])
        .await;
    assert_eq!(results, vec![Ok(()), Ok(())]);
}

#[tokio::test]
async fn an_empty_call_gives_an_empty_result_and_is_recorded() {
    let webhook = MemoryWebhook::default();
    assert!(webhook.send_all(&[]).await.is_empty());
    assert_eq!(webhook.calls(), vec![Vec::<AlertCondition>::new()]);
}

#[tokio::test]
async fn the_adapter_records_each_call_with_its_conditions() {
    let webhook = MemoryWebhook::default();
    webhook
        .send_all(&[AlertCondition::LowDisk, AlertCondition::Panics])
        .await;
    webhook.send_all(&[AlertCondition::NotReady]).await;
    assert_eq!(
        webhook.calls(),
        vec![
            vec![AlertCondition::LowDisk, AlertCondition::Panics],
            vec![AlertCondition::NotReady],
        ]
    );
}

#[tokio::test]
async fn a_fault_of_one_condition_leaves_the_other_results_alone() {
    let webhook = MemoryWebhook::default();
    webhook.fail_condition(AlertCondition::LowDisk, WebhookFault::Timeout);
    let results = webhook
        .send_all(&[
            AlertCondition::Panics,
            AlertCondition::LowDisk,
            AlertCondition::NotReady,
        ])
        .await;
    assert_eq!(results, vec![Ok(()), Err(WebhookFault::Timeout), Ok(())]);
}

#[tokio::test]
async fn each_fault_kind_is_returned_as_given() {
    let webhook = MemoryWebhook::default();
    let faults = [
        WebhookFault::Connection,
        WebhookFault::Tls,
        WebhookFault::Timeout,
        WebhookFault::Status { code: 503 },
    ];
    for (condition, fault) in AlertCondition::ALL.into_iter().zip(faults) {
        webhook.fail_condition(condition, fault);
        let results = webhook.send_all(&[condition]).await;
        assert_eq!(results, vec![Err(fault)]);
    }
}

#[tokio::test]
async fn a_fault_on_the_second_send_of_a_condition_hits_that_send_only() {
    let webhook = MemoryWebhook::default();
    webhook.fail_on_call(
        AlertCondition::Panics,
        2,
        WebhookFault::Status { code: 500 },
    );
    assert_eq!(
        webhook.send_all(&[AlertCondition::Panics]).await,
        vec![Ok(())]
    );
    assert_eq!(
        webhook.send_all(&[AlertCondition::Panics]).await,
        vec![Err(WebhookFault::Status { code: 500 })]
    );
    assert_eq!(
        webhook.send_all(&[AlertCondition::Panics]).await,
        vec![Ok(())]
    );
}

#[tokio::test]
async fn the_counter_of_a_condition_ignores_the_other_conditions() {
    let webhook = MemoryWebhook::default();
    webhook
        .send_all(&[AlertCondition::Panics, AlertCondition::LowDisk])
        .await;
    webhook.send_all(&[AlertCondition::Panics]).await;
    assert_eq!(webhook.sent(AlertCondition::Panics), 2);
    assert_eq!(webhook.sent(AlertCondition::LowDisk), 1);
    assert_eq!(webhook.sent(AlertCondition::NotReady), 0);
}

#[tokio::test]
async fn clear_faults_stops_the_faults() {
    let webhook = MemoryWebhook::default();
    webhook.fail_condition(AlertCondition::Panics, WebhookFault::Tls);
    webhook.clear_faults();
    assert_eq!(
        webhook.send_all(&[AlertCondition::Panics]).await,
        vec![Ok(())]
    );
}

#[tokio::test]
async fn a_clone_shares_the_calls() {
    let webhook = MemoryWebhook::default();
    let copy = webhook.clone();
    copy.send_all(&[AlertCondition::Panics]).await;
    assert_eq!(webhook.calls().len(), 1);
}
