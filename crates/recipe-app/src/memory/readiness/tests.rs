//! Tests of the in-memory `ReadinessProbe` (task 3.10, design D6, test row T14).

use std::time::Duration;

use super::MemoryReadiness;
use crate::ports::{NotReadyCause, Readiness, ReadinessProbe as _};

const TIMEOUT: Duration = Duration::from_secs(2);

#[tokio::test]
async fn a_new_probe_is_ready() {
    assert_eq!(
        MemoryReadiness::default().check(TIMEOUT).await,
        Readiness::Ready
    );
}

#[tokio::test]
async fn the_probe_returns_each_readiness_value() {
    let probe = MemoryReadiness::default();
    let values = [
        Readiness::Ready,
        Readiness::NotReady(NotReadyCause::Unreachable),
        Readiness::NotReady(NotReadyCause::TimedOut),
        Readiness::NotReady(NotReadyCause::SchemaMismatch),
    ];
    for value in values {
        probe.set(value);
        assert_eq!(probe.check(TIMEOUT).await, value);
    }
}

#[tokio::test]
async fn the_probe_records_the_timeout_of_each_call() {
    let probe = MemoryReadiness::default();
    probe.check(Duration::from_secs(1)).await;
    probe.check(Duration::from_secs(3)).await;
    assert_eq!(
        probe.timeouts(),
        vec![Duration::from_secs(1), Duration::from_secs(3)]
    );
    assert_eq!(probe.calls(), 2);
}

#[tokio::test]
async fn a_fault_on_the_second_call_gives_one_not_ready_result() {
    let probe = MemoryReadiness::default();
    probe.fail_on_call(2, NotReadyCause::TimedOut);
    assert_eq!(probe.check(TIMEOUT).await, Readiness::Ready);
    assert_eq!(
        probe.check(TIMEOUT).await,
        Readiness::NotReady(NotReadyCause::TimedOut)
    );
    assert_eq!(probe.check(TIMEOUT).await, Readiness::Ready);
}

#[tokio::test]
async fn a_fault_from_a_call_on_lasts_until_it_is_cleared() {
    let probe = MemoryReadiness::default();
    probe.fail_from_call(1, NotReadyCause::Unreachable);
    for _ in 0..2 {
        assert_eq!(
            probe.check(TIMEOUT).await,
            Readiness::NotReady(NotReadyCause::Unreachable)
        );
    }
    probe.clear_faults();
    assert_eq!(probe.check(TIMEOUT).await, Readiness::Ready);
}

#[tokio::test]
async fn a_fault_wins_over_the_set_value() {
    let probe = MemoryReadiness::default();
    probe.set(Readiness::NotReady(NotReadyCause::SchemaMismatch));
    probe.fail_on_call(1, NotReadyCause::TimedOut);
    assert_eq!(
        probe.check(TIMEOUT).await,
        Readiness::NotReady(NotReadyCause::TimedOut)
    );
    assert_eq!(
        probe.check(TIMEOUT).await,
        Readiness::NotReady(NotReadyCause::SchemaMismatch)
    );
}

#[tokio::test]
async fn a_clone_shares_the_value_and_the_counter() {
    let probe = MemoryReadiness::default();
    let copy = probe.clone();
    copy.set(Readiness::NotReady(NotReadyCause::Unreachable));
    assert!(!probe.check(TIMEOUT).await.is_ready());
    assert_eq!(copy.calls(), 1);
}
