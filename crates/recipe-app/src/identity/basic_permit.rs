//! The gate that limits the Basic checks of the export routes (design D2 step 1, ADR-0109 (n)).

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::{AppError, TaskStage, TimeoutStage};

/// The gate in front of the Basic checks.
///
/// The type solves one problem: a flood of Basic checks must not use all `argon2.permits`, because
/// the UI logins share them. A semaphore with `export.basic_permits` permits limits the checks
/// that wait for or hold an Argon2 permit at the same time. A caller waits at most
/// `argon2.acquire_timeout`.
#[derive(Clone)]
pub(crate) struct BasicGate {
    semaphore: Arc<Semaphore>,
    acquire_timeout: Duration,
}

impl BasicGate {
    /// Build a gate with `permits` permits and the wait limit `acquire_timeout`.
    pub(crate) fn new(permits: usize, acquire_timeout: Duration) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
            acquire_timeout,
        }
    }

    /// Wait for a Basic permit, at most `argon2.acquire_timeout`.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Timeout` with the stage `Argon2Permit` (`E-WEB-009`) when the limit
    /// elapses. Returns `AppError::TaskFailed` when the semaphore is closed, which never happens.
    pub(crate) async fn acquire(&self) -> Result<BasicPermit, AppError> {
        let waiting = Arc::clone(&self.semaphore).acquire_owned();
        let permit = tokio::time::timeout(self.acquire_timeout, waiting)
            .await
            .map_err(|_elapsed| AppError::Timeout {
                stage: TimeoutStage::Argon2Permit,
            })?
            .map_err(|_closed| AppError::TaskFailed {
                stage: TaskStage::PasswordHash,
            })?;
        Ok(BasicPermit { _permit: permit })
    }
}

/// One permit of the Basic gate.
///
/// The type solves one problem: the permit must live as long as the Argon2 permit of the same
/// Basic check. The `HashPermit` of the check owns it and moves it into the blocking task. A drop
/// gives the permit back.
#[derive(Debug)]
#[must_use = "the permit limits the Basic checks only while the caller holds it"]
pub struct BasicPermit {
    _permit: OwnedSemaphorePermit,
}

#[cfg(test)]
mod tests;
