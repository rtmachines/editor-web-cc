//! Tests of the detached tasks: the slot, the reaper, the drain, and the lost event.

use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use parking_lot::Mutex;
use recipe_domain::audit::SecurityEvent;
use recipe_domain::identity::UserId;
use tokio::sync::Notify;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use uuid::Uuid;

use super::{DetachedTasks, record_security_event};
use crate::error::{AppError, TaskStage, TimeoutStage};
use crate::identity::new_request_id;
use crate::memory::{Fault, MemorySink};
use crate::ports::SecurityEventSink;

const SLOT_WAIT: Duration = Duration::from_secs(2);
const DRAIN_WAIT: Duration = Duration::from_secs(5);

fn tasks(slots: usize) -> DetachedTasks {
    DetachedTasks::new(slots, SLOT_WAIT)
}

/// The payload of a panic that writes no message on stderr.
#[derive(Clone, Copy, Debug)]
struct SilentPanic;

fn unlock_event() -> SecurityEvent {
    SecurityEvent::unlock(
        UserId::from(Uuid::from_u128(1)),
        UserId::from(Uuid::from_u128(2)),
        new_request_id().unwrap(),
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        None,
    )
}

fn failing_sink() -> Arc<dyn SecurityEventSink> {
    let sink = MemorySink::default();
    sink.fail_with(Fault::Deadlock);
    Arc::new(sink)
}

#[tokio::test(start_paused = true)]
async fn detached_body_ends_after_the_caller_drops_the_future() {
    let tasks = tasks(2);
    let release = Arc::new(Notify::new());
    let finished = Arc::new(AtomicBool::new(false));
    let slot = tasks.acquire().await.unwrap();

    let body = {
        let (release, finished) = (Arc::clone(&release), Arc::clone(&finished));
        async move {
            release.notified().await;
            finished.store(true, Ordering::SeqCst);
        }
    };
    let caller = async { tasks.spawn(slot, body).await };
    let cancelled = tokio::time::timeout(Duration::from_millis(10), caller).await;
    assert!(cancelled.is_err(), "the caller must time out first");
    assert!(!finished.load(Ordering::SeqCst));

    release.notify_one();
    assert_eq!(tasks.drain(DRAIN_WAIT).await, 0);
    assert!(
        finished.load(Ordering::SeqCst),
        "the body must run to its end"
    );
}

#[tokio::test(start_paused = true)]
async fn detached_spawn_gives_timeout_when_every_slot_is_taken() {
    let tasks = tasks(1);
    let _held = tasks.acquire().await.unwrap();

    let refused = tasks.acquire().await;

    assert!(
        matches!(
            refused,
            Err(AppError::Timeout {
                stage: TimeoutStage::DetachedSlot
            })
        ),
        "{refused:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn detached_slots_return_after_many_batches() {
    const SLOTS: usize = 3;
    let tasks = tasks(SLOTS);
    for _batch in 0..50 {
        let mut results = Vec::new();
        for _task in 0..SLOTS {
            let slot = tasks.acquire().await.unwrap();
            results.push(tasks.spawn(slot, async { 7_u8 }));
            assert!(tasks.tracked() <= SLOTS, "{}", tasks.tracked());
        }
        for result in results {
            assert_eq!(result.await.unwrap(), 7);
        }
    }
    tokio::task::yield_now().await;
    let slot = tasks.acquire().await;
    assert!(slot.is_ok());
}

#[tokio::test(start_paused = true)]
async fn detached_slot_returns_after_an_error() {
    let tasks = tasks(1);
    let slot = tasks.acquire().await.unwrap();
    let failed = tasks
        .spawn(slot, async {
            Err::<(), AppError>(AppError::Timeout {
                stage: TimeoutStage::Request,
            })
        })
        .await
        .unwrap();
    assert!(failed.is_err());

    let again = tasks.acquire().await;

    assert!(again.is_ok(), "the slot must come back after an error");
}

#[tokio::test(start_paused = true)]
async fn detached_reaper_frees_the_slot_of_a_panic_and_writes_no_line() {
    let (capture, _guard) = LogCapture::install();
    let tasks = tasks(1);
    let slot = tasks.acquire().await.unwrap();

    let panicked: Result<(), AppError> = tasks
        .spawn(slot, async {
            std::panic::resume_unwind(Box::new(SilentPanic))
        })
        .await;
    assert!(
        matches!(
            panicked,
            Err(AppError::TaskFailed {
                stage: TaskStage::Detached
            })
        ),
        "{panicked:?}"
    );

    let slot = tasks.acquire().await.unwrap();
    let after: Result<u8, AppError> = tasks.spawn(slot, async { 1_u8 }).await;
    assert_eq!(after.unwrap(), 1);
    assert_eq!(tasks.tracked(), 1, "the spawn reaps the panicked task");
    assert_eq!(capture.count(), 0, "a reaped panic writes no line");
}

#[tokio::test(start_paused = true)]
async fn lost_event_gives_one_warn_line_for_a_cancel_before_the_send_after_the_send_and_after_the_receive()
 {
    for cancel in [Cancel::BeforeSend, Cancel::AfterSend, Cancel::AfterReceive] {
        let (capture, _guard) = LogCapture::install();
        let tasks = tasks(1);
        let sink = failing_sink();
        let slot = tasks.acquire().await.unwrap();
        let handle = tasks.spawn(slot, async move {
            record_security_event(sink.as_ref(), unlock_event()).await;
        });

        match cancel {
            Cancel::BeforeSend => {
                drop(handle);
                assert_eq!(tasks.drain(DRAIN_WAIT).await, 0);
            }
            Cancel::AfterSend => {
                assert_eq!(tasks.drain(DRAIN_WAIT).await, 0);
                drop(handle);
            }
            Cancel::AfterReceive => {
                handle.await.unwrap();
            }
        }

        let lines = capture.events();
        assert_eq!(lines.len(), 1, "{cancel:?}: {lines:?}");
        let line = lines.first().unwrap();
        assert_eq!(line.level, Level::WARN);
        assert_eq!(line.field("event"), Some("\"unlock\""));
        assert!(line.field("error_code").is_some(), "{line:?}");
        assert!(line.field("request_id").is_some(), "{line:?}");
        assert!(line.field("chain").is_some(), "{line:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_recorded_event_writes_no_line() {
    let (capture, _guard) = LogCapture::install();
    let sink = MemorySink::default();

    record_security_event(&sink, unlock_event()).await;

    assert_eq!(sink.events().len(), 1);
    assert_eq!(capture.count(), 0);
}

#[tokio::test(start_paused = true)]
async fn detached_drain_returns_zero_when_the_tasks_end_in_time() {
    let tasks = tasks(2);
    let slot = tasks.acquire().await.unwrap();
    let _result = tasks.spawn(slot, async {
        tokio::time::sleep(Duration::from_secs(1)).await;
    });

    assert_eq!(tasks.drain(DRAIN_WAIT).await, 0);
    assert!(tasks.acquire().await.is_ok());
}

/// A guard that sets a flag when its task future drops.
struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tokio::test(start_paused = true)]
async fn detached_drain_aborts_the_rest_after_the_deadline_and_returns_their_count() {
    let tasks = tasks(2);
    let dropped = Arc::new(AtomicBool::new(false));
    let slot = tasks.acquire().await.unwrap();
    let _result = tasks.spawn(slot, {
        let flag = DropFlag(Arc::clone(&dropped));
        async move {
            let _flag = flag;
            std::future::pending::<()>().await;
        }
    });

    let aborted = tasks.drain(Duration::from_secs(1)).await;

    assert_eq!(aborted, 1);
    assert!(
        dropped.load(Ordering::SeqCst),
        "the abort must drop the body"
    );
    assert_eq!(tasks.tracked(), 0);
    assert!(
        tasks.acquire().await.is_ok(),
        "the abort must free the slot"
    );
}

#[derive(Clone, Copy, Debug)]
enum Cancel {
    BeforeSend,
    AfterSend,
    AfterReceive,
}

// ---- log capture ----

/// One event that the code wrote.
#[derive(Clone, Debug)]
struct Logged {
    level: Level,
    fields: Vec<(String, String)>,
}

impl Logged {
    fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// A subscriber that keeps every event of its thread.
#[derive(Clone, Default)]
struct LogCapture {
    events: Arc<Mutex<Vec<Logged>>>,
}

impl LogCapture {
    fn install() -> (Self, DefaultGuard) {
        LazyLock::force(&KEEPALIVE);
        let capture = Self::default();
        let guard = tracing::subscriber::set_default(capture.clone());
        (capture, guard)
    }

    fn events(&self) -> Vec<Logged> {
        self.events.lock().clone()
    }

    fn count(&self) -> usize {
        self.events.lock().len()
    }
}

#[derive(Default)]
struct FieldCopy {
    fields: Vec<(String, String)>,
}

impl Visit for FieldCopy {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.fields
            .push((field.name().to_owned(), format!("{value:?}")));
    }
}

impl Subscriber for LogCapture {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut copy = FieldCopy::default();
        event.record(&mut copy);
        self.events.lock().push(Logged {
            level: *event.metadata().level(),
            fields: copy.fields,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
