//! The six jobs of the server and their loops (step 6 and step 9 of ADR-0061).
//!
//! The supervisor runs a job through the trait [`JobRun`]. The jobs live in `recipe-app`, so this
//! file adds the trait to each job. [`Jobs`] holds the job instances. The first run of
//! `disk_check` uses the same instance as its loop, so the numbers of the runs continue.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use recipe_app::error::AppError;
use recipe_app::jobs::RunKind;
use recipe_app::jobs::alerts::{AlertPorts, AlertSources, AlertVolumes, Alerts};
use recipe_app::jobs::audit_chain_check::{AuditChainCheck, AuditChainCheckPorts};
use recipe_app::jobs::cleanup::Cleanup;
use recipe_app::jobs::clock_check::ClockCheck;
use recipe_app::jobs::disk_check::DiskCheck;
use recipe_app::jobs::heartbeat::Heartbeat;
use recipe_app::operations::{AlertCounters, ReadinessState};
use recipe_app::ports::{AlertWebhook, Clock, DiskSpace, ReadOnlySwitch};
use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;

use super::host::{Host, MainPool};
use super::plan::ServeSettings;
use crate::schedule::{Schedule, ScheduleFault, TimeOfDay};
use crate::supervisor::{JobName, JobRun, Supervisor};

/// Implement [`JobRun`] for jobs whose method `run_once` has the shape of the trait.
macro_rules! job_run {
    ($($job:ty),+ $(,)?) => {
        $(
            impl JobRun for $job {
                async fn run(&self, request_id: RequestId, kind: RunKind) -> Result<(), AppError> {
                    self.run_once(request_id, kind).await
                }
            }
        )+
    };
}

job_run!(
    Cleanup,
    DiskCheck,
    AuditChainCheck,
    Heartbeat,
    ClockCheck,
    Alerts
);

/// Whether the start schedules the catch-up run of `audit_chain_check` (ADR-0060).
///
/// The run is due when no check exists or the last check is older than `age`. A last check in the
/// future gives no run, because a clock error must not start a long check.
pub(super) fn needs_catchup(last_check: Option<Timestamp>, now: Timestamp, age: Duration) -> bool {
    let Some(last_check) = last_check else {
        return true;
    };
    let elapsed = now
        .as_offset_date_time()
        .unix_timestamp_nanos()
        .checked_sub(last_check.as_offset_date_time().unix_timestamp_nanos());
    let limit = i128::try_from(age.as_nanos()).ok();
    matches!((elapsed, limit), (Some(elapsed), Some(limit)) if elapsed > limit)
}

/// Build the job `audit_chain_check` with the value of `audit.last_check_at` that the start read.
pub(super) fn audit_job(
    main: &MainPool,
    clock: Arc<dyn Clock>,
    last_check: Option<Timestamp>,
    settings: &ServeSettings,
) -> AuditChainCheck {
    let ports = AuditChainCheckPorts {
        chain: Arc::clone(&main.audit_chain),
        store: Arc::clone(&main.maintenance),
        operations: Arc::clone(&main.operations),
        sink: Arc::clone(&main.security_events),
        clock,
    };
    AuditChainCheck::new(ports, settings.audit_chain_check, last_check)
}

/// The job instances that exist from step 6 on.
///
/// The type solves one problem: the first run of `disk_check` and its loop must share one instance.
/// The other loops need their instances at step 9.
pub(super) struct Jobs {
    pub disk_check: Arc<DiskCheck>,
    cleanup: Arc<Cleanup>,
    heartbeat: Arc<Heartbeat>,
    clock_check: Arc<ClockCheck>,
    alerts: Arc<Alerts>,
}

/// What the jobs read from the host, the pool, and the process.
pub(super) struct JobInputs<'a> {
    pub host: &'a dyn Host,
    pub main: &'a MainPool,
    pub webhook: Arc<dyn AlertWebhook>,
    pub read_only: Arc<dyn ReadOnlySwitch>,
    pub readiness: ReadinessState,
    pub counters: AlertCounters,
}

impl Jobs {
    /// Build the five jobs that exist before step 9.
    pub(super) fn new(inputs: JobInputs<'_>, settings: &ServeSettings) -> Self {
        let JobInputs {
            host,
            main,
            webhook,
            read_only,
            readiness,
            counters,
        } = inputs;
        let clock = host.clock();
        let disk = host.disk();
        let disk_check = DiskCheck::new(
            Arc::clone(&disk),
            read_only,
            Arc::clone(&clock),
            settings.pgdata_path.clone(),
            settings.disk_check,
        );
        let cleanup = Cleanup::new(
            Arc::clone(&main.maintenance),
            Arc::clone(&clock),
            settings.identity.session_absolute_timeout,
            settings.cleanup,
        );
        let heartbeat = Heartbeat::new(Arc::clone(&main.maintenance), Arc::clone(&clock));
        let clock_check =
            ClockCheck::new(Arc::clone(&main.operations), clock, settings.clock_check);
        let alerts = alerts_job(host, main, webhook, disk, readiness, counters, settings);
        Self {
            disk_check: Arc::new(disk_check),
            cleanup: Arc::new(cleanup),
            heartbeat: Arc::new(heartbeat),
            clock_check: Arc::new(clock_check),
            alerts: Arc::new(alerts),
        }
    }

    /// Start the six loops on the supervisor (step 9).
    ///
    /// `audit` is the job `audit_chain_check`. `catchup` tells whether it gets the one-shot run.
    ///
    /// # Errors
    ///
    /// Returns an error when an interval of the settings is zero or a time of day is not valid.
    pub(super) fn spawn_loops(
        &self,
        supervisor: &mut Supervisor,
        audit: AuditChainCheck,
        catchup: bool,
        settings: &ServeSettings,
    ) -> Result<()> {
        let cleanup = every(JobName::Cleanup, settings.cleanup.loop_interval())?;
        supervisor.spawn_shared(JobName::Cleanup, cleanup, Arc::clone(&self.cleanup));
        let disk = every(JobName::DiskCheck, settings.disk_check_interval)?;
        supervisor.spawn_shared(JobName::DiskCheck, disk, Arc::clone(&self.disk_check));
        let nightly = audit_schedule(settings, catchup)?;
        supervisor.spawn_shared(JobName::AuditChainCheck, nightly, Arc::new(audit));
        let heartbeat = every(JobName::Heartbeat, settings.heartbeat_interval)?;
        supervisor.spawn_shared(JobName::Heartbeat, heartbeat, Arc::clone(&self.heartbeat));
        let clock = now_then_every(JobName::ClockCheck, settings.clock_check_interval)?;
        supervisor.spawn_shared(JobName::ClockCheck, clock, Arc::clone(&self.clock_check));
        let alerts = every(JobName::Alerts, settings.alerts.interval)?;
        supervisor.spawn_shared(JobName::Alerts, alerts, Arc::clone(&self.alerts));
        Ok(())
    }
}

/// Build the job `alerts` with the ports of the host and the shared counters.
fn alerts_job(
    host: &dyn Host,
    main: &MainPool,
    webhook: Arc<dyn AlertWebhook>,
    disk: Arc<dyn DiskSpace>,
    readiness: ReadinessState,
    counters: AlertCounters,
    settings: &ServeSettings,
) -> Alerts {
    let ports = AlertPorts {
        operations: Arc::clone(&main.operations),
        webhook,
        disk,
        backups: host.backups(&settings.backups_path),
    };
    let sources = AlertSources {
        counters,
        readiness,
    };
    let volumes = AlertVolumes {
        pgdata: settings.pgdata_path.clone(),
        backups: settings.backups_path.clone(),
        walarchive: settings.walarchive_path.clone(),
    };
    Alerts::new(ports, sources, volumes, settings.alerts)
}

fn every(job: JobName, interval: Duration) -> Result<Schedule> {
    Schedule::every(interval).map_err(|fault| schedule_error(job, fault))
}

fn now_then_every(job: JobName, interval: Duration) -> Result<Schedule> {
    Schedule::now_then_every(interval).map_err(|fault| schedule_error(job, fault))
}

/// The nightly schedule of `audit_chain_check`, with the catch-up run when `catchup` is true.
fn audit_schedule(settings: &ServeSettings, catchup: bool) -> Result<Schedule> {
    let time = TimeOfDay::parse(settings.audit_check_hour, settings.audit_check_minute)
        .map_err(|fault| schedule_error(JobName::AuditChainCheck, fault))?;
    let nightly = Schedule::daily_at(time);
    Ok(if catchup {
        nightly.with_once_after(settings.audit_catchup_delay)
    } else {
        nightly
    })
}

/// The error of a schedule that the constructor refuses, with the job and the reason.
fn schedule_error(job: JobName, fault: ScheduleFault) -> anyhow::Error {
    anyhow!(
        "the schedule of the job {} is not valid: {fault}",
        job.as_str()
    )
}

#[cfg(test)]
mod tests;
