//! The gate that limits the Argon2 work (design D5, finding F2).

use std::sync::Arc;
use std::time::Duration;

use recipe_domain::identity::{NewPassword, Password, PasswordHash};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::PasswordHasher;
use crate::error::{AppError, TaskStage, TimeoutStage};

/// The error for a blocking task that did not complete (`E-WEB-011`).
const fn hash_task_failed() -> AppError {
    AppError::TaskFailed {
        stage: TaskStage::PasswordHash,
    }
}

/// The gate in front of every Argon2 call.
///
/// The type solves one problem: a flood of logins must not fill the blocking pool with
/// 19 MiB hashes. A semaphore with `argon2.permits` permits limits the work, and a caller waits
/// at most `argon2.acquire_timeout` for a permit.
#[derive(Clone)]
pub struct HashGate {
    semaphore: Arc<Semaphore>,
    acquire_timeout: Duration,
    hasher: Arc<dyn PasswordHasher>,
}

impl HashGate {
    /// Build a gate with `permits` permits and the wait limit `acquire_timeout`.
    #[must_use]
    pub fn new(permits: usize, acquire_timeout: Duration, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
            acquire_timeout,
            hasher,
        }
    }

    /// Wait for a permit, at most `argon2.acquire_timeout`.
    ///
    /// A use case calls this before it opens a connection (design D5).
    ///
    /// # Errors
    ///
    /// Returns `AppError::Timeout` with the stage `Argon2Permit` (`E-WEB-009`) when the limit
    /// elapses. Returns `AppError::TaskFailed` when the semaphore is closed, which never happens.
    pub async fn acquire(&self) -> Result<HashPermit, AppError> {
        let waiting = Arc::clone(&self.semaphore).acquire_owned();
        let permit = tokio::time::timeout(self.acquire_timeout, waiting)
            .await
            .map_err(|_elapsed| AppError::Timeout {
                stage: TimeoutStage::Argon2Permit,
            })?
            .map_err(|_closed| hash_task_failed())?;
        Ok(HashPermit {
            permit,
            hasher: Arc::clone(&self.hasher),
        })
    }

    /// Return the number of free permits.
    #[cfg(any(test, feature = "memory"))]
    #[must_use]
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

/// One permit to run Argon2 work.
///
/// The type solves one problem: the permit must outlive the hash, also when the caller drops its
/// future. Each method moves the permit into the `spawn_blocking` closure and gets it back with
/// the result. A hash that keeps running after the caller is gone therefore still holds its
/// permit. One permit serves several calls in a row, such as the check and the hash of
/// `ChangeOwnPassword`.
pub struct HashPermit {
    permit: OwnedSemaphorePermit,
    hasher: Arc<dyn PasswordHasher>,
}

impl HashPermit {
    /// Check a typed password in `spawn_blocking`. Return the answer and the permit.
    ///
    /// With `None`, the check runs against the dummy hash and answers `false`.
    ///
    /// # Errors
    ///
    /// Returns `AppError::TaskFailed` (`E-WEB-011`) when the blocking task panics.
    pub async fn verify(
        self,
        password: Password,
        hash: Option<PasswordHash>,
    ) -> Result<(bool, Self), AppError> {
        let Self { permit, hasher } = self;
        let joined = tokio::task::spawn_blocking(move || {
            let matched = hasher.verify(&password, hash.as_ref());
            (matched, permit, hasher)
        })
        .await;
        let (matched, permit, hasher) = joined.map_err(|_join| hash_task_failed())?;
        Ok((matched, Self { permit, hasher }))
    }

    /// Hash a new password in `spawn_blocking`. Return the hash and the permit.
    ///
    /// # Errors
    ///
    /// Returns `AppError::TaskFailed` (`E-WEB-011`) when the blocking task panics or when the
    /// hasher reports a `HashFailure`. The permit returns to the gate in both cases.
    pub async fn hash(self, password: NewPassword) -> Result<(PasswordHash, Self), AppError> {
        let Self { permit, hasher } = self;
        let joined = tokio::task::spawn_blocking(move || {
            let hashed = hasher.hash(&password);
            (hashed, permit, hasher)
        })
        .await;
        let (hashed, permit, hasher) = joined.map_err(|_join| hash_task_failed())?;
        Ok((hashed?, Self { permit, hasher }))
    }
}

#[cfg(test)]
mod tests;
