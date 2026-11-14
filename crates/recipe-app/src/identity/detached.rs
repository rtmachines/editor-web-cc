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
    tasks: Mutex<JoinSet<()>>,
}

impl DetachedTasks {
    /// Build the set with `slots` slots and the slot wait limit `acquire_timeout`.
    #[must_use]
    pub fn new(slots: usize, acquire_timeout: Duration) -> Self {
        Self {
            shared: Arc::new(Shared {
                slots: Arc::new(Semaphore::new(slots)),
                acquire_timeout,
                tasks: Mutex::new(JoinSet::new()),
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
    /// elapses. Returns `AppError::TaskFailed` when the semaphore is closed, which never happens.
    pub async fn acquire(&self) -> Result<DetachedSlot, AppError> {
        let waiting = Arc::clone(&self.shared.slots).acquire_owned();
        let permit = tokio::time::timeout(self.shared.acquire_timeout, waiting)
            .await
            .map_err(|_elapsed| AppError::Timeout {
                stage: TimeoutStage::DetachedSlot,
            })?
            .map_err(|_closed| detached_failed())?;
        Ok(DetachedSlot { _permit: permit })
    }

    /// Run `work` in a detached task that owns `slot`, and return the handle of its result.
    ///
    /// The call first reaps every finished task, so the set holds the open tasks and the tasks
    /// that ended since the last spawn (ADR-0172 (y), (z)). A reaped panic writes no line and only
    /// frees its slot (ADR-0172 (ab)). The task runs inside the span of the caller, so its lines
    /// keep the `request_id` (ADR-0172 (t)). A dropped handle does not stop the task.
    pub fn spawn<T, F>(&self, slot: DetachedSlot, work: F) -> DetachedResult<T>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let task = async move {
            let _slot = slot;
            let output = work.await;
            // A closed receiver means that no future waits for the result.
            let _closed = sender.send(output);
        }
        .instrument(Span::current());
        let mut tasks = self.shared.tasks.lock();
        while tasks.try_join_next().is_some() {}
        tasks.spawn(task);
        DetachedResult { receiver }
    }

    /// Wait at most `deadline` for the open tasks, then abort the rest. Return the aborted count.
    ///
    /// The stop of the server calls this in step 3 and step 4 of `reliability.md` section 7.3
    /// (ADR-0172 (af) to (ah)). The call returns after every aborted task has dropped its future,
    /// so the transactions of those tasks have rolled back. A task that a later spawn starts
    /// belongs to the next call.
    pub async fn drain(&self, deadline: Duration) -> usize {
        let mut open = std::mem::take(&mut *self.shared.tasks.lock());
        let ended = tokio::time::timeout(deadline, async {
            while open.join_next().await.is_some() {}
        })
        .await;
        if ended.is_ok() {
            return 0;
        }
        let aborted = open.len();
        open.abort_all();
        while open.join_next().await.is_some() {}
        aborted
    }

    /// Return the number of tasks that the set holds, finished or not.
    #[cfg(test)]
    fn tracked(&self) -> usize {
        self.shared.tasks.lock().len()
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
/// (`E-WEB-011`, ADR-0172 (aa)).
#[derive(Debug)]
#[must_use = "a dropped handle discards the result, and the task still runs"]
pub struct DetachedResult<T> {
    receiver: oneshot::Receiver<T>,
}

impl<T> Future for DetachedResult<T> {
    type Output = Result<T, AppError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.receiver)
            .poll(cx)
            .map_err(|_closed| detached_failed())
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
