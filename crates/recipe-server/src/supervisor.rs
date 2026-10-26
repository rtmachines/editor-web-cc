//! The supervisor of the background jobs (design D6, ADR-0060, `reliability.md` section 2.4).
//!
//! `recipe-app` holds one run of each job. This module holds the loop around it. For each job, the
//! supervisor runs one loop task. The loop asks its [`Schedule`] for the next wait, sleeps, and
//! then starts one run in its own `tokio::spawn` task. The join of that task is the place that
//! catches a panic of the job (`reliability.md` section 2.4 rule 6). Application code never calls
//! `catch_unwind`.
//!
//! The rules of the loop:
//!
//! - A loop starts the next run one interval after the end of the last run. A late run therefore
//!   gives no burst.
//! - After a panic, the loop waits `job.restart_delay` and starts the job again. An `Err` does not
//!   change the schedule.
//! - The supervisor opens the span `job` around each run, with the fields `job`, `iteration`, and
//!   `outcome` (ADR-0088 (a)). It logs a panic and an `Err` once inside that span and without a
//!   `request_id`. A job logs a handled fault itself (`reliability.md` section 1.1).
//! - [`Supervisor::run_at_start`] runs the first run of a job before the loops start, in the same
//!   span with `iteration` 1. The later loop of that job continues the numbers with 2.
//! - Each run gets its own `request_id`. The job writes it into the rows of its run.
//! - The cancel token stops the schedule and the restart wait at once. It does not stop a run that
//!   is open. The run ends by itself, or [`Supervisor::drain`] aborts it after the deadline.
//!
//! The supervisor reads neither the configuration nor the environment. The caller passes the
//! settings.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use recipe_app::error::AppError;
use recipe_app::identity::new_request_id;
use recipe_app::jobs::RunKind;
use recipe_app::ports::Clock;
use recipe_domain::audit::RequestId;
use tokio::task::{JoinError, JoinHandle, JoinSet};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::Instrument;

use crate::schedule::{Moment, Schedule};

/// The value of the span field `outcome` for a run that ended with `Ok`.
const OUTCOME_OK: &str = "ok";

/// The value of the span field `outcome` for a run that panicked.
const OUTCOME_PANIC: &str = "panic";

/// The name of a supervised job. It is the value of the log field `job`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobName {
    /// Delete expired sessions and old rows.
    Cleanup,
    /// Read the free disk space and set the read-only mode.
    DiskCheck,
    /// Verify the hash chain of the audit log.
    AuditChainCheck,
    /// Write the state key `heartbeat.last_at`.
    Heartbeat,
    /// Compare the clock of the host with the clock of the database.
    ClockCheck,
    /// Test the alert conditions and call the webhook.
    Alerts,
}

impl JobName {
    /// The name as it appears in the logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cleanup => "cleanup",
            Self::DiskCheck => "disk_check",
            Self::AuditChainCheck => "audit_chain_check",
            Self::Heartbeat => "heartbeat",
            Self::ClockCheck => "clock_check",
            Self::Alerts => "alerts",
        }
    }
}

/// The numbers of the supervisor. The caller fills them from `reliability.md` section 7.11.
///
/// The type has no default, because a default in code must equal that table and the caller owns
/// the configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorSettings {
    /// The wait before the supervisor starts a job again after a panic (`job.restart_delay`).
    pub restart_delay: Duration,
}

/// One run of a job.
///
/// The trait solves one problem: the supervisor must start a run of any job and give it the
/// `request_id` and the [`RunKind`] of that run, and the jobs live in another crate with
/// different state. The caller implements the trait for each job. The returned future must be
/// `Send`, because the supervisor runs it in its own task.
pub trait JobRun: Send + Sync + 'static {
    /// Run the job one time. Return `Err` only for an error of level `ERROR`.
    ///
    /// `kind` gives the kind of the run, such as a scheduled run or a catch-up run. A job with one
    /// trigger ignores it. The job logs a handled fault itself at `WARN`. The supervisor logs the
    /// `Err` once.
    fn run(
        &self,
        request_id: RequestId,
        kind: RunKind,
    ) -> impl Future<Output = Result<(), AppError>> + Send;
}

/// The result of [`Supervisor::drain`].
///
/// The type solves one problem: the caller writes the `WARN` line of the stop (design D4), so it
/// needs the count of the cut runs and not a log line of the supervisor. The type has its own
/// constructor path, because `listener::DrainOutcome` keeps its field private to its module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainOutcome {
    cut: usize,
}

impl DrainOutcome {
    /// The number of runs that the supervisor aborted at the end of the drain.
    #[must_use]
    pub const fn cut(self) -> usize {
        self.cut
    }

    /// Whether every run ended before the deadline.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        self.cut == 0
    }
}

/// How the start run of a job failed (see [`Supervisor::run_at_start`]).
///
/// The type solves one problem. The caller turns each failure of the start run into a
/// failure of start step 6. It must tell an `Err` from a panic. The supervisor writes no line
/// for either.
#[derive(Debug)]
pub enum StartRunFault {
    /// The run returned `Err`.
    Failed(AppError),
    /// The run panicked. The panic hook wrote the line of the panic.
    Panicked,
    /// The runtime cancelled the task of the run before it ended.
    Cancelled,
}

/// The function that makes the `request_id` of a loop run. Production uses `new_request_id`.
type RequestIdSource = fn() -> Result<RequestId, AppError>;

/// The supervisor of the jobs and the handle of the stop.
pub struct Supervisor {
    settings: SupervisorSettings,
    clock: Arc<dyn Clock>,
    stop: CancellationToken,
    loops: JoinSet<()>,
    open: Arc<AtomicUsize>,
    /// The number of start runs of each job that no loop has continued yet.
    start_runs: HashMap<JobName, u64>,
    /// The source of the `request_id` of each loop run.
    request_ids: RequestIdSource,
}

impl Supervisor {
    /// Make a supervisor without a job. The jobs stop when `stop` is cancelled.
    ///
    /// The supervisor keeps a child token of `stop`. It never cancels `stop` itself.
    #[must_use]
    pub fn new(
        settings: SupervisorSettings,
        clock: Arc<dyn Clock>,
        stop: CancellationToken,
    ) -> Self {
        Self {
            settings,
            clock,
            stop: stop.child_token(),
            loops: JoinSet::new(),
            open: Arc::new(AtomicUsize::new(0)),
            start_runs: HashMap::new(),
            request_ids: new_request_id,
        }
    }

    /// Replace the source of the `request_id` of each loop run. A test makes the source fail.
    #[cfg(test)]
    pub(super) fn with_request_ids(mut self, request_ids: RequestIdSource) -> Self {
        self.request_ids = request_ids;
        self
    }

    /// Run the first run of a job now, before the loops start, and wait for its end.
    ///
    /// The run gets the span `job` with `iteration` 1, a new `request_id`, and
    /// [`RunKind::Scheduled`].
    /// The span field `outcome` shows `ok`, the error code, or `panic` after the join. The method
    /// writes no line for an `Err` or a panic, because the caller owns the failure of the start.
    /// A drop of the returned future aborts the run at once.
    /// Call it before [`Supervisor::spawn_shared`] of the same job. That loop then continues the
    /// numbers with 2 on the same job instance, and it starts one schedule wait after its spawn.
    ///
    /// # Errors
    ///
    /// Returns [`StartRunFault`] when the run returns `Err`, panics, or loses its task.
    pub async fn run_at_start<R: JobRun>(
        &mut self,
        name: JobName,
        run: &Arc<R>,
    ) -> Result<(), StartRunFault> {
        let iteration = self.count_start_run(name);
        let span = job_span(name, iteration);
        let request_id = new_request_id().map_err(|error| {
            record_error_code(&span, &error);
            StartRunFault::Failed(error)
        })?;
        let kind = RunKind::Scheduled;
        match run_in_task(Arc::clone(run), &self.open, &span, request_id, kind).await {
            Ok(Ok(())) => {
                span.record("outcome", OUTCOME_OK);
                Ok(())
            }
            Ok(Err(error)) => {
                record_error_code(&span, &error);
                Err(StartRunFault::Failed(error))
            }
            Err(joined) if joined.is_panic() => {
                span.record("outcome", OUTCOME_PANIC);
                Err(StartRunFault::Panicked)
            }
            Err(_) => Err(StartRunFault::Cancelled),
        }
    }

    /// Add one start run of `name` and return its number. The first start run has number 1.
    fn count_start_run(&mut self, name: JobName) -> u64 {
        let count = self.start_runs.entry(name).or_insert(0);
        // One start run per job at startup, so the count cannot reach u64::MAX.
        *count = count.checked_add(1).unwrap_or(*count);
        *count
    }

    /// Start the loop of one job. The loop takes ownership of `run`. The time since the start of
    /// the schedule begins now. Use [`Supervisor::spawn_shared`] to keep a handle on the job.
    pub fn spawn<R: JobRun>(&mut self, name: JobName, schedule: Schedule, run: R) {
        self.spawn_shared(name, schedule, Arc::new(run));
    }

    /// Start the loop of one job on a job instance that the caller keeps.
    ///
    /// The loop continues the numbers of [`Supervisor::run_at_start`] of the same job.
    /// The caller passes the same `Arc` to `run_at_start` and then to `spawn_shared`.
    pub fn spawn_shared<R: JobRun>(&mut self, name: JobName, schedule: Schedule, run: Arc<R>) {
        let job = JobLoop {
            name,
            schedule,
            run,
            restart_delay: self.settings.restart_delay,
            clock: Arc::clone(&self.clock),
            stop: self.stop.child_token(),
            open: Arc::clone(&self.open),
            request_ids: self.request_ids,
            started: Instant::now(),
            iteration: self.start_runs.remove(&name).unwrap_or(0),
        };
        self.loops.spawn(job.run_loop());
    }

    /// Stop the jobs and wait for the open runs up to the deadline.
    ///
    /// The function first cancels the child token of the supervisor, so no loop starts a run
    /// after the call. It never cancels the token of the caller. A run that is open may end
    /// by itself before the deadline. After the deadline, the function aborts every open run.
    /// It does not wait for an aborted run, because a run that does not yield would block
    /// the stop without a bound. The pool close of the caller waits for the database connections
    /// of the aborted runs, and an open transaction rolls back (ADR-0060 (j), ADR-0087 (d)).
    pub async fn drain(mut self, deadline: Duration) -> DrainOutcome {
        self.stop.cancel();
        let finished = tokio::time::timeout(deadline, async {
            while let Some(joined) = self.loops.join_next().await {
                collect(joined);
            }
        })
        .await
        .is_ok();
        if finished {
            return DrainOutcome { cut: 0 };
        }
        while let Some(joined) = self.loops.try_join_next() {
            collect(joined);
        }
        // The count and the abort follow each other with no `.await` between them.
        let cut = self.open.load(Ordering::Acquire);
        self.loops.abort_all();
        DrainOutcome { cut }
    }
}

/// Take one finished loop out of the set. A loop ends by itself after the cancel.
///
/// The loop code has no panic path. A panic here is a fault of the supervisor, so it gets an
/// `ERROR` line, and the other jobs go on.
fn collect(joined: Result<(), JoinError>) {
    if let Err(error) = joined
        && error.is_panic()
    {
        tracing::error!("the loop of a job panicked and the job stopped");
    }
}

/// What one run did, as far as the loop needs to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The run ended with `Ok`, with `Err`, or was cancelled. The schedule goes on.
    Ended,
    /// The run panicked. The loop waits for the restart delay.
    Panicked,
}

/// The state of one job loop.
struct JobLoop<R> {
    name: JobName,
    schedule: Schedule,
    run: Arc<R>,
    restart_delay: Duration,
    clock: Arc<dyn Clock>,
    stop: CancellationToken,
    open: Arc<AtomicUsize>,
    request_ids: RequestIdSource,
    started: Instant,
    /// The number of the last run since the start of the process. The first run has number 1.
    iteration: u64,
}

impl<R: JobRun> JobLoop<R> {
    async fn run_loop(mut self) {
        let mut outcome = Outcome::Ended;
        // A restart after a panic repeats the kind of the run that panicked (ADR-0088 (b)).
        let mut kind = RunKind::Scheduled;
        loop {
            let next = match outcome {
                Outcome::Panicked => Some((self.restart_delay, kind)),
                Outcome::Ended => self.schedule.next_delay(self.moment()),
            };
            let Some((delay, next_kind)) = next else {
                return;
            };
            kind = next_kind;
            if !self.wait(delay).await {
                return;
            }
            outcome = self.iterate(kind).await;
            if self.stop.is_cancelled() {
                return;
            }
        }
    }

    /// The time since the start and the wall clock in UTC.
    fn moment(&self) -> Moment {
        let now = self.clock.now();
        let instant = now.as_offset_date_time();
        Moment::new(
            self.started.elapsed(),
            instant.unix_timestamp(),
            instant.nanosecond(),
        )
    }

    /// Sleep for `delay`. Return `false` when the cancel came first.
    async fn wait(&self, delay: Duration) -> bool {
        tokio::select! {
            biased;
            () = self.stop.cancelled() => false,
            () = tokio::time::sleep(delay) => true,
        }
    }

    /// Run the job once, with the run kind `kind`, in its own task and wait for the join.
    ///
    /// The span `job` encloses the run (ADR-0088 (a)). The task carries the span, so every event
    /// of the run gets the field `job`, also under the filter `warn` or `error`. The loop records
    /// the field `outcome` after the join.
    async fn iterate(&mut self, kind: RunKind) -> Outcome {
        // Each run adds one. A process cannot run long enough to reach u64::MAX.
        self.iteration = self.iteration.checked_add(1).unwrap_or(self.iteration);
        let span = job_span(self.name, self.iteration);
        let request_id = match (self.request_ids)() {
            Ok(request_id) => request_id,
            Err(error) => return log_failed_start(&span, &error),
        };
        let run = Arc::clone(&self.run);
        match run_in_task(run, &self.open, &span, request_id, kind).await {
            Ok(Ok(())) => {
                span.record("outcome", OUTCOME_OK);
                Outcome::Ended
            }
            Ok(Err(error)) => log_failed_run(self.name, &span, &error),
            Err(joined) if joined.is_panic() => {
                span.record("outcome", OUTCOME_PANIC);
                span.in_scope(|| {
                    tracing::error!(
                        "the job run panicked, the supervisor starts the job again after the restart delay"
                    );
                });
                Outcome::Panicked
            }
            // Nobody aborts a run while its loop waits, so this branch is only a safe default.
            Err(_) => Outcome::Ended,
        }
    }
}

/// Open the span `job` of one run (ADR-0088 (a)).
///
/// The level is `ERROR`, so a filter above `INFO` keeps the span with any line of the run.
fn job_span(name: JobName, iteration: u64) -> tracing::Span {
    tracing::error_span!(
        "job",
        job = name.as_str(),
        iteration = iteration,
        outcome = tracing::field::Empty
    )
}

/// Run `run` once in its own task inside `span` and wait for the join.
///
/// The join returns the panic of the run as `Err` (`reliability.md` section 2.4 rule 6). Dropping
/// the future aborts the task, and the run counts as open until the task ends.
async fn run_in_task<R: JobRun>(
    run: Arc<R>,
    open: &Arc<AtomicUsize>,
    span: &tracing::Span,
    request_id: RequestId,
    kind: RunKind,
) -> Result<Result<(), AppError>, JoinError> {
    let guard = OpenRun::enter(open);
    let mut task = AbortOnDrop(tokio::spawn(
        async move {
            let _guard = guard;
            run.run(request_id, kind).await
        }
        .instrument(span.clone()),
    ));
    (&mut task.0).await
}

/// Record the code of a failed run in the span field `outcome` and return the code.
fn record_error_code(span: &tracing::Span, error: &AppError) -> String {
    let code = error.code().to_string();
    span.record("outcome", code.as_str());
    code
}

/// Record the code of a run that could not start and write its one `ERROR` line.
///
/// The run never starts when the `request_id` fails. This is a fault of the supervisor and not a
/// handled fault of the job, so the level is `ERROR` for every job, also for `alerts`.
fn log_failed_start(span: &tracing::Span, error: &AppError) -> Outcome {
    let code = record_error_code(span, error);
    let chain = display_chain(error);
    span.in_scope(|| {
        tracing::error!(
            error_code = %code,
            chain = %chain,
            "the job run could not start, the schedule goes on"
        );
    });
    Outcome::Ended
}

/// Record the code of a failed run in `span` and write the one line of the run.
///
/// The level is `WARN` only for a failed run of the job `alerts`, which is a failed snapshot read
/// and a handled fault. The level is `ERROR` for every other job (ADR-0089 (d)). The line stays
/// inside the span, so it gets the field `job` from the span and does not repeat it.
fn log_failed_run(name: JobName, span: &tracing::Span, error: &AppError) -> Outcome {
    let code = record_error_code(span, error);
    let chain = display_chain(error);
    span.in_scope(|| {
        if name == JobName::Alerts {
            tracing::warn!(
                error_code = %code,
                chain = %chain,
                "the job run failed, the schedule goes on"
            );
        } else {
            tracing::error!(
                error_code = %code,
                chain = %chain,
                "the job run failed, the schedule goes on"
            );
        }
    });
    Outcome::Ended
}

/// Return the `Display` text of an error and of each of its sources, joined by `: `.
///
/// The text never comes from `Debug`, because the `Debug` of a database error can hold a row value
/// (ADR-0035). The web layer has the same helper for the log line of a request.
fn display_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        source = inner.source();
    }
    text
}

/// Counts a run while its future is alive.
///
/// The guard lives inside the task of the run. It drops when the run ends, panics, or is aborted,
/// so [`Supervisor::drain`] counts exactly the runs that are open.
struct OpenRun(Arc<AtomicUsize>);

impl OpenRun {
    fn enter(open: &Arc<AtomicUsize>) -> Self {
        open.fetch_add(1, Ordering::AcqRel);
        Self(Arc::clone(open))
    }
}

impl Drop for OpenRun {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Aborts the task of a run when the loop that waits for it goes away.
///
/// The type solves one problem: dropping a `JoinHandle` detaches the task. An aborted loop must
/// take its open run with it, or the run would go on after the drain.
struct AbortOnDrop(JoinHandle<Result<(), AppError>>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod chain_tests;
#[cfg(test)]
mod drain_tests;
#[cfg(test)]
mod span_tests;
#[cfg(test)]
mod start_run_tests;
#[cfg(test)]
mod support;
#[cfg(test)]
mod tests;
