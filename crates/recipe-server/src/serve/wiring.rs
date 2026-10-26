//! Step 6 of the start: the use case context, the web state, and the health state.
//!
//! The functions build values and open nothing. They read the adapters of the main pool and the
//! numbers of the plan. The one `AlertCounters` of the process goes into the use cases here.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use recipe_app::context::AppContext;
use recipe_app::error::{AppError, TaskStage};
use recipe_app::identity::{HashGate, IdentityContext, PasswordHasher};
use recipe_app::operations::{AlertCounters, ReadinessState};
use recipe_web::{HealthState, WebState};

use super::host::{HasherBuild, Host, MainPool};
use super::plan::Plan;
use crate::coded::coded;

/// Build the web state: the Argon2id hasher, the use cases, and the text bundles.
///
/// The state starts with the read-only mode off. The job `disk_check` switches it.
///
/// # Errors
///
/// Returns an error when Argon2 refuses its parameters, the hasher task fails, or a text bundle
/// is not valid. A failed task has the stage `PasswordHash` and no secret.
pub(super) async fn web_state(
    host: &dyn Host,
    plan: &Plan,
    main: &MainPool,
    counters: &AlertCounters,
) -> Result<WebState> {
    let settings = &plan.settings;
    let identity = &settings.identity;
    let hasher = build_hasher(host.password_hasher(identity.argon2)).await?;
    let gate = HashGate::new(
        identity.argon2_permits,
        identity.argon2_acquire_timeout,
        hasher,
    );
    let context = IdentityContext::new(
        app_context(host, plan, main, counters),
        Arc::clone(&main.login_attempts),
        Arc::clone(&main.sessions),
        gate,
        plan.login_keys.clone(),
        *identity,
    );
    WebState::new(context, settings.web.clone(), plan.csrf_keys.clone())
        .map_err(|reason| anyhow!("{reason}"))
}

/// Build the context of the use cases with the one `AlertCounters` of the process.
///
/// The function solves one problem: the job `alerts` reads the counters that the use cases write.
/// A context with its own counters would hide the export results `E-EXP-002` from the job.
pub(super) fn app_context(
    host: &dyn Host,
    plan: &Plan,
    main: &MainPool,
    counters: &AlertCounters,
) -> AppContext {
    AppContext::new(
        Arc::clone(&main.unit_of_work),
        Arc::clone(&main.security_events),
        host.clock(),
        plan.settings.app,
    )
    .with_counters(counters.clone())
}

/// Run the build of the Argon2id hasher in `spawn_blocking`.
///
/// The build hashes once. On the runtime thread it would block the signal race of the start.
async fn build_hasher(build: HasherBuild) -> Result<Arc<dyn PasswordHasher>> {
    let built = tokio::task::spawn_blocking(build).await.map_err(|_join| {
        coded(AppError::TaskFailed {
            stage: TaskStage::PasswordHash,
        })
    })?;
    built.context("the password hasher cannot start")
}

/// Build the readiness state and the health state of the internal listener.
pub(super) fn health_state(host: &dyn Host, plan: &Plan) -> (ReadinessState, HealthState) {
    let clock = host.clock();
    let readiness = ReadinessState::new(clock.now());
    let health = HealthState::new(readiness.clone(), clock, plan.settings.health);
    (readiness, health)
}

#[cfg(test)]
mod tests;
