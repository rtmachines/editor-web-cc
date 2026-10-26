//! Tests of the schedule, the restart, and the cancel of the supervisor (design D14 row T20).
//!
//! Every test uses paused time, so a test never waits for a real interval. Each test also runs
//! inside a bound of virtual time. A run of the probe writes its start to a channel, so a test
//! reads the exact second of each start.

use std::collections::HashSet;
use std::sync::Arc;

use recipe_app::error::{AppError, TaskStage};
use recipe_app::jobs::RunKind;
use recipe_app::ports::Clock;
use recipe_domain::audit::RequestId;
use tokio::time::{Instant, sleep};
use tokio_util::sync::CancellationToken;

use super::span_tests::lines_with;
use super::support::{
    Behavior, Event, LogCapture, PausedClock, bounded, object_of, probe, secs, settings,
};
use super::{JobName, Supervisor};
use crate::schedule::{Schedule, TimeOfDay};

/// The text that marks a log line of the level `ERROR`.
const ERROR_LEVEL: &str = "\"level\":\"ERROR\"";
/// The text that marks a log line of the level `WARN`.
const WARN_LEVEL: &str = "\"level\":\"WARN\"";

/// A supervisor with a paused clock at midnight and a token that the test cancels.
fn start() -> (Supervisor, CancellationToken, Instant) {
    let clock: Arc<dyn Clock> = PausedClock::at("2026-10-24T00:00:00.000000Z");
    let stop = CancellationToken::new();
    let supervisor = Supervisor::new(settings(), clock, stop.clone());
    (supervisor, stop, Instant::now())
}

fn every(seconds: u64) -> Schedule {
    Schedule::every(secs(seconds)).unwrap()
}

#[tokio::test(start_paused = true)]
async fn a_run_of_90_seconds_with_an_interval_of_60_gives_no_burst() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(90), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Heartbeat, every(60), job);
        sleep(secs(400)).await;
        stop.cancel();
        let outcome = supervisor.drain(secs(100)).await;
        // Start at 60, end at 150, start at 210, end at 300, start at 360, end at 450.
        assert_eq!(seen.starts(), vec![60, 210, 360]);
        assert!(outcome.is_complete());
        assert_eq!(origin.elapsed(), secs(450));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_job_restarts_after_the_restart_delay_when_a_run_panics() {
    bounded(async {
        let logs = LogCapture::start();
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Panic, Behavior::Succeed]);
        supervisor.spawn(JobName::Cleanup, every(60), job);
        sleep(secs(200)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        // The panic at 60 gives the next start after the restart delay, then the interval again.
        assert_eq!(seen.starts(), vec![60, 70, 130, 190]);
        let panics: Vec<String> = logs
            .lines()
            .into_iter()
            .filter(|line| line.contains("panicked"))
            .collect();
        assert_eq!(panics.len(), 1, "{panics:?}");
        assert!(panics[0].contains("\"job\":\"cleanup\""), "{panics:?}");
        assert!(!panics[0].contains("request_id"), "{panics:?}");
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_job_goes_on_after_an_error_and_the_supervisor_logs_it_once() {
    bounded(async {
        let logs = LogCapture::start();
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Fail, Behavior::Succeed]);
        supervisor.spawn(JobName::Cleanup, every(60), job);
        sleep(secs(130)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        // An error does not shorten the wait: the next run starts one interval after the end.
        assert_eq!(seen.starts(), vec![60, 120]);
        let errors = lines_with(&logs.lines(), ERROR_LEVEL);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("\"job\":\"cleanup\""), "{errors:?}");
        assert!(errors[0].contains("E-WEB-004"), "{errors:?}");
        assert!(!errors[0].contains("request_id"), "{errors:?}");
    })
    .await;
}

/// Run one job `name` whose first run fails. Return the lines that the capture holds.
async fn lines_of_failed_first_run(name: JobName) -> Vec<String> {
    let logs = LogCapture::start();
    let (mut supervisor, stop, origin) = start();
    let (job, _seen) = probe(origin, secs(0), &[Behavior::Fail, Behavior::Succeed]);
    supervisor.spawn(name, every(60), job);
    sleep(secs(90)).await;
    stop.cancel();
    assert!(supervisor.drain(secs(5)).await.is_complete());
    logs.lines()
}

// ADR-0089 (d): the only `Err` of `alerts` is a failed snapshot, a handled fault.
#[tokio::test(start_paused = true)]
async fn a_failed_run_of_alerts_gives_one_warn_line_and_no_error_line() {
    bounded(async {
        let lines = lines_of_failed_first_run(JobName::Alerts).await;
        let warns = lines_with(&lines, WARN_LEVEL);
        assert_eq!(warns.len(), 1, "{lines:?}");
        assert!(lines_with(&lines, ERROR_LEVEL).is_empty(), "{lines:?}");
        let span = object_of(&warns[0], "span");
        assert!(span.contains("\"job\":\"alerts\""), "{warns:?}");
        assert!(span.contains("\"iteration\":1"), "{warns:?}");
        assert!(span.contains("\"outcome\":\"E-WEB-004\""), "{warns:?}");
        let fields = object_of(&warns[0], "fields");
        assert!(fields.contains("\"error_code\":\"E-WEB-004\""), "{warns:?}");
        assert!(!warns[0].contains("request_id"), "{warns:?}");
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_failed_run_of_another_job_still_gives_one_error_line_and_no_warn_line() {
    bounded(async {
        let lines = lines_of_failed_first_run(JobName::Cleanup).await;
        let errors = lines_with(&lines, ERROR_LEVEL);
        assert_eq!(errors.len(), 1, "{lines:?}");
        assert!(lines_with(&lines, WARN_LEVEL).is_empty(), "{lines:?}");
        let span = object_of(&errors[0], "span");
        assert!(span.contains("\"job\":\"cleanup\""), "{errors:?}");
        assert!(span.contains("\"outcome\":\"E-WEB-004\""), "{errors:?}");
        let fields = object_of(&errors[0], "fields");
        assert!(
            fields.contains("\"error_code\":\"E-WEB-004\""),
            "{errors:?}"
        );
    })
    .await;
}

/// A source of the `request_id` that fails like a missing random source of the system.
fn failing_request_id() -> Result<RequestId, AppError> {
    Err(AppError::TaskFailed {
        stage: TaskStage::Token,
    })
}

// ADR-0089 (d): `WARN` is only for a failed snapshot read. A failed `request_id` is a fault of
// the supervisor, so the job `alerts` gets `ERROR` as well.
#[tokio::test(start_paused = true)]
async fn a_failed_request_id_gives_one_error_line_also_for_alerts() {
    bounded(async {
        let logs = LogCapture::start();
        let (supervisor, stop, origin) = start();
        let mut supervisor = supervisor.with_request_ids(failing_request_id);
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Alerts, every(60), job);
        sleep(secs(90)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        // The run never starts, so the probe sees no start.
        assert!(seen.starts().is_empty());
        let lines = logs.lines();
        let errors = lines_with(&lines, ERROR_LEVEL);
        assert_eq!(errors.len(), 1, "{lines:?}");
        assert!(lines_with(&lines, WARN_LEVEL).is_empty(), "{lines:?}");
        let span = object_of(&errors[0], "span");
        assert!(span.contains("\"job\":\"alerts\""), "{errors:?}");
        assert!(span.contains("\"outcome\":\"E-WEB-011\""), "{errors:?}");
        let fields = object_of(&errors[0], "fields");
        assert!(
            fields.contains("\"error_code\":\"E-WEB-011\""),
            "{errors:?}"
        );
        assert!(!errors[0].contains("request_id"), "{errors:?}");
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_cancel_during_the_wait_stops_the_schedule_at_once() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Heartbeat, every(60), job);
        sleep(secs(30)).await;
        stop.cancel();
        let outcome = supervisor.drain(secs(100)).await;
        assert!(outcome.is_complete());
        assert_eq!(origin.elapsed(), secs(30));
        assert!(seen.starts().is_empty());
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_cancel_during_the_restart_wait_stops_the_restart() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Panic, Behavior::Succeed]);
        supervisor.spawn(JobName::Cleanup, every(60), job);
        sleep(secs(65)).await;
        stop.cancel();
        let outcome = supervisor.drain(secs(100)).await;
        assert!(outcome.is_complete());
        assert_eq!(origin.elapsed(), secs(65));
        assert_eq!(seen.starts(), vec![60]);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_cancel_during_a_run_lets_the_run_end_and_starts_no_other_run() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(100), &[Behavior::Succeed]);
        supervisor.spawn(JobName::AuditChainCheck, every(60), job);
        sleep(secs(100)).await;
        stop.cancel();
        let outcome = supervisor.drain(secs(200)).await;
        assert!(outcome.is_complete());
        // The run from 60 to 160 is not cut. The drain returns when it ends.
        assert_eq!(origin.elapsed(), secs(160));
        let events = seen.events();
        assert!(matches!(events.last(), Some(Event::Ended(end)) if *end == secs(160)));
        assert_eq!(events.len(), 2);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_run_gets_its_own_request_id() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Heartbeat, every(10), job);
        sleep(secs(55)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        let ids: Vec<_> = seen
            .events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Started(_, id, _) => Some(id),
                _ => None,
            })
            .collect();
        assert_eq!(ids.len(), 5);
        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 5);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_job_that_runs_at_once_runs_again_after_one_interval() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        let schedule = Schedule::now_then_every(secs(3_600)).unwrap();
        supervisor.spawn(JobName::ClockCheck, schedule, job);
        sleep(secs(7_300)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        assert_eq!(seen.starts(), vec![0, 3_600, 7_200]);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_run_gets_the_kind_of_the_trigger_that_fired_it() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        let schedule = every(60).with_once_after(secs(10));
        supervisor.spawn(JobName::AuditChainCheck, schedule, job);
        sleep(secs(140)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        // The one-shot run starts at 10. The interval runs start at 70 and 130.
        let kinds = seen.kinds();
        assert_eq!(
            kinds,
            vec![RunKind::CatchUp, RunKind::Scheduled, RunKind::Scheduled]
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn an_interval_run_gets_the_scheduled_kind() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        supervisor.spawn(JobName::Heartbeat, every(60), job);
        sleep(secs(130)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        assert_eq!(seen.kinds(), vec![RunKind::Scheduled, RunKind::Scheduled]);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_restart_after_a_panic_keeps_the_kind_of_the_run_that_panicked() {
    bounded(async {
        let (mut supervisor, stop, origin) = start();
        let script = [Behavior::Panic, Behavior::Succeed];
        let (job, mut seen) = probe(origin, secs(0), &script);
        let schedule = every(60).with_once_after(secs(10));
        supervisor.spawn(JobName::AuditChainCheck, schedule, job);
        sleep(secs(100)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        // The one-shot run panics at 10. The restart at 20 repeats its kind. The interval run
        // at 80 is a scheduled run again.
        assert_eq!(
            seen.kinds(),
            vec![RunKind::CatchUp, RunKind::CatchUp, RunKind::Scheduled]
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_daily_job_runs_at_its_time_of_day_in_utc() {
    bounded(async {
        let clock: Arc<dyn Clock> = PausedClock::at("2026-10-24T02:59:30.000000Z");
        let stop = CancellationToken::new();
        let mut supervisor = Supervisor::new(settings(), clock, stop.clone());
        let origin = Instant::now();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        let three = TimeOfDay::parse(3, 0).unwrap();
        supervisor.spawn(JobName::AuditChainCheck, Schedule::daily_at(three), job);
        sleep(secs(100_000)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        // At 03:00:00 and one day later.
        assert_eq!(seen.starts(), vec![30, 86_430]);
    })
    .await;
}
