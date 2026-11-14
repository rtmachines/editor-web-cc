//! The detached tasks of the identity use cases (ADR-0172, `domain-model.md` section 5).

use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use parking_lot::Mutex;
use recipe_domain::audit::SecurityEvent;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot};
use tokio::task::JoinSet;
use tracing::{Instrument as _, Span};

use crate::error::{AppError, TaskStage, TimeoutStage};
use crate::ports::SecurityEventSink;

/// The error for a detached task that ended without a result (`E-WEB-011`).
const fn detached_failed() -> AppError {
    AppError::TaskFailed {
        stage: TaskStage::Detached,
    }
}

/// The result of a detached task, as far as the task itself must know it.
///
/// The trait solves one problem: the task logs an error that no future receives (ADR-0173 (m)),
/// and the task type is generic. A result of `Result<_, AppError>` gives its error. A task that
/// returns nothing gives none.
pub trait TaskOutput: Send + 'static {
    /// Return the error of the result, if it holds one.
    fn failure(&self) -> Option<&AppError>;
}

impl TaskOutput for () {
    fn failure(&self) -> Option<&AppError> {
        None
    }
}

impl<T: Send + 'static> TaskOutput for Result<T, AppError> {
    fn failure(&self) -> Option<&AppError> {
        self.as_ref().err()
    }
}

/// Write the one `WARN` line of an error that no future receives.
///
/// The detached code is the one log place of that error (ADR-0173 (o)). The task calls this
/// function when its send fails. The drop of an unpolled [`DetachedResult`] calls it for a value
/// that the task sent and no future took. The line holds the `error_code` and the `chain`. The
/// `request_id` comes from the span of the task. An `Ok` result writes no line.
fn log_unreceived(output: &impl TaskOutput) {
    if let Some(error) = output.failure() {
        tracing::warn!(
            error_code = %error.code(),
            chain = %display_chain(error),
            "error of a detached task that no future received"
        );
    }
}

/// The slots and the task set of the detached tasks.
///
/// The type solves one problem: a request future that a timeout or a closed connection drops must
/// not lose the security event of a committed step (F-07). The span from the first transaction to
/// the write of the event runs in a task that no request can cancel. A semaphore with
/// `identity.detached_tasks` permits bounds the open tasks. One `JoinSet` behind a mutex owns them.
/// A clone shares the slots and the set.
#[derive(Clone)]
pub struct DetachedTasks {
    shared: Arc<Shared>,
}

struct Shared {
    slots: Arc<Semaphore>,
    acquire_timeout: Duration,
    tasks: Mutex<TaskSet>,
}

/// The `JoinSet` and the closed mark. One mutex guards both, so a spawn and the drain never
/// interleave between the take of the set and the mark.
struct TaskSet {
    open: JoinSet<()>,
    is_closed: bool,
}

/// The error for a slot or a spawn that comes after the drain (`E-WEB-009`, ADR-0172 (ah)).
const fn detached_closed() -> AppError {
    AppError::Timeout {
        stage: TimeoutStage::DetachedSlot,
    }
}

impl DetachedTasks {
    /// Build the set with `slots` slots and the slot wait limit `acquire_timeout`.
    #[must_use]
    pub fn new(slots: usize, acquire_timeout: Duration) -> Self {
        Self {
            shared: Arc::new(Shared {
                slots: Arc::new(Semaphore::new(slots)),
                acquire_timeout,
                tasks: Mutex::new(TaskSet {
                    open: JoinSet::new(),
                    is_closed: false,
                }),
            }),
        }
    }

    /// Wait for a free slot, at most `argon2.acquire_timeout`.
    ///
    /// The wait is cancellable. A caller takes the slot before its first transaction, so a late
    /// slot commits nothing (ADR-0172 (w)).
    ///
    /// # Errors
    ///
    /// Returns `AppError::Timeout` with the stage `DetachedSlot` (`E-WEB-009`) when the limit
    /// elapses. Returns the same error at once after `drain`, because the drain closes the
    /// semaphore (ADR-0172 (ah)).
    pub async fn acquire(&self) -> Result<DetachedSlot, AppError> {
        let waiting = Arc::clone(&self.shared.slots).acquire_owned();
        let permit = tokio::time::timeout(self.shared.acquire_timeout, waiting)
            .await
            .map_err(|_elapsed| AppError::Timeout {
                stage: TimeoutStage::DetachedSlot,
            })?
            .map_err(|_closed| detached_closed())?;
        Ok(DetachedSlot { _permit: permit })
    }

    /// Run `work` in a detached task that owns `slot`, and return the handle of its result.
    ///
    /// The call first reaps every finished task, so the set holds the open tasks and the tasks
    /// that ended since the last spawn (ADR-0172 (y), (z)). A reaped panic writes no line and only
    /// frees its slot (ADR-0172 (ab)). The task runs inside the span of the caller, so its lines
    /// keep the `request_id` (ADR-0172 (t)). A dropped handle does not stop the task.
    ///
    /// When no future takes the result and the result is an `Err`, the detached code writes the one
    /// `WARN` line of that error. The task writes it when its send fails. The drop of the handle
    /// writes it when the task sent the `Err` and the handle dropped before a poll took it, inside
    /// the span of the caller. A result with `Ok` writes no line (ADR-0173 (m) to (r)).
    ///
    /// A spawn after `drain` does not start `work`. The handle resolves at once to
    /// `AppError::Timeout` with the stage `DetachedSlot` (`E-WEB-009`), as a missing slot does.
    /// The caller then commits nothing (ADR-0172 (ah)).
    pub fn spawn<T, F>(&self, slot: DetachedSlot, work: F) -> DetachedResult<T>
    where
        T: TaskOutput,
        F: Future<Output = T> + Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let task = async move {
            let _slot = slot;
            let output = work.await;
            // A closed receiver means that no future waits for the result (ADR-0173 (m)).
            if let Err(unreceived) = sender.send(output) {
                log_unreceived(&unreceived);
            }
        }
        .instrument(Span::current());
        let is_refused = !self.launch(task);
        DetachedResult {
            receiver,
            is_refused,
            span: Span::current(),
        }
    }

    /// Run `work` in a detached task that owns `slot` and keeps it until T2 decides.
    ///
    /// The method solves one problem: a use case that passes its judge still has a step in the
    /// request future (T2), and that step can refuse with a security event. The task sends the
    /// passed value together with a [`RefusalSender`], then waits on it and holds its slot. A
    /// refusal through the sender makes the task write the event with [`record_security_event`],
    /// so a drop of the request future after the send cannot lose it. A drop of the sender ends
    /// the task with no write (ADR-0173 (c) to (f), (h), (j)).
    ///
    /// An `Err` of `work` goes to the handle at once and the task ends. A closed receiver and an
    /// `Err` give the one `WARN` line of [`spawn`](Self::spawn), also when the handle drops after
    /// the send. A closed receiver and an `Ok` write no line. The rules of `spawn` for the span,
    /// the reaper, and the drain hold here too.
    pub(crate) fn spawn_judged<T, F>(
        &self,
        slot: DetachedSlot,
        sink: Arc<dyn SecurityEventSink>,
        work: F,
    ) -> DetachedResult<Result<(T, RefusalSender), AppError>>
    where
        T: Send + 'static,
        F: Future<Output = Result<T, AppError>> + Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let task = async move {
            let _slot = slot;
            let passed = match work.await {
                Ok(passed) => passed,
                Err(error) => {
                    if let Err(unreceived) = sender.send(Err(error)) {
                        log_unreceived(&unreceived);
                    }
                    return;
                }
            };
            let (refusal, verdict) = oneshot::channel();
            if sender
                .send(Ok((passed, RefusalSender { sender: refusal })))
                .is_err()
            {
                return;
            }
            if let Ok(Refusal { event, written }) = verdict.await {
                record_security_event(sink.as_ref(), event).await;
                let _reported = written.send(());
            }
        }
        .instrument(Span::current());
        let is_refused = !self.launch(task);
        DetachedResult {
            receiver,
            is_refused,
            span: Span::current(),
        }
    }

    /// Reap the finished tasks and start `task`. Return `false` after the drain, with no start.
    fn launch(&self, task: impl Future<Output = ()> + Send + 'static) -> bool {
        let mut tasks = self.shared.tasks.lock();
        if tasks.is_closed {
            return false;
        }
        while tasks.open.try_join_next().is_some() {}
        tasks.open.spawn(task);
        true
    }

    /// Wait at most `deadline` for the open tasks, then abort the rest. Return the aborted count.
    ///
    /// The stop of the server calls this in step 3 and step 4 of `reliability.md` section 7.3
    /// (ADR-0172 (af) to (ah)). The call returns after every aborted task has dropped its future,
    /// so the transactions of those tasks have rolled back. The call also closes the set and the
    /// slots in the lock section of the take. A later `spawn` or `acquire` fails at once, so no
    /// task starts that no drain sees (ADR-0172 (ah)). A second call returns 0.
    /// It reaps the tasks that ended before it counts, as the public listener does.
    pub async fn drain(&self, deadline: Duration) -> usize {
        let mut open = {
            let mut tasks = self.shared.tasks.lock();
            tasks.is_closed = true;
            self.shared.slots.close();
            std::mem::take(&mut tasks.open)
        };
        let ended = tokio::time::timeout(deadline, async {
            while open.join_next().await.is_some() {}
        })
        .await;
        if ended.is_ok() {
            return 0;
        }
        while open.try_join_next().is_some() {}
        let aborted = open.len();
        open.abort_all();
        while open.join_next().await.is_some() {}
        aborted
    }

    /// Return the number of tasks that the set holds, finished or not.
    #[cfg(test)]
    fn tracked(&self) -> usize {
        self.shared.tasks.lock().open.len()
    }

    /// Return the number of slots that no task holds.
    #[cfg(test)]
    fn free_slots(&self) -> usize {
        self.shared.slots.available_permits()
    }
}

/// One slot of `identity.detached_tasks`.
///
/// The type solves one problem: the slot must live as long as its task. The task owns it. The slot
/// returns when the task ends, also after an error and during the unwind of a panic
/// (ADR-0172 (x)).
#[derive(Debug)]
#[must_use = "the slot bounds the detached tasks only while the caller or its task holds it"]
pub struct DetachedSlot {
    _permit: OwnedSemaphorePermit,
}

/// The result of one detached task.
///
/// The type solves one problem: a use case must wait for the judge result of its task and map a
/// panic to an error. A closed sender gives `AppError::TaskFailed` with the stage `Detached`
/// (`E-WEB-011`, ADR-0172 (aa)). A spawn after the drain gives `AppError::Timeout` with the stage
/// `DetachedSlot` (`E-WEB-009`).
///
/// A drop of the handle also drops a value that the task sent and no poll took. The drop writes
/// the one `WARN` line of an `Err` value inside the span of the spawn (ADR-0173 (m)). A value that
/// a poll took, or that the task failed to send, writes no line here.
#[derive(Debug)]
#[must_use = "a dropped handle discards the result, and the task still runs"]
pub struct DetachedResult<T: TaskOutput> {
    receiver: oneshot::Receiver<T>,
    is_refused: bool,
    span: Span,
}

impl<T: TaskOutput> Drop for DetachedResult<T> {
    fn drop(&mut self) {
        if let Ok(unreceived) = self.receiver.try_recv() {
            let _entered = self.span.enter();
            log_unreceived(&unreceived);
        }
    }
}

impl<T: TaskOutput> Future for DetachedResult<T> {
    type Output = Result<T, AppError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_refused {
            return Poll::Ready(Err(detached_closed()));
        }
        Pin::new(&mut self.receiver)
            .poll(cx)
            .map_err(|_closed| detached_failed())
    }
}

/// A refusal event of T2 and the signal for the end of its write.
#[derive(Debug)]
struct Refusal {
    event: SecurityEvent,
    written: oneshot::Sender<()>,
}

/// The sender of a refusal event, in the hand of the request future.
///
/// The type solves one problem: T2 stays in the request future, but the write of its refusal event
/// must survive a drop of that future (ADR-0173). The task of [`DetachedTasks::spawn_judged`]
/// waits on this sender. The request future calls [`RefusalSender::refuse`] on a refusal, and
/// drops the sender on a grant.
#[derive(Debug)]
#[must_use = "a dropped sender tells the task that T2 granted"]
pub(crate) struct RefusalSender {
    sender: oneshot::Sender<Refusal>,
}

impl RefusalSender {
    /// Send `event` to the task and wait until the task has written it.
    ///
    /// A drop of this future after the send does not stop the write. The task writes the event
    /// once and writes the one `WARN` line when the write fails (ADR-0173 (e), (g), (h)). When
    /// the task is gone, the call returns at once and the caller still gives its refusal
    /// (ADR-0173 (i)).
    pub(crate) async fn refuse(self, event: SecurityEvent) {
        let (written, ended) = oneshot::channel();
        if self.sender.send(Refusal { event, written }).is_ok() {
            // An `Err` means that the task ended before it reported the write.
            let _ended = ended.await;
        }
    }
}

/// Write one security event. A failed write gives the one `WARN` line of a lost event.
///
/// The function solves one problem: the four use cases of ADR-0172 (g) share one log place for a
/// lost event. The detached task calls it before it sends its result, so a cancel at any point of
/// the request gives exactly one line (ADR-0172 (n)). The function returns no storage fault
/// (ADR-0172 (o)).
///
/// The field `chain` holds the `Display` chain of the fault, as the line of `log_error` does.
pub async fn record_security_event(sink: &dyn SecurityEventSink, event: SecurityEvent) {
    let request_id = event.request_id();
    let kind = event.kind();
    if let Err(lost) = sink.record(event).await {
        tracing::warn!(
            error_code = %lost.code(),
            request_id = %request_id,
            chain = %display_chain(&lost),
            event = kind.as_sql(),
            "security event lost after a committed step"
        );
    }
}

/// Return the `Display` text of an error and of each of its sources, joined by `: `.
///
/// The text never comes from `Debug`, because the `Debug` of a database error can hold a row value.
fn display_chain(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        source = inner.source();
    }
    text
}

#[cfg(test)]
mod tests;
