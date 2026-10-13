use recipe_domain::audit::{RequestId, SecurityEvent, UserAgent};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::UserId;
use uuid::Uuid;

use super::MemorySink;
use crate::memory::Fault;
use crate::ports::SecurityEventSink;

fn event(n: u128) -> SecurityEvent {
    SecurityEvent::forbidden(
        UserId::from(Uuid::from_u128(n)),
        "CreateMachine",
        RequestId::from(Uuid::from_u128(n)),
        None,
        Some(UserAgent::from("test")),
    )
}

#[tokio::test]
async fn the_sink_keeps_two_events_in_order() {
    let sink = MemorySink::default();
    sink.record(event(1)).await.unwrap();
    sink.record(event(2)).await.unwrap();
    assert_eq!(sink.events(), vec![event(1), event(2)]);
}

#[tokio::test]
async fn a_new_sink_has_no_events() {
    assert!(MemorySink::default().events().is_empty());
}

#[tokio::test]
async fn the_failing_sink_returns_e_db_005_and_keeps_nothing() {
    let sink = MemorySink::default();
    sink.fail_with(Fault::Unavailable);
    let error = sink.record(event(1)).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert!(sink.events().is_empty());
}

#[tokio::test]
async fn the_sink_records_again_after_the_failure_clears() {
    let sink = MemorySink::default();
    sink.fail_with(Fault::Unavailable);
    sink.record(event(1)).await.unwrap_err();
    sink.clear_failure();
    sink.record(event(2)).await.unwrap();
    assert_eq!(sink.events(), vec![event(2)]);
}

#[tokio::test]
async fn a_clone_shares_the_events() {
    let sink = MemorySink::default();
    let copy = sink.clone();
    sink.record(event(1)).await.unwrap();
    assert_eq!(copy.events(), vec![event(1)]);
}
