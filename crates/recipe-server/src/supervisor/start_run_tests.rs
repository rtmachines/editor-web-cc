//! Tests of `Supervisor::run_at_start`, the first run of a job before the loops start (ADR-0089 (b)).
//!
//! Every test uses paused time. The log tests read the objects `span` and `fields` of the JSON
//! lines. The outcome tests read the span itself through the layer of `span_tests`.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};

use recipe_app::error::AppError;
use recipe_app::jobs::RunKind;
use recipe_app::ports::Clock;
use recipe_domain::audit::RequestId;
use tokio::time::{Instant, sleep};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::Registry;

use super::span_tests::{Noisy, Seen, SpanRecorder, WARN_TEXT, lines_with};
use super::support::{
    Behavior, Event, LogCapture, PausedClock, bounded, object_of, probe, secs, set_test_default,
    settings,
};
use super::{JobName, JobRun, StartRunFault, Supervisor};
use crate::schedule::Schedule;

/// A supervisor with a paused clock at midnight and a token that the test cancels.
fn start() -> (Supervisor, CancellationToken) {
    let clock: Arc<dyn Clock> = PausedClock::at("2026-10-24T00:00:00.000000Z");
    let stop = CancellationToken::new();
    (Supervisor::new(settings(), clock, stop.clone()), stop)
}

/// Run `Noisy` with `script` once as the start run of `DiskCheck`.
async fn start_once(script: &[Behavior]) -> Result<(), StartRunFault> {
    let (mut supervisor, _stop) = start();
    let job = Arc::new(Noisy::new(script));
    supervisor.run_at_start(JobName::DiskCheck, &job).await
}

/// Run `start_once` under the span layer and return the closed spans `job`.
async fn spans_of_start_once(script: &[Behavior]) -> (Result<(), StartRunFault>, Vec<Seen>) {
    let (sender, receiver): (_, Receiver<Seen>) = channel();
    let _guard = set_test_default(Registry::default().with(SpanRecorder(sender)));
    let result = start_once(script).await;
    (result, receiver.try_iter().collect())
}

#[tokio::test(start_paused = true)]
async fn the_warn_line_of_the_start_run_gets_job_and_iteration_1_from_the_span() {
    bounded(async {
        let logs = LogCapture::start();
        assert!(start_once(&[Behavior::Succeed]).await.is_ok());
        let warns = lines_with(&logs.lines(), WARN_TEXT);
        assert_eq!(warns.len(), 1, "{warns:?}");
        let span = object_of(&warns[0], "span");
        assert!(span.contains("\"job\":\"disk_check\""), "{warns:?}");
        assert!(span.contains("\"iteration\":1"), "{warns:?}");
        assert!(
            !object_of(&warns[0], "fields").contains("\"job\""),
            "{warns:?}"
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_outcome_of_a_start_run_that_succeeds_is_ok() {
    bounded(async {
        let (result, spans) = spans_of_start_once(&[Behavior::Succeed]).await;
        assert!(result.is_ok());
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!(spans[0].job, "disk_check");
        assert_eq!(spans[0].iteration, 1);
        assert_eq!(spans[0].outcome.as_deref(), Some("ok"));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_start_run_that_fails_returns_the_error_and_records_its_code() {
    bounded(async {
        let (result, spans) = spans_of_start_once(&[Behavior::Fail]).await;
        assert!(matches!(
            result,
            Err(StartRunFault::Failed(AppError::Unauthenticated))
        ));
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!(spans[0].iteration, 1);
        assert_eq!(spans[0].outcome.as_deref(), Some("E-WEB-004"));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_start_run_that_fails_writes_no_line_of_its_own() {
    bounded(async {
        let logs = LogCapture::start();
        assert!(start_once(&[Behavior::Fail]).await.is_err());
        // The only line is the `WARN` line of the job. The supervisor writes none.
        let lines = logs.lines();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines_with(&lines, WARN_TEXT).len(), 1, "{lines:?}");
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_start_run_that_panics_returns_the_panic_and_records_it() {
    bounded(async {
        let (result, spans) = spans_of_start_once(&[Behavior::Panic]).await;
        assert!(matches!(result, Err(StartRunFault::Panicked)));
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!(spans[0].iteration, 1);
        assert_eq!(spans[0].outcome.as_deref(), Some("panic"));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_start_run_that_panics_writes_no_line_of_its_own() {
    bounded(async {
        let logs = LogCapture::start();
        assert!(matches!(
            start_once(&[Behavior::Panic]).await,
            Err(StartRunFault::Panicked)
        ));
        // The panic hook of the server is not installed in a unit test. The only line is the
        // `WARN` line that the job writes before it panics.
        let lines = logs.lines();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines_with(&lines, WARN_TEXT).len(), 1, "{lines:?}");
    })
    .await;
}

/// A job that counts its calls in shared state and writes the count in a `WARN` line.
struct Counting {
    calls: AtomicUsize,
}

impl JobRun for Counting {
    async fn run(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        tracing::warn!(call, "the counting job ran");
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn the_first_loop_run_after_the_start_run_has_iteration_2_and_the_same_job_state() {
    bounded(async {
        let logs = LogCapture::start();
        let (mut supervisor, stop) = start();
        let job = Arc::new(Counting {
            calls: AtomicUsize::new(0),
        });
        assert!(
            supervisor
                .run_at_start(JobName::DiskCheck, &job)
                .await
                .is_ok()
        );
        let schedule = Schedule::every(secs(60)).unwrap();
        supervisor.spawn_shared(JobName::DiskCheck, schedule, Arc::clone(&job));
        sleep(secs(70)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        let lines = lines_with(&logs.lines(), "the counting job ran");
        assert_eq!(lines.len(), 2, "{lines:?}");
        let expected = [
            ("\"call\":0", "\"iteration\":1"),
            ("\"call\":1", "\"iteration\":2"),
        ];
        for (line, (call, iteration)) in lines.iter().zip(expected) {
            assert!(object_of(line, "fields").contains(call), "{lines:?}");
            let span = object_of(line, "span");
            assert!(span.contains("\"job\":\"disk_check\""), "{lines:?}");
            assert!(span.contains(iteration), "{lines:?}");
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_start_run_gets_the_run_kind_scheduled() {
    bounded(async {
        let (mut supervisor, _stop) = start();
        let (job, mut seen) = probe(Instant::now(), secs(0), &[Behavior::Succeed]);
        assert!(
            supervisor
                .run_at_start(JobName::DiskCheck, &Arc::new(job))
                .await
                .is_ok()
        );
        assert_eq!(seen.kinds(), vec![RunKind::Scheduled]);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_request_id_of_the_start_run_is_not_nil_and_differs_from_the_first_loop_run() {
    bounded(async {
        let (mut supervisor, stop) = start();
        let (job, mut seen) = probe(Instant::now(), secs(0), &[Behavior::Succeed]);
        let job = Arc::new(job);
        assert!(
            supervisor
                .run_at_start(JobName::DiskCheck, &job)
                .await
                .is_ok()
        );
        supervisor.spawn_shared(
            JobName::DiskCheck,
            Schedule::every(secs(60)).unwrap(),
            Arc::clone(&job),
        );
        sleep(secs(70)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        let ids: Vec<RequestId> = seen
            .events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Started(_, request_id, _) => Some(request_id),
                _ => None,
            })
            .collect();
        assert_eq!(ids.len(), 2, "{ids:?}");
        assert!(!ids[0].as_uuid().is_nil(), "{ids:?}");
        assert!(!ids[1].as_uuid().is_nil(), "{ids:?}");
        assert_ne!(ids[0], ids[1]);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn two_start_runs_get_different_request_ids() {
    bounded(async {
        let (mut supervisor, _stop) = start();
        let (job, mut seen) = probe(Instant::now(), secs(0), &[Behavior::Succeed]);
        let job = Arc::new(job);
        for name in [JobName::DiskCheck, JobName::Heartbeat] {
            assert!(supervisor.run_at_start(name, &job).await.is_ok());
        }
        let ids: Vec<RequestId> = seen
            .events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Started(_, request_id, _) => Some(request_id),
                _ => None,
            })
            .collect();
        assert_eq!(ids.len(), 2, "{ids:?}");
        assert_ne!(ids[0], ids[1]);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_loop_does_not_repeat_the_start_run() {
    bounded(async {
        let (mut supervisor, stop) = start();
        let origin = Instant::now();
        let (job, mut seen) = probe(origin, secs(0), &[Behavior::Succeed]);
        let job = Arc::new(job);
        assert!(
            supervisor
                .run_at_start(JobName::DiskCheck, &job)
                .await
                .is_ok()
        );
        supervisor.spawn_shared(
            JobName::DiskCheck,
            Schedule::every(secs(60)).unwrap(),
            Arc::clone(&job),
        );
        sleep(secs(59)).await;
        // Only the start run has begun, 59 seconds after the spawn.
        assert_eq!(seen.starts(), vec![0]);
        sleep(secs(2)).await;
        assert_eq!(seen.starts(), vec![60]);
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_finished_start_run_leaves_no_open_count() {
    bounded(async {
        let (mut supervisor, _stop) = start();
        // The start run ends at once. The first loop run at 60 s hangs.
        let (job, mut seen) = probe(
            Instant::now(),
            secs(0),
            &[Behavior::Succeed, Behavior::Hang],
        );
        let job = Arc::new(job);
        assert!(
            supervisor
                .run_at_start(JobName::DiskCheck, &job)
                .await
                .is_ok()
        );
        supervisor.spawn_shared(
            JobName::DiskCheck,
            Schedule::every(secs(60)).unwrap(),
            Arc::clone(&job),
        );
        sleep(secs(70)).await;
        assert_eq!(seen.starts(), vec![0, 60]);
        // A count that the start run did not release would give 2 and not 1.
        let outcome = supervisor.drain(secs(0)).await;
        assert_eq!(outcome.cut(), 1);
    })
    .await;
}
