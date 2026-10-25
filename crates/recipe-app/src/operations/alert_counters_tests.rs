use std::thread;

use super::AlertCounters;

fn assert_shareable<T: Clone + Send + Sync + 'static>() {}

#[test]
fn counters_are_cheap_to_clone_and_shareable_between_threads() {
    assert_shareable::<AlertCounters>();
}

#[test]
fn both_counters_start_at_zero() {
    let counters = AlertCounters::default();
    assert_eq!(counters.panics(), 0);
    assert_eq!(counters.export_hash_mismatches(), 0);
}

#[test]
fn record_panic_counts_a_panic_and_no_export_failure() {
    let counters = AlertCounters::default();
    counters.record_panic();
    counters.record_panic();
    assert_eq!(counters.panics(), 2);
    assert_eq!(counters.export_hash_mismatches(), 0);
}

#[test]
fn record_export_hash_mismatch_counts_one_result_and_no_panic() {
    let counters = AlertCounters::default();
    counters.record_export_hash_mismatch();
    assert_eq!(counters.export_hash_mismatches(), 1);
    assert_eq!(counters.panics(), 0);
}

/// Fixes the closure bounds of a panic hook (`Fn` + `Send` + `Sync` + `'static`).
fn as_panic_hook<F: Fn() + Send + Sync + 'static>(hook: F) -> F {
    hook
}

#[test]
fn a_clone_in_a_panic_hook_closure_counts_into_the_same_total_on_another_thread() {
    let counters = AlertCounters::default();
    let hook_counters = counters.clone();
    let hook = as_panic_hook(move || hook_counters.record_panic());
    counters.record_panic();
    thread::spawn(move || {
        hook();
        hook();
    })
    .join()
    .unwrap();
    assert_eq!(counters.panics(), 3);
    assert_eq!(counters.export_hash_mismatches(), 0);
}

#[test]
fn clones_share_both_counters() {
    let first = AlertCounters::default();
    let second = first.clone();
    first.record_panic();
    second.record_export_hash_mismatch();
    assert_eq!(second.panics(), 1);
    assert_eq!(first.export_hash_mismatches(), 1);
}

#[test]
fn threads_lose_no_increment() {
    let counters = AlertCounters::default();
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let counters = counters.clone();
            thread::spawn(move || {
                for _ in 0..250 {
                    counters.record_panic();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(counters.panics(), 1000);
}
