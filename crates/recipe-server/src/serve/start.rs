//! The ten steps of the start (design D4, ADR-0061).
//!
//! The function [`start`] runs the steps in order. Steps 2 to 4 run in synchronous code, because
//! the runtime does not exist before step 3. Steps 5 to 10 run in one `block_on` that races a stop
//! signal (rule (f)). Every resource that a step opens goes into [`Resources`] at once.
//! The unwind then closes it in the same order for a failure, a signal, and a finished server.

use std::future::Future;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::task::Poll;

use anyhow::{Context, Result, anyhow, bail};
use axum::Router;
use recipe_app::identity::new_request_id;
use recipe_app::operations::{AlertCounters, CompareSettings};
use recipe_app::ports::{AlertWebhook, ReadinessProbe, TimeStateKey};
use recipe_web::{HealthState, ReadOnlyMode, WebState, health_router};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

use super::host::{HealthPool, Host, ListenerKind, MainPool, Signal, WatchdogControl};
use super::jobs::{JobInputs, Jobs, audit_job, needs_catchup};
use super::opened::{Opened, Resources};
use super::plan::Plan;
use super::server::{ListenerHandle, Server};
use super::wiring;
use crate::coded::coded;
use crate::listener::{AcceptLoop, ListenerSettings};
use crate::supervisor::{JobName, StartRunFault, Supervisor, SupervisorSettings};

/// The result of a start that did not fail.
pub enum Outcome {
    /// All ten steps passed.
    Running(Box<Server>),
    /// A signal came before step 10. The start closed what it had opened.
    Interrupted,
}

/// Turn the failure of the first run of a job into the error of the start (step 6).
pub(in crate::serve) fn start_run_error(job: JobName, fault: StartRunFault) -> anyhow::Error {
    let name = job.as_str();
    match fault {
        StartRunFault::Failed(error) => {
            coded(error).context(format!("the first run of the job {name} failed"))
        }
        StartRunFault::Panicked => anyhow!("the first run of the job {name} panicked"),
        StartRunFault::Cancelled => {
            anyhow!("the runtime cancelled the first run of the job {name}")
        }
    }
}

/// Run the steps 2 to 10 of the start.
///
/// # Errors
///
/// Returns the error of the first step that fails. The start closes what it opened before. The
/// caller writes the one `ERROR` line and exits with code 1 (rule (b)).
pub fn start(host: &dyn Host, plan: Plan, counters: AlertCounters) -> Result<Outcome> {
    let Prepared {
        watchdog,
        runtime,
        mut signal,
        webhook,
    } = prepare(host, &plan)?;
    let mut resources = Resources::default();
    let steps = Steps {
        host,
        plan: &plan,
        counters: &counters,
        webhook,
        watchdog: watchdog.as_ref(),
        resources: &mut resources,
    };
    let result = runtime.block_on(race(&mut signal, steps.run()));
    let opened = Opened {
        watchdog,
        runtime,
        resources,
        shutdown: plan.settings.shutdown,
    };
    let outcome = finish(result, opened, signal)?;
    if matches!(outcome, Outcome::Running(_)) {
        log_started(&plan);
    }
    Ok(outcome)
}

/// Write the one `INFO` event of a start that passed step 10 (`reliability.md` 3.3).
fn log_started(plan: &Plan) {
    tracing::info!(
        public_port = plan.configured.public_port,
        internal_port = plan.configured.internal_port,
        "the server started"
    );
}

/// What steps 2 to 4 produce.
struct Prepared {
    watchdog: Box<dyn WatchdogControl>,
    runtime: Runtime,
    signal: Signal,
    webhook: Arc<dyn AlertWebhook>,
}

/// Steps 2 to 4: the webhook client, the watchdog thread, the runtime, and the signal stream.
fn prepare(host: &dyn Host, plan: &Plan) -> Result<Prepared> {
    let ca_file = plan.configured.webhook_ca_file.as_deref().map(Path::new);
    let webhook = host.webhook(plan.webhook_url.as_ref(), ca_file)?;
    let watchdog = host
        .start_watchdog(plan.settings.watchdog, plan.configured.internal_port)
        .context("the watchdog thread cannot start")?;
    let runtime = match host.build_runtime(plan.settings.runtime_workers) {
        Ok(runtime) => runtime,
        Err(error) => return Err(abandon(watchdog, error, "the runtime cannot start")),
    };
    let signal = match register_signals(host, &runtime) {
        Ok(signal) => signal,
        Err(error) => {
            return Err(abandon(
                watchdog,
                error,
                "the signal handlers cannot register",
            ));
        }
    };
    Ok(Prepared {
        watchdog,
        runtime,
        signal,
        webhook,
    })
}

/// Register the signal stream. The handlers need the runtime of step 3 as the current runtime.
fn register_signals(host: &dyn Host, runtime: &Runtime) -> io::Result<Signal> {
    let _entered = runtime.enter();
    host.register_signals()
}

/// End the watchdog thread of a start that fails before the async steps, and build the error.
fn abandon(
    watchdog: Box<dyn WatchdogControl>,
    error: io::Error,
    what: &'static str,
) -> anyhow::Error {
    // A panic of the thread has its line from the panic hook. The start fails anyway.
    let _stopped = watchdog.stop();
    anyhow::Error::new(error).context(what)
}

/// Race the steps against the stop signal. The signal wins a tie.
///
/// A step can fire the signal and end in the same poll. The `Ok` case never reaches this function
/// with a stale signal, because [`Steps::run`] yields between the steps. A failure that ends in the
/// poll of the signal still needs one more look at the signal (ADR-0061).
async fn race<F>(signal: &mut Signal, steps: F) -> Result<Option<ReadOnlyMode>>
where
    F: Future<Output = Result<ReadOnlyMode>>,
{
    let result = tokio::select! {
        biased;
        () = &mut *signal => return Ok(None),
        result = steps => result,
    };
    if result.is_err() && stop_is_pending(signal).await {
        return Ok(None);
    }
    result.map(Some)
}

/// Poll the stop signal once. Return `true` when it fired.
async fn stop_is_pending(signal: &mut Signal) -> bool {
    std::future::poll_fn(|context| Poll::Ready(signal.as_mut().poll(context).is_ready())).await
}

/// Give the race one look at the stop signal between two steps.
///
/// The race polls the signal first. A step that fires the signal and ends in the same poll would
/// otherwise let the next step run. The yield ends the poll, so the race sees the signal.
async fn between_steps() {
    tokio::task::yield_now().await;
}

/// Turn the result of the steps into the outcome. Close what is open when the start ends early.
fn finish(result: Result<Option<ReadOnlyMode>>, opened: Opened, signal: Signal) -> Result<Outcome> {
    match result {
        Ok(Some(read_only)) => match opened.into_parts(signal, read_only) {
            Ok(parts) => Ok(Outcome::Running(Box::new(Server::new(parts)))),
            Err(opened) => {
                opened.close().log();
                bail!("a step of the start left a resource out")
            }
        },
        Ok(None) => {
            opened.close().log();
            Ok(Outcome::Interrupted)
        }
        Err(error) => {
            opened.close().log();
            Err(error)
        }
    }
}

/// The inputs of steps 5 to 10 and the place where they store what they open.
struct Steps<'a> {
    host: &'a dyn Host,
    plan: &'a Plan,
    counters: &'a AlertCounters,
    webhook: Arc<dyn AlertWebhook>,
    watchdog: &'a dyn WatchdogControl,
    resources: &'a mut Resources,
}

impl Steps<'_> {
    /// Steps 5 to 10. Return the read-only flag that the stop and `disk_check` share.
    async fn run(mut self) -> Result<ReadOnlyMode> {
        let (main, health_pool) = self.open_pools().await?;
        between_steps().await;
        let web = wiring::web_state(self.host, self.plan, &main, self.counters).await?;
        let (jobs, health) = self.first_disk_check(&main, &web).await?;
        between_steps().await;
        self.compare_settings(&main).await?;
        between_steps().await;
        self.open_internal(&health_pool, health.clone()).await?;
        between_steps().await;
        self.start_loops(&main, &jobs).await?;
        between_steps().await;
        let read_only = web.read_only().clone();
        // No look at the signal after step 10. A start that passed it runs.
        self.open_public(web, &health).await?;
        Ok(read_only)
    }

    /// Step 5: open both pools and check the role of the connection.
    async fn open_pools(&mut self) -> Result<(MainPool, HealthPool)> {
        let settings = &self.plan.settings;
        let main = self
            .host
            .open_main_pool(&settings.main_pool)
            .await
            .context("the main pool cannot open")?;
        self.resources.main_pool = Some(Arc::clone(&main.closer));
        let health = self
            .host
            .open_health_pool(&settings.health_pool())
            .await
            .context("the health pool cannot open")?;
        self.resources.health_pool = Some(Arc::clone(&health.closer));
        check_role(&main).await?;
        Ok((main, health))
    }

    /// Step 6: build the jobs and the health state, then run `disk_check` once.
    async fn first_disk_check(
        &mut self,
        main: &MainPool,
        web: &WebState,
    ) -> Result<(Jobs, HealthState)> {
        let (readiness, health) = wiring::health_state(self.host, self.plan);
        self.resources.health = Some(health.clone());
        let jobs = Jobs::new(
            JobInputs {
                host: self.host,
                main,
                webhook: Arc::clone(&self.webhook),
                read_only: Arc::new(web.read_only().clone()),
                readiness,
                counters: self.counters.clone(),
            },
            &self.plan.settings,
        );
        let restart_delay = self.plan.settings.job_restart_delay;
        let supervisor = self.resources.supervisor.insert(Supervisor::new(
            SupervisorSettings { restart_delay },
            self.host.clock(),
            CancellationToken::new(),
        ));
        supervisor
            .run_at_start(JobName::DiskCheck, &jobs.disk_check)
            .await
            .map_err(|fault| start_run_error(JobName::DiskCheck, fault))?;
        Ok((jobs, health))
    }

    /// Step 7: write one audit entry for each setting that changed since the last start.
    async fn compare_settings(&self, main: &MainPool) -> Result<()> {
        let request_id = new_request_id().map_err(coded)?;
        CompareSettings::new(Arc::clone(&main.unit_of_work))
            .run(&self.plan.configured, request_id)
            .await
            .map_err(|error| coded(error).context("the comparison of the settings failed"))?;
        Ok(())
    }

    /// Step 8: open the internal listener, then arm the watchdog (rule (g)).
    async fn open_internal(&mut self, pool: &HealthPool, health: HealthState) -> Result<()> {
        let port = self.plan.configured.internal_port;
        let listener = self
            .host
            .bind(ListenerKind::Internal, port)
            .await
            .context("the internal listener cannot bind its port")?;
        let router = health_router(
            Arc::clone(&pool.readiness) as Arc<dyn ReadinessProbe>,
            health,
        );
        let settings = self.plan.settings.listener(ListenerKind::Internal);
        self.resources.internal = Some(spawn_listener(
            ListenerKind::Internal,
            router,
            settings,
            listener,
        ));
        self.watchdog.arm();
        Ok(())
    }

    /// Step 9: start the loops of the six jobs.
    async fn start_loops(&mut self, main: &MainPool, jobs: &Jobs) -> Result<()> {
        let settings = &self.plan.settings;
        let last_check = main
            .operations
            .state(TimeStateKey::AuditLastCheckAt)
            .await
            .map_err(|error| coded(error).context("the last audit check cannot be read"))?;
        let clock = self.host.clock();
        let catchup = needs_catchup(last_check, clock.now(), settings.audit_catchup_age);
        let audit = audit_job(main, clock, last_check, settings);
        let supervisor = self
            .resources
            .supervisor
            .as_mut()
            .ok_or_else(|| anyhow!("the supervisor is missing"))?;
        jobs.spawn_loops(supervisor, audit, catchup, settings)
    }

    /// Step 10: open the public listener, then open the startup gate of `/readyz`.
    ///
    /// Until the bind succeeds, `/readyz` answers `not ready` (ADR-0061 step 10, ADR-0091 (e)).
    async fn open_public(&mut self, web: WebState, health: &HealthState) -> Result<()> {
        let port = self.plan.configured.public_port;
        let listener = self
            .host
            .bind(ListenerKind::Public, port)
            .await
            .context("the public listener cannot bind its port")?;
        let router = recipe_web::router(web);
        let settings = self.plan.settings.listener(ListenerKind::Public);
        self.resources.public = Some(spawn_listener(
            ListenerKind::Public,
            router,
            settings,
            listener,
        ));
        health.open_gate();
        Ok(())
    }
}

/// Check the role of the connection (step 5). A role with too many rights is a refusal.
async fn check_role(main: &MainPool) -> Result<()> {
    let role = main
        .operations
        .role_check()
        .await
        .map_err(|error| coded(error).context("the role of the connection cannot be read"))?;
    if !role.is_allowed() {
        bail!("the role of the connection is not allowed");
    }
    Ok(())
}

/// Start the accept loop of one listener on the runtime.
fn spawn_listener(
    kind: ListenerKind,
    router: Router,
    settings: ListenerSettings,
    listener: TcpListener,
) -> ListenerHandle {
    let stop = CancellationToken::new();
    let task = tokio::spawn(AcceptLoop::new(router, settings, stop.clone()).run(listener));
    ListenerHandle::new(kind, stop, task)
}
