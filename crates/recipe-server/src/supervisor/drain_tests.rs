//! Tests of `drain(deadline)` and of the rule that two runs of one job never overlap (row T20).
//!
//! After the deadline, the drain aborts the open runs and does not wait for them (ADR-0060 (j),
//! ADR-0087 (d)). The tests use paused time and prove the "no wait" with the time since the start.

use std::sync::Arc;

use recipe_app::ports::Clock;
use tokio::time::{Instant, sleep};
use tokio_util::sync::CancellationToken;

use super::support::{Behavior, Event, PausedClock, bounded, probe, secs, settings};
use super::{JobName, Supervisor};
use crate::schedule::{Schedule, TimeOfDay};

fn start() -> (Supervisor, CancellationToken, Instant) {
    let clock: Arc<dyn Clock> = PausedClock::at("2026-10-24T00:00:00.000000Z");
    let stop = CancellationToken::new();
    (
        Supervisor::new(settings(), clock, stop.clone()),
        stop,
        Instant::now(),
    )
}

fn every(seconds: u64) -> Schedule {
    Schedule::every(secs(seconds)).unwrap()
}

#[tokio::test(start_paused = true)]
async fn a_run_that_never_ends_is_cut_at_the_deadline_and_drain_does_not_wait() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Hang]);
        supervisor.spawn(JobName::Alerts, every(60), job);
        sleep(secs(70)).await;
        stop.cancel();
        let outcome = supervisor.drain(secs(30)).await;
        assert_eq!(outcome.cut(), 1);
        assert!(!outcome.is_complete());
        // The drain returns at the deadline (70 + 30) and not later.
        assert_eq!(origin.elapsed(), secs(100));
        // The current-thread runtime drops an aborted run only after the test yields. A drain
        // that waited for the aborted runs would return after the drop, and this check fails.
        let at_return = seen.events();
        assert!(
            !at_return
                .iter()
                .any(|event| matches!(event, Event::Dropped(_))),
            "{at_return:?}"
        );
        // The abort drops the future of the run. An open transaction then rolls back.
        sleep(secs(1)).await;
        let after_yield = seen.events();
        assert!(
            matches!(after_yield.last(), Some(Event::Dropped(_))),
            "{after_yield:?}"
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_run_that_ends_before_the_deadline_is_not_counted_as_cut() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, _seen) = probe(origin, secs(20), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Alerts, every(60), job);
        sleep(secs(70)).await;
        stop.cancel();
        let outcome = supervisor.drain(secs(30)).await;
        assert!(outcome.is_complete());
        assert_eq!(outcome.cut(), 0);
        // The run from 60 to 80 ends first. The drain does not wait for the deadline.
        assert_eq!(origin.elapsed(), secs(80));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_panicked_run_leaves_no_open_count() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        // The first run panics at 60 s. The restart starts a second run at 70 s, and it hangs.
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Panic, Behavior::Hang]);
        supervisor.spawn(JobName::Alerts, every(60), job);
        sleep(secs(80)).await;
        assert_eq!(seen.starts(), vec![60, 70]);
        stop.cancel();
        // A count that the panic did not release would give 2 and not 1.
        let outcome = supervisor.drain(secs(10)).await;
        assert_eq!(outcome.cut(), 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_cut_count_covers_each_open_run_and_no_idle_job() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (first, _first_seen) = probe(origin, secs(0), &[Behavior::Hang]);
        let (second, _second_seen) = probe(origin, secs(0), &[Behavior::Hang]);
        let (idle, mut idle_seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Cleanup, every(60), first);
        supervisor.spawn(JobName::Alerts, every(60), second);
        supervisor.spawn(JobName::Heartbeat, every(1_000), idle);
        sleep(secs(70)).await;
        stop.cancel();
        // A deadline of zero keeps the idle loop in the set: three loops, two open runs.
        // A count of the loops would give 3 and not 2.
        let outcome = supervisor.drain(secs(0)).await;
        assert_eq!(outcome.cut(), 2);
        assert!(idle_seen.starts().is_empty());
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_drain_of_zero_seconds_cuts_the_open_run_at_once() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, _seen) = probe(origin, secs(0), &[Behavior::Hang]);
        supervisor.spawn(JobName::Cleanup, every(60), job);
        sleep(secs(70)).await;
        stop.cancel();
        let outcome = supervisor.drain(secs(0)).await;
        assert_eq!(outcome.cut(), 1);
        assert_eq!(origin.elapsed(), secs(70));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn drain_stops_the_jobs_and_leaves_the_token_of_the_caller_alone() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Heartbeat, every(60), job);
        sleep(secs(10)).await;
        let outcome = supervisor.drain(secs(30)).await;
        assert!(outcome.is_complete());
        assert_eq!(origin.elapsed(), secs(10));
        assert!(seen.starts().is_empty());
        // The supervisor owns a child token. It never cancels the token that the caller owns.
        assert!(!stop.is_cancelled());
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_cancel_of_the_token_of_the_caller_stops_the_loops_without_a_drain() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Heartbeat, every(60), job);
        sleep(secs(10)).await;
        stop.cancel();
        // The loop would start a run at 60 s if the cancel had not reached it.
        sleep(secs(100)).await;
        assert!(seen.starts().is_empty());
        assert!(supervisor.drain(secs(30)).await.is_complete());
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_nightly_run_and_the_catch_up_never_overlap() {
    bounded(async {
        // The clock shows 02:59:50, so the nightly run starts after 10 s and lasts 1 000 s.
        // The catch-up is due after 100 s, while the nightly run is open.
        let clock: Arc<dyn Clock> = PausedClock::at("2026-10-24T02:59:50.000000Z");
        let stop = CancellationToken::new();
        let mut supervisor = Supervisor::new(settings(), clock, stop.clone());
        let origin = Instant::now();
        let (job, mut seen) = probe(origin, secs(1_000), &[Behavior::Succeed]);
        let schedule =
            Schedule::daily_at(TimeOfDay::parse(3, 0).unwrap()).with_once_after(secs(100));
        supervisor.spawn(JobName::AuditChainCheck, schedule, job);
        sleep(secs(3_000)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        // The catch-up waits for the end of the nightly run (1 010) and then starts at once.
        assert_eq!(seen.starts(), vec![10, 1_010]);
        assert_eq!(seen.overlaps(), 0);
    })
    .await;
}
