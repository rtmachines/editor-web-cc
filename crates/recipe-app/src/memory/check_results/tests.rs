//! Tests of the in-memory `CheckResults` (ADR-0151 (j3), (r), design D5).

use std::time::Duration;

use super::MemoryCheckResults;
use crate::memory::testkit::at;
use crate::ports::{
    CheckOutcome, CheckResult, CheckResultFault, CheckResults as _, ScheduledCheck,
};

const CHECKS: [ScheduledCheck; 2] = [ScheduledCheck::RestoreTest, ScheduledCheck::BaseBackup];

fn passed(seconds: i64) -> CheckResult {
    CheckResult {
        outcome: CheckOutcome::Passed,
        finished_at: at(seconds),
    }
}

#[tokio::test]
async fn a_new_adapter_gives_no_result_for_each_check() {
    let results = MemoryCheckResults::default();
    for check in CHECKS {
        assert_eq!(results.newest(check).await.unwrap(), None);
    }
}

#[tokio::test]
async fn a_set_result_comes_back_for_its_check_only() {
    let results = MemoryCheckResults::default();
    results.set_result(ScheduledCheck::RestoreTest, Some(passed(100)));
    assert_eq!(
        results.newest(ScheduledCheck::RestoreTest).await.unwrap(),
        Some(passed(100))
    );
    assert_eq!(
        results.newest(ScheduledCheck::BaseBackup).await.unwrap(),
        None
    );
    let failed = CheckResult {
        outcome: CheckOutcome::Failed,
        finished_at: at(200),
    };
    results.set_result(ScheduledCheck::BaseBackup, Some(failed));
    assert_eq!(
        results.newest(ScheduledCheck::BaseBackup).await.unwrap(),
        Some(failed)
    );
    assert_eq!(
        results.newest(ScheduledCheck::RestoreTest).await.unwrap(),
        Some(passed(100))
    );
    results.set_result(ScheduledCheck::RestoreTest, None);
    assert_eq!(
        results.newest(ScheduledCheck::RestoreTest).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn an_unreadable_check_is_a_fault_and_not_none() {
    let results = MemoryCheckResults::default();
    results.set_unreadable(ScheduledCheck::BaseBackup);
    let fault = results
        .newest(ScheduledCheck::BaseBackup)
        .await
        .unwrap_err();
    assert!(matches!(
        fault,
        CheckResultFault::Unreadable {
            check: ScheduledCheck::BaseBackup,
            ..
        }
    ));
    assert_eq!(
        results.newest(ScheduledCheck::RestoreTest).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn set_result_ends_the_unreadable_state() {
    let results = MemoryCheckResults::default();
    results.set_unreadable(ScheduledCheck::RestoreTest);
    results.set_result(ScheduledCheck::RestoreTest, Some(passed(5)));
    assert_eq!(
        results.newest(ScheduledCheck::RestoreTest).await.unwrap(),
        Some(passed(5))
    );
}

#[tokio::test]
async fn a_fault_on_the_second_call_hits_that_call_only() {
    let results = MemoryCheckResults::default();
    results.set_result(ScheduledCheck::RestoreTest, Some(passed(5)));
    results.fail_on_call(ScheduledCheck::RestoreTest, 2);
    results.newest(ScheduledCheck::RestoreTest).await.unwrap();
    let fault = results
        .newest(ScheduledCheck::RestoreTest)
        .await
        .unwrap_err();
    assert!(matches!(
        fault,
        CheckResultFault::Unreadable {
            check: ScheduledCheck::RestoreTest,
            ..
        }
    ));
    results.newest(ScheduledCheck::RestoreTest).await.unwrap();
    assert_eq!(results.calls(ScheduledCheck::RestoreTest), 3);
    assert_eq!(results.calls(ScheduledCheck::BaseBackup), 0);
}

#[tokio::test]
async fn a_fault_from_a_call_on_lasts_until_it_is_cleared() {
    let results = MemoryCheckResults::default();
    results.fail_from_call(ScheduledCheck::BaseBackup, 1);
    assert!(results.newest(ScheduledCheck::BaseBackup).await.is_err());
    assert!(results.newest(ScheduledCheck::BaseBackup).await.is_err());
    assert_eq!(
        results.newest(ScheduledCheck::RestoreTest).await.unwrap(),
        None
    );
    results.clear_faults();
    assert_eq!(
        results.newest(ScheduledCheck::BaseBackup).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn a_clone_shares_the_values() {
    let results = MemoryCheckResults::default();
    let copy = results.clone();
    copy.set_result(ScheduledCheck::BaseBackup, Some(passed(1)));
    assert_eq!(
        results.newest(ScheduledCheck::BaseBackup).await.unwrap(),
        Some(passed(1))
    );
    assert_eq!(copy.calls(ScheduledCheck::BaseBackup), 1);
}

#[tokio::test(start_paused = true)]
async fn a_planned_delay_makes_newest_return_after_that_delay() {
    let results = MemoryCheckResults::default();
    results.set_delay(Duration::from_secs(30));
    let early = tokio::time::timeout(
        Duration::from_secs(29),
        results.newest(ScheduledCheck::RestoreTest),
    )
    .await;
    assert!(early.is_err());
    let started = tokio::time::Instant::now();
    let late = tokio::time::timeout(
        Duration::from_secs(31),
        results.newest(ScheduledCheck::RestoreTest),
    )
    .await;
    assert_eq!(late.unwrap().unwrap(), None);
    assert_eq!(started.elapsed(), Duration::from_secs(30));
}

#[tokio::test(start_paused = true)]
async fn no_delay_is_planned_by_default() {
    let results = MemoryCheckResults::default();
    let started = tokio::time::Instant::now();
    results.newest(ScheduledCheck::BaseBackup).await.unwrap();
    assert_eq!(started.elapsed(), Duration::ZERO);
}
