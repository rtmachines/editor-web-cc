//! Tests of a counter that grows while the job waits for the webhook (ADR-0073 (g), ADR-0060 (aa)).
//!
//! A run reads the counters once, before its awaited calls. An event that arrives during the
//! webhook call is not in the totals of that run. The next run must count it once.

use std::sync::atomic::Ordering;

use tokio::time::{Duration, timeout};

use super::support::{Gate, Rig, at, gated_rig, rig, run, sent_by_one_run, set_time};
use crate::jobs::alerts::NEVER;
use crate::ports::AlertCondition;

/// The default `alerts.panic_window` in seconds: 10 minutes.
const PANIC_WINDOW_SECONDS: i64 = 600;

/// The longest wait for a gate in a test. Paused time is not needed, because the gate is local.
const GATE_WAIT: Duration = Duration::from_secs(5);

/// Make `SecurityEvents` active, so a run calls the webhook.
fn raise_security_alert(rig: &Rig) {
    rig.operations.edit(|snapshot| snapshot.security_alerts = 1);
}

/// Make `SecurityEvents` inactive again.
fn clear_security_alert(rig: &Rig) {
    rig.operations.edit(|snapshot| snapshot.security_alerts = 0);
}

/// Run the job once. Call `during` while the webhook holds the call of that run.
async fn run_and_change_during_the_call(rig: &Rig, gate: &Gate, during: impl FnOnce()) {
    let control = async {
        timeout(GATE_WAIT, gate.wait_reached()).await.unwrap();
        during();
        gate.release();
    };
    let (result, ()) = tokio::join!(run(rig), control);
    result.unwrap();
}

#[tokio::test]
async fn a_mismatch_during_the_webhook_call_is_not_alerted_by_that_run() {
    let (rig, gate) = gated_rig();
    raise_security_alert(&rig);
    run_and_change_during_the_call(&rig, &gate, || {
        rig.counters.record_export_hash_mismatch();
    })
    .await;
    assert_eq!(
        rig.webhook.calls(),
        vec![vec![AlertCondition::SecurityEvents]]
    );
    assert_eq!(
        rig.job.state.export_hash_mismatches.load(Ordering::Relaxed),
        0
    );
}

#[tokio::test]
async fn a_mismatch_during_the_webhook_call_is_alerted_once_by_the_next_run() {
    let (rig, gate) = gated_rig();
    raise_security_alert(&rig);
    run_and_change_during_the_call(&rig, &gate, || {
        rig.counters.record_export_hash_mismatch();
    })
    .await;
    clear_security_alert(&rig);
    assert_eq!(
        sent_by_one_run(&rig).await,
        vec![AlertCondition::ExportHashMismatch]
    );
    assert_eq!(
        rig.job.state.export_hash_mismatches.load(Ordering::Relaxed),
        1
    );
    assert!(sent_by_one_run(&rig).await.is_empty());
}

#[tokio::test]
async fn panics_during_the_webhook_call_are_not_counted_by_that_run() {
    let (rig, gate) = gated_rig();
    raise_security_alert(&rig);
    run_and_change_during_the_call(&rig, &gate, || {
        for _ in 0..4 {
            rig.counters.record_panic();
        }
    })
    .await;
    assert_eq!(
        rig.webhook.calls(),
        vec![vec![AlertCondition::SecurityEvents]]
    );
    assert_eq!(rig.job.state.panics.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn panics_during_the_webhook_call_are_counted_once_by_the_next_run() {
    let (rig, gate) = gated_rig();
    raise_security_alert(&rig);
    run_and_change_during_the_call(&rig, &gate, || {
        for _ in 0..4 {
            rig.counters.record_panic();
        }
    })
    .await;
    clear_security_alert(&rig);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    assert_eq!(rig.job.state.panics.load(Ordering::Relaxed), 4);
    // The window ends. A second count of the four panics would make `panics` active again.
    set_time(&rig, PANIC_WINDOW_SECONDS + 1);
    assert_eq!(sent_by_one_run(&rig).await, Vec::new());
}

/// Run the job until it holds a call at `gate`. Then drop the run, as a stop of the supervisor does.
async fn drop_run_at(rig: &Rig, gate: &Gate) {
    let first = run(rig);
    tokio::pin!(first);
    let ended = tokio::select! {
        _ = &mut first => true,
        () = gate.wait_reached() => false,
    };
    assert!(!ended, "the run ended before the held call");
}

/// Count `count` panics.
fn count_panics(rig: &Rig, count: u32) {
    for _ in 0..count {
        rig.counters.record_panic();
    }
}

/// Return the panic slots as seconds after the start of the test. A free slot is `None`.
fn slots(rig: &Rig) -> Vec<Option<i64>> {
    let base = at(0).as_offset_date_time().unix_timestamp();
    rig.job
        .state
        .panic_times
        .iter()
        .map(|slot| slot.load(Ordering::Relaxed))
        .map(|time| (time != NEVER).then(|| time.checked_sub(base).unwrap()))
        .collect()
}

/// Return the slot that the next panic takes.
fn cursor(rig: &Rig) -> usize {
    rig.job.state.next_slot.load(Ordering::Relaxed)
}

#[tokio::test]
async fn a_run_dropped_at_the_webhook_call_does_not_count_its_panics_twice() {
    let (rig, gate) = gated_rig();
    raise_security_alert(&rig);
    count_panics(&rig, 2);
    drop_run_at(&rig, &gate).await;
    clear_security_alert(&rig);
    // Two panics are not more than three. The dropped run must not make a third and a fourth.
    assert_eq!(sent_by_one_run(&rig).await, Vec::new());
    assert_eq!(slots(&rig), vec![Some(0), Some(0), None, None]);
    assert_eq!(cursor(&rig), 2);
    assert_eq!(rig.job.state.panics.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn a_panic_after_a_run_dropped_at_the_webhook_call_is_counted_once_and_not_lost() {
    let (rig, gate) = gated_rig();
    raise_security_alert(&rig);
    count_panics(&rig, 2);
    drop_run_at(&rig, &gate).await;
    clear_security_alert(&rig);
    set_time(&rig, 100);
    count_panics(&rig, 1);
    assert_eq!(sent_by_one_run(&rig).await, Vec::new());
    assert_eq!(slots(&rig), vec![Some(0), Some(0), Some(100), None]);
    set_time(&rig, 200);
    count_panics(&rig, 1);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    assert_eq!(slots(&rig), vec![Some(0), Some(0), Some(100), Some(200)]);
    assert_eq!(cursor(&rig), 0);
}

#[tokio::test]
async fn a_run_dropped_at_the_snapshot_read_loses_no_panic() {
    let rig = rig();
    let gate = rig.operations.gate();
    gate.arm();
    count_panics(&rig, 2);
    drop_run_at(&rig, &gate).await;
    assert_eq!(slots(&rig), vec![None; 4]);
    assert_eq!(cursor(&rig), 0);
    assert_eq!(rig.job.state.panics.load(Ordering::Relaxed), 0);
    set_time(&rig, 100);
    count_panics(&rig, 2);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    assert_eq!(slots(&rig), vec![Some(100); 4]);
    assert_eq!(rig.webhook.calls(), vec![vec![AlertCondition::Panics]]);
}

#[tokio::test]
async fn a_run_dropped_twice_in_a_row_counts_each_panic_once() {
    let (rig, gate) = gated_rig();
    raise_security_alert(&rig);
    count_panics(&rig, 2);
    drop_run_at(&rig, &gate).await;
    gate.arm();
    set_time(&rig, 100);
    count_panics(&rig, 1);
    drop_run_at(&rig, &gate).await;
    assert_eq!(slots(&rig), vec![Some(0), Some(0), Some(100), None]);
    assert_eq!(cursor(&rig), 3);
    clear_security_alert(&rig);
    set_time(&rig, 200);
    count_panics(&rig, 1);
    assert_eq!(sent_by_one_run(&rig).await, vec![AlertCondition::Panics]);
    assert_eq!(slots(&rig), vec![Some(0), Some(0), Some(100), Some(200)]);
    assert_eq!(rig.job.state.panics.load(Ordering::Relaxed), 4);
    // Only the last run reached the webhook. The two dropped runs sent nothing.
    assert_eq!(rig.webhook.calls(), vec![vec![AlertCondition::Panics]]);
}
