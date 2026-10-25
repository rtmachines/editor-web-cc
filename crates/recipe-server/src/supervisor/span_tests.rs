//! Tests of the span `job` around each run (ADR-0088 (a), `reliability.md` section 3.2).
//!
//! Every test uses paused time. The log tests read the objects `span` and `fields` of the JSON
//! lines. The outcome test reads the span itself through a layer, because the span is closed
//! before any line can show a recorded value.

use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use recipe_app::error::AppError;
use recipe_app::jobs::RunKind;
use recipe_app::ports::Clock;
use recipe_domain::audit::RequestId;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::Subscriber;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::{LookupSpan, Registry};

use super::support::{
    Behavior, LogCapture, PausedClock, bounded, object_of, secs, set_test_default, settings,
};
use super::{JobName, JobRun, Supervisor};
use crate::schedule::Schedule;

/// The text of the `WARN` line that the fake job writes. The fake never writes the field `job`.
pub(super) const WARN_TEXT: &str = "the fake job saw a handled fault";

/// A job that writes one `WARN` line, then follows its script. Later calls use the last item.
pub(super) struct Noisy {
    script: Vec<Behavior>,
    calls: AtomicUsize,
}

impl Noisy {
    pub(super) fn new(script: &[Behavior]) -> Self {
        Self {
            script: script.to_vec(),
            calls: AtomicUsize::new(0),
        }
    }
}

impl JobRun for Noisy {
    async fn run(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let behavior = self
            .script
            .get(call)
            .or(self.script.last())
            .copied()
            .unwrap_or(Behavior::Succeed);
        tracing::warn!("{WARN_TEXT}");
        match behavior {
            Behavior::Fail => Err(AppError::Unauthenticated),
            Behavior::Panic => {
                let number: u8 = "not a number".parse().unwrap();
                assert_eq!(number, 0);
                Ok(())
            }
            Behavior::Succeed | Behavior::Hang => Ok(()),
        }
    }
}

/// Run `Noisy` as the job `name` with an interval of 60 seconds for `seconds` of virtual time.
async fn run_noisy(name: JobName, script: &[Behavior], seconds: u64) {
    let clock: Arc<dyn Clock> = PausedClock::at("2026-10-24T00:00:00.000000Z");
    let stop = CancellationToken::new();
    let mut supervisor = Supervisor::new(settings(), clock, stop.clone());
    let schedule = Schedule::every(secs(60)).unwrap();
    supervisor.spawn(name, schedule, Noisy::new(script));
    sleep(secs(seconds)).await;
    stop.cancel();
    assert!(supervisor.drain(secs(5)).await.is_complete());
}

/// The lines of `lines` that hold `needle`.
pub(super) fn lines_with(lines: &[String], needle: &str) -> Vec<String> {
    lines
        .iter()
        .filter(|line| line.contains(needle))
        .cloned()
        .collect()
}

#[tokio::test(start_paused = true)]
async fn the_error_line_of_a_failed_run_gets_job_and_iteration_from_the_span_only() {
    bounded(async {
        let logs = LogCapture::start();
        run_noisy(JobName::Cleanup, &[Behavior::Fail, Behavior::Succeed], 130).await;
        let lines = logs.lines();
        // The run writes its `WARN` line before it fails. That line is inside the same span.
        let first_warn = lines_with(&lines, WARN_TEXT);
        let warn_span = first_warn
            .first()
            .map_or("", |line| object_of(line, "span"));
        assert!(warn_span.contains("\"job\":\"cleanup\""), "{first_warn:?}");
        assert!(warn_span.contains("\"iteration\":1"), "{first_warn:?}");
        let errors = lines_with(&lines, "\"level\":\"ERROR\"");
        assert_eq!(errors.len(), 1, "{errors:?}");
        let span = object_of(&errors[0], "span");
        assert!(span.contains("\"job\":\"cleanup\""), "{errors:?}");
        assert!(span.contains("\"iteration\":1"), "{errors:?}");
        // The span gives the field once. The event does not repeat it.
        let fields = object_of(&errors[0], "fields");
        assert!(
            fields.contains("\"error_code\":\"E-WEB-004\""),
            "{errors:?}"
        );
        assert!(!fields.contains("\"job\""), "{errors:?}");
        assert!(!errors[0].contains("request_id"), "{errors:?}");
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_warn_event_of_the_job_gets_job_and_iteration_from_the_span() {
    bounded(async {
        let logs = LogCapture::start();
        run_noisy(JobName::Heartbeat, &[Behavior::Succeed], 130).await;
        let warns = lines_with(&logs.lines(), WARN_TEXT);
        assert_eq!(warns.len(), 2, "{warns:?}");
        for line in &warns {
            let span = object_of(line, "span");
            assert!(span.contains("\"job\":\"heartbeat\""), "{line}");
            assert!(span.contains("\"iteration\""), "{line}");
            assert!(!object_of(line, "fields").contains("\"job\""), "{line}");
        }
    })
    .await;
}

// The panic hook of `panic_hook.rs` is a process-wide hook, so a unit test does not install it.
// The hook writes with `tracing::error!`, which reads the span of the panicking thread, as the
// `WARN` line of the job does. The test therefore takes the `WARN` line that the job writes
// before its panic and the own line of the supervisor.
#[tokio::test(start_paused = true)]
async fn the_lines_around_a_panic_get_the_job_from_the_span() {
    bounded(async {
        let logs = LogCapture::start();
        run_noisy(JobName::Cleanup, &[Behavior::Panic, Behavior::Succeed], 65).await;
        let lines = logs.lines();
        let warns = lines_with(&lines, WARN_TEXT);
        let panics = lines_with(&lines, "panicked");
        assert_eq!(warns.len(), 1, "{lines:?}");
        assert_eq!(panics.len(), 1, "{lines:?}");
        for line in warns.iter().chain(&panics) {
            let span = object_of(line, "span");
            assert!(span.contains("\"job\":\"cleanup\""), "{line}");
            assert!(span.contains("\"iteration\":1"), "{line}");
            assert!(!object_of(line, "fields").contains("\"job\""), "{line}");
            assert!(!line.contains("request_id"), "{line}");
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_iteration_counts_the_runs_and_a_restart_gets_the_next_number() {
    bounded(async {
        let logs = LogCapture::start();
        // The panic at 60 gives the restart at 70. The interval runs follow at 130 and 190.
        run_noisy(JobName::Cleanup, &[Behavior::Panic, Behavior::Succeed], 200).await;
        let spans: Vec<String> = lines_with(&logs.lines(), WARN_TEXT)
            .iter()
            .map(|line| object_of(line, "span").to_owned())
            .collect();
        assert_eq!(spans.len(), 4, "{spans:?}");
        for (index, span) in spans.iter().enumerate() {
            let expected = format!("\"iteration\":{}", index + 1);
            assert!(span.contains(&expected), "{spans:?}");
        }
    })
    .await;
}

/// Run `Noisy` as the job `Cleanup` under the log filter `directives`. Return the lines that
/// hold `needle`. The job `Alerts` is not used, because its failed run writes a `WARN` line.
async fn first_run_lines(directives: &str, behavior: Behavior, needle: &str) -> Vec<String> {
    let logs = LogCapture::start_with(directives);
    run_noisy(JobName::Cleanup, &[behavior, Behavior::Succeed], 65).await;
    lines_with(&logs.lines(), needle)
}

/// Assert that every line of `lines` holds the span fields `job` and `iteration`.
fn assert_lines_in_span(lines: &[String], expected: usize) {
    assert_eq!(lines.len(), expected, "{lines:?}");
    for line in lines {
        let span = object_of(line, "span");
        assert!(span.contains("\"job\":\"cleanup\""), "{line}");
        assert!(span.contains("\"iteration\":1"), "{line}");
        assert!(!object_of(line, "fields").contains("\"job\""), "{line}");
    }
}

// An operator may set `RUST_LOG=warn` or `error` (`reliability.md` section 7.3). The span must stay
// on, or the lines of a run lose the field `job`.
#[tokio::test(start_paused = true)]
async fn the_lines_of_a_failed_run_keep_the_span_under_the_filter_warn() {
    bounded(async {
        let warns = first_run_lines("warn", Behavior::Fail, WARN_TEXT).await;
        assert_lines_in_span(&warns, 1);
        let errors = first_run_lines("warn", Behavior::Fail, "\"level\":\"ERROR\"").await;
        assert_lines_in_span(&errors, 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_error_line_of_a_failed_run_keeps_the_span_under_the_filter_error() {
    bounded(async {
        let errors = first_run_lines("error", Behavior::Fail, "\"level\":\"ERROR\"").await;
        assert_lines_in_span(&errors, 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_error_line_of_a_panic_keeps_the_span_under_the_filter_error() {
    bounded(async {
        let panics = first_run_lines("error", Behavior::Panic, "panicked").await;
        assert_lines_in_span(&panics, 1);
    })
    .await;
}

/// One closed span `job` as the layer saw it.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Seen {
    pub(super) job: String,
    pub(super) iteration: u64,
    pub(super) outcome: Option<String>,
}

impl Visit for Seen {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "job" => value.clone_into(&mut self.job),
            "outcome" => self.outcome = Some(value.to_owned()),
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "iteration" {
            self.iteration = value;
        }
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn Debug) {}
}

/// A layer that sends each closed span `job` with its recorded fields.
pub(super) struct SpanRecorder(pub(super) Sender<Seen>);

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for SpanRecorder {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if span.name() != "job" {
            return;
        }
        let mut seen = Seen::default();
        attrs.record(&mut seen);
        span.extensions_mut().insert(seen);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if let Some(seen) = span.extensions_mut().get_mut::<Seen>() {
            values.record(seen);
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else { return };
        if let Some(seen) = span.extensions_mut().remove::<Seen>() {
            // The receiver is gone at the end of a test. Then nobody needs the span.
            let _ = self.0.send(seen);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn the_outcome_is_ok_the_error_code_or_panic() {
    bounded(async {
        let (sender, receiver): (_, Receiver<Seen>) = channel();
        let _guard = set_test_default(Registry::default().with(SpanRecorder(sender)));
        // The runs start at 60, 120, and 180. The panic at 180 gives the restart at 190.
        let script = [
            Behavior::Succeed,
            Behavior::Fail,
            Behavior::Panic,
            Behavior::Succeed,
        ];
        run_noisy(JobName::Alerts, &script, 195).await;
        let seen: Vec<(String, u64, Option<String>)> = receiver
            .try_iter()
            .map(|span| (span.job, span.iteration, span.outcome))
            .collect();
        let expected = [
            ("alerts".to_owned(), 1, Some("ok".to_owned())),
            ("alerts".to_owned(), 2, Some("E-WEB-004".to_owned())),
            ("alerts".to_owned(), 3, Some("panic".to_owned())),
            ("alerts".to_owned(), 4, Some("ok".to_owned())),
        ];
        assert_eq!(seen, expected);
    })
    .await;
}
