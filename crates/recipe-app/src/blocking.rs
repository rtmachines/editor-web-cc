//! Blocking work and settings (design D8, `reliability.md` sections 2.2, 4.4, and 7.4).

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use recipe_domain::recipe::ApprovalPolicy;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;

use crate::error::{AppError, StorageError, TaskStage, TimeoutStage};

/// Default of `timeout.validation` in seconds (`reliability.md` section 7.11).
const DEFAULT_VALIDATION_SECONDS: u64 = 10;
/// Default of `timeout.export` in seconds (`reliability.md` section 7.11).
const DEFAULT_EXPORT_SECONDS: u64 = 10;

/// How `Blocking::validate` treats its work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Run the work in `spawn_blocking`.
    Run,
    /// Test double: report an elapsed limit and skip the work.
    #[cfg(any(test, feature = "memory"))]
    Elapsed,
    /// Test double: report a panicked task and skip the work.
    #[cfg(any(test, feature = "memory"))]
    Panicked,
}

/// Runner of CPU work and of time-limited use cases.
///
/// The type solves one problem: CPU work such as the snapshot check must not block the async
/// workers, and a slow stage must end with a known error instead of a hang. `validate` runs a
/// closure in `spawn_blocking` inside `timeout.validation`. `rebuild_export` runs the export
/// rebuild in `spawn_blocking` with a permit of the export gate. `export` limits a future with
/// `timeout.export`, which includes that rebuild. A panic in a closure becomes `E-WEB-011`, and
/// an elapsed limit becomes `E-WEB-009`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blocking {
    validation: Duration,
    export: Duration,
    mode: Mode,
}

impl Blocking {
    /// Build a runner with the limits of `timeout.validation` and `timeout.export`.
    #[must_use]
    pub const fn new(validation: Duration, export: Duration) -> Self {
        Self {
            validation,
            export,
            mode: Mode::Run,
        }
    }

    /// Return a test double whose `validate` reports the validation timeout and skips the work.
    #[cfg(any(test, feature = "memory"))]
    #[must_use]
    pub fn elapsed() -> Self {
        Self {
            mode: Mode::Elapsed,
            ..Self::default()
        }
    }

    /// Return a test double whose `validate` reports a panicked task and skips the work.
    #[cfg(any(test, feature = "memory"))]
    #[must_use]
    pub fn panicked() -> Self {
        Self {
            mode: Mode::Panicked,
            ..Self::default()
        }
    }

    /// Run `work` on a blocking thread and wait at most `timeout.validation`.
    ///
    /// When the limit elapses, the blocking task keeps running on the blocking pool, and its
    /// result is dropped. Tokio cannot stop a running blocking task.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Timeout` when the limit elapses.
    /// Returns `AppError::TaskFailed` when `work` panics.
    pub async fn validate<T, F>(&self, work: F) -> Result<T, AppError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        match self.mode {
            Mode::Run => {}
            #[cfg(any(test, feature = "memory"))]
            Mode::Elapsed => return Err(validation_timeout()),
            #[cfg(any(test, feature = "memory"))]
            Mode::Panicked => return Err(validation_failed()),
        }
        let handle = tokio::task::spawn_blocking(work);
        match tokio::time::timeout(self.validation, handle).await {
            Err(_elapsed) => Err(validation_timeout()),
            Ok(joined) => joined_or_failed(joined, TaskStage::Validation),
        }
    }

    /// Run the export `work` on a blocking thread and wait for it without a limit of its own.
    ///
    /// The future of `export` limits the wait, because `timeout.export` covers the rebuild.
    /// The blocking task holds `permit` and gives it back when it ends (ADR-0056 (c)). A caller
    /// that drops the future aborts the task. A task that has not started then never starts, and
    /// the drop of its closure gives the permit back. Tokio cannot stop a running blocking task.
    /// A running task keeps the permit until it ends. So at most `export.permits` export tasks
    /// run at the same time.
    ///
    /// # Errors
    ///
    /// Returns `AppError::TaskFailed` with the stage `Export` when `work` panics.
    pub(crate) async fn rebuild_export<T, F>(
        &self,
        permit: ExportPermit,
        work: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (handle, _abort_on_drop) = spawn_aborting(move || {
            let value = work();
            drop(permit);
            value
        });
        joined_or_failed(handle.await, TaskStage::Export)
    }

    /// Run `future` and wait at most `timeout.export`.
    ///
    /// The future covers the wait for an `export.permits` permit, the read, the locks, and the
    /// task of `rebuild_export` (ADR-0056). That task runs the rebuild, the hash compare, and the
    /// writer. The commit of the audit entry runs after this limit (ADR-0029).
    ///
    /// # Errors
    ///
    /// Returns `AppError::Timeout` when the limit elapses. Otherwise returns the result of `future`.
    pub async fn export<T, Fut>(&self, future: Fut) -> Result<T, AppError>
    where
        Fut: Future<Output = Result<T, AppError>>,
    {
        tokio::time::timeout(self.export, future)
            .await
            .unwrap_or(Err(AppError::Timeout {
                stage: TimeoutStage::Export,
            }))
    }

    /// Return the deadline of a diff: the time now plus `timeout.validation` (ADR-0050 (g)).
    ///
    /// The wait for the permit and the blocking task of the diff share this deadline.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Storage` when the deadline is beyond the range of the clock.
    pub(crate) fn diff_deadline(&self) -> Result<Instant, AppError> {
        Instant::now()
            .checked_add(self.validation)
            .ok_or_else(|| StorageError::invariant("the diff deadline has no instant").into())
    }

    /// Run the diff `work` on a blocking thread and wait for it until `deadline` (ADR-0050 (g)).
    ///
    /// The blocking task holds `permit` and gives it back with the result. When the deadline
    /// passes, the function aborts the task. A task that has not started then never starts, and
    /// the drop of its closure gives the permit back. A thread of the pool drops that closure
    /// when it takes the task from the queue, so the permit stays held until then. A caller that
    /// drops the future aborts the task in the same way. Tokio cannot stop a running blocking
    /// task. A running task keeps the permit until it ends. So at most `diff.permits` diffs run
    /// at the same time. A deadline that has already passed starts no task.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Timeout` with the stage `Diff` (`E-WEB-009`) when the deadline passes.
    /// Returns `AppError::TaskFailed` with the stage `Diff` (`E-WEB-011`) when `work` panics.
    pub(crate) async fn diff<T, F>(
        &self,
        deadline: Instant,
        permit: DiffPermit,
        work: F,
    ) -> Result<(T, DiffPermit), AppError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        match self.mode {
            Mode::Run => {}
            #[cfg(any(test, feature = "memory"))]
            Mode::Elapsed => return Err(diff_timeout()),
            #[cfg(any(test, feature = "memory"))]
            Mode::Panicked => {
                return Err(AppError::TaskFailed {
                    stage: TaskStage::Diff,
                });
            }
        }
        if Instant::now() >= deadline {
            return Err(diff_timeout());
        }
        let (mut handle, _abort_on_drop) = spawn_aborting(move || (work(), permit));
        match tokio::time::timeout_at(deadline, &mut handle).await {
            Err(_elapsed) => {
                handle.abort();
                Err(diff_timeout())
            }
            Ok(joined) => joined_or_failed(joined, TaskStage::Diff),
        }
    }
}

/// Aborts a blocking task when the guard drops.
///
/// The type solves one problem: a caller can drop the future of `Blocking::diff` or
/// `Blocking::rebuild_export`. For example, the client closes the connection, or a limit
/// elapses. A dropped `JoinHandle` only detaches its task.
/// Without the guard, a task that waits in the queue of the blocking pool starts later for no
/// reader and holds its permit. The abort keeps a queued task from its start. A thread of the
/// pool takes the task from the queue and drops its closure. That drop gives the permit back,
/// so the permit stays held until a thread is free. A task that already runs, or has ended,
/// ignores the abort.
struct AbortOnDrop(tokio::task::AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Spawn `work` on the blocking pool. Return its handle and a guard that aborts it on drop.
///
/// The function solves one problem: the diff and the export rebuild both move a permit into a
/// blocking task. A dropped future must keep a queued task from its start (see `AbortOnDrop`).
fn spawn_aborting<T, F>(work: F) -> (tokio::task::JoinHandle<T>, AbortOnDrop)
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let handle = tokio::task::spawn_blocking(work);
    let guard = AbortOnDrop(handle.abort_handle());
    (handle, guard)
}

const fn diff_timeout() -> AppError {
    AppError::Timeout {
        stage: TimeoutStage::Diff,
    }
}

/// A semaphore of permits that every clone of the gate shares.
///
/// The type solves one problem: the diff gate and the export gate bound parallel blocking work
/// in the same way. Each gate wraps this type and adds its own rule for the wait.
#[derive(Clone, Debug)]
struct Gate {
    permits: Arc<Semaphore>,
    /// The message of the invariant error when the gate is closed.
    closed: &'static str,
}

impl Gate {
    /// Build a gate with `permits` permits. A count above the largest count of the semaphore
    /// gives the largest count.
    fn new(permits: usize, closed: &'static str) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(permits.min(Semaphore::MAX_PERMITS))),
            closed,
        }
    }

    /// Wait for a permit without a limit of its own.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Storage` when the gate is closed. No code closes it.
    async fn acquire(&self) -> Result<OwnedSemaphorePermit, AppError> {
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .map_err(|_closed| StorageError::invariant(self.closed))?;
        Ok(permit)
    }

    /// Return the number of free permits.
    #[cfg(test)]
    fn available(&self) -> usize {
        self.permits.available_permits()
    }
}

/// The gate of the diffs that run at the same time (`diff.permits`, ADR-0050 (g)).
///
/// The type solves one problem: a diff page can hold much memory, and the `app` container has
/// 512 MiB. The gate bounds the parallel diffs. `AppContext::new` builds one gate, and every
/// clone of the context shares it.
#[derive(Clone, Debug)]
pub(crate) struct DiffGate {
    gate: Gate,
}

impl DiffGate {
    /// Build a gate with `permits` permits. A count above the largest count of the semaphore
    /// gives the largest count.
    pub(crate) fn new(permits: usize) -> Self {
        Self {
            gate: Gate::new(permits, "the diff gate is closed"),
        }
    }

    /// Wait for a permit until `deadline` (ADR-0050 (g)).
    ///
    /// # Errors
    ///
    /// Returns `AppError::Timeout` with the stage `Diff` (`E-WEB-009`) when the deadline passes.
    /// Returns `AppError::Storage` when the gate is closed. No code closes it.
    pub(crate) async fn acquire(&self, deadline: Instant) -> Result<DiffPermit, AppError> {
        let permit = tokio::time::timeout_at(deadline, self.gate.acquire())
            .await
            .map_err(|_elapsed| diff_timeout())??;
        Ok(DiffPermit { _permit: permit })
    }

    /// Return the number of free permits.
    #[cfg(test)]
    pub(crate) fn available(&self) -> usize {
        self.gate.available()
    }
}

/// The gate of the exports that run at the same time (`export.permits`, ADR-0056).
///
/// The type solves one problem: an export task holds the rows, the body, and the document in
/// memory, and Tokio cannot stop it after `timeout.export`. The gate bounds the parallel export
/// tasks. `AppContext::new` builds one gate, and every clone of the context shares it.
#[derive(Clone, Debug)]
pub(crate) struct ExportGate {
    gate: Gate,
}

impl ExportGate {
    /// Build a gate with `permits` permits. A count above the largest count of the semaphore
    /// gives the largest count.
    pub(crate) fn new(permits: usize) -> Self {
        Self {
            gate: Gate::new(permits, "the export gate is closed"),
        }
    }

    /// Wait for a permit (ADR-0056 (b)).
    ///
    /// The wait has no limit of its own. `ExportVersion` waits inside `Blocking::export`, so
    /// `timeout.export` limits the wait.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Storage` when the gate is closed. No code closes it.
    pub(crate) async fn acquire(&self) -> Result<ExportPermit, AppError> {
        let permit = self.gate.acquire().await?;
        Ok(ExportPermit { _permit: permit })
    }

    /// Return the number of free permits.
    #[cfg(test)]
    pub(crate) fn available(&self) -> usize {
        self.gate.available()
    }
}

/// A permit of the export gate (ADR-0056).
///
/// The type solves one problem: the permit must live as long as the blocking task of the
/// rebuild, also after `timeout.export`. `Blocking::rebuild_export` moves the permit into that
/// task, and the end of the task gives the permit back.
#[derive(Debug)]
pub(crate) struct ExportPermit {
    /// The permit. Its drop gives it back to the gate, so no code reads it.
    _permit: OwnedSemaphorePermit,
}

/// A permit of the diff gate (ADR-0050 (g)).
///
/// The type solves one problem: the permit must outlive the diff, also when the caller is gone.
/// `Blocking::diff` moves the permit into the blocking task and gets it back with the result.
/// The result of `DiffVersions` and `GetApprovalPreview` then holds the permit. `recipe-web`
/// drops the result after the page renders, and the drop gives the permit back to the gate.
#[derive(Debug)]
pub struct DiffPermit {
    /// The permit. Its drop gives it back to the gate, so no code reads it.
    _permit: OwnedSemaphorePermit,
}

impl Default for Blocking {
    /// Return the limits of `reliability.md` section 7.11: 10 seconds each.
    fn default() -> Self {
        Self::new(
            Duration::from_secs(DEFAULT_VALIDATION_SECONDS),
            Duration::from_secs(DEFAULT_EXPORT_SECONDS),
        )
    }
}

const fn validation_timeout() -> AppError {
    AppError::Timeout {
        stage: TimeoutStage::Validation,
    }
}

#[cfg(any(test, feature = "memory"))]
const fn validation_failed() -> AppError {
    AppError::TaskFailed {
        stage: TaskStage::Validation,
    }
}

/// Map the end of a blocking task to the result.
///
/// The function solves one problem: every blocking stage maps a `JoinError` the same way,
/// to `E-WEB-011` with its own stage.
fn joined_or_failed<T>(
    joined: Result<T, tokio::task::JoinError>,
    stage: TaskStage,
) -> Result<T, AppError> {
    joined.map_err(|_join_error| AppError::TaskFailed { stage })
}

/// The number of rows of one page of the machine list (`page.machines`, `reliability.md` section 7.11).
const DEFAULT_PAGE_MACHINES: u32 = 100;
/// The number of rows of one page of the type usage list (`page.type_usage`, `reliability.md` section 7.11).
const DEFAULT_PAGE_TYPE_USAGE: u32 = 100;
/// The number of rows of one page of the recipe list (`page.recipes`, `reliability.md` section 7.11).
const DEFAULT_PAGE_RECIPES: u32 = 100;
/// The number of rows of one page of the version list (`page.versions`, `reliability.md` section 7.11).
const DEFAULT_PAGE_VERSIONS: u32 = 100;
/// The number of rows of one page of the parameter table (`page.parameters`, `reliability.md` section 7.11).
const DEFAULT_PAGE_PARAMETERS: u32 = 100;
/// The number of rows of one page of the audit list (`page.audit`, `reliability.md` section 7.11).
const DEFAULT_PAGE_AUDIT: u32 = 100;
/// The number of rows of one page of the recipe history (`page.history`, `reliability.md` section 7.11).
const DEFAULT_PAGE_HISTORY: u32 = 100;
/// The number of rows of one page of the security events (`page.security_events`, `reliability.md` section 7.11).
const DEFAULT_PAGE_SECURITY_EVENTS: u32 = 100;
/// The number of parameters of one diff page (`page.diff`, `reliability.md` section 7.11).
const DEFAULT_PAGE_DIFF: u32 = 50;

/// The number of rows of one page of each list (`reliability.md` section 7.11).
///
/// The type solves one problem: the queries of the Plant Model and of the recipe pages cut a
/// list into pages. Each query reads its size from the settings and holds no number in its code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageSizes {
    /// The rows of one page of `ListMachines` (`page.machines`).
    pub machines: u32,
    /// The rows of one page of `GetTypeUsage` (`page.type_usage`).
    pub type_usage: u32,
    /// The rows of one page of `ListRecipes` (`page.recipes`).
    pub recipes: u32,
    /// The rows of one page of `GetRecipe` and `ListVersions` (`page.versions`).
    pub versions: u32,
    /// The rows of one page of `ListParameters` (`page.parameters`).
    pub parameters: u32,
    /// The rows of one page of `ListAuditEntries` (`page.audit`, ADR-0051 (k)).
    pub audit: u32,
    /// The rows of one page of `GetRecipeHistory` (`page.history`, ADR-0051 (k)).
    pub history: u32,
    /// The rows of one page of `ListSecurityEvents` (`page.security_events`, ADR-0051 (k)).
    pub security_events: u32,
    /// The parameters of one page of `DiffVersions` and `GetApprovalPreview` (`page.diff`, ADR-0050 (e)).
    pub diff: u32,
}

impl Default for PageSizes {
    /// Return the sizes of `reliability.md` section 7.11: 100 rows each.
    fn default() -> Self {
        Self {
            machines: DEFAULT_PAGE_MACHINES,
            type_usage: DEFAULT_PAGE_TYPE_USAGE,
            recipes: DEFAULT_PAGE_RECIPES,
            versions: DEFAULT_PAGE_VERSIONS,
            parameters: DEFAULT_PAGE_PARAMETERS,
            audit: DEFAULT_PAGE_AUDIT,
            history: DEFAULT_PAGE_HISTORY,
            security_events: DEFAULT_PAGE_SECURITY_EVENTS,
            diff: DEFAULT_PAGE_DIFF,
        }
    }
}

/// The number of rows of one parameter in one section of a diff (`diff.leaf_rows`, `reliability.md` section 7.11).
const DEFAULT_DIFF_LEAF_ROWS: u32 = 1_000;
/// The number of diffs that run at the same time (`diff.permits`, `reliability.md` section 7.11).
const DEFAULT_DIFF_PERMITS: usize = 2;

/// The bounds of a diff of two versions (ADR-0050 (e), (g), `reliability.md` section 7.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffSettings {
    /// The rows of one parameter in one section. The rest shows as a count (`diff.leaf_rows`).
    pub leaf_rows: u32,
    /// The diffs that run at the same time (`diff.permits`).
    pub permits: usize,
}

impl Default for DiffSettings {
    /// Return the bounds of `reliability.md` section 7.11: 1 000 rows and 2 permits.
    fn default() -> Self {
        Self {
            leaf_rows: DEFAULT_DIFF_LEAF_ROWS,
            permits: DEFAULT_DIFF_PERMITS,
        }
    }
}

/// The number of exports that run at the same time (`export.permits`, `reliability.md` section 7.11).
const DEFAULT_EXPORT_PERMITS: usize = 2;

/// The bounds of an export (ADR-0056, `reliability.md` section 7.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportSettings {
    /// The exports that run the read, the rebuild, the hash compare, and the writer at the same
    /// time (`export.permits`).
    pub permits: usize,
}

impl Default for ExportSettings {
    /// Return the bound of `reliability.md` section 7.11: 2 permits.
    fn default() -> Self {
        Self {
            permits: DEFAULT_EXPORT_PERMITS,
        }
    }
}

/// The settings that the application layer reads (`reliability.md` section 7.11).
///
/// The type solves one problem: use cases must not read a configuration file. `recipe-server`
/// builds this value and passes it down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppSettings {
    /// The limits of the blocking stages.
    pub blocking: Blocking,
    /// The approval rule (A34).
    pub approval_policy: ApprovalPolicy,
    /// The sizes of the pages of the lists.
    pub pages: PageSizes,
    /// The bounds of a diff of two versions.
    pub diff: DiffSettings,
    /// The bounds of an export.
    pub export: ExportSettings,
}

#[cfg(test)]
pub(crate) mod tests;
