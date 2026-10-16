//! Blocking work and settings (design D8, `reliability.md` sections 2.2, 4.4, and 7.4).

use std::future::Future;
use std::time::Duration;

use recipe_domain::recipe::ApprovalPolicy;

use crate::error::{AppError, TaskStage, TimeoutStage};

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
/// rebuild in `spawn_blocking`. `export` limits a future with `timeout.export`, which includes
/// that rebuild. A panic in a closure becomes `E-WEB-011`, and an elapsed limit becomes
/// `E-WEB-009`.
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

    /// Run `work` on a blocking thread and wait for it without a limit of its own.
    ///
    /// The future of `export` limits the wait, because `timeout.export` covers the rebuild.
    ///
    /// # Errors
    ///
    /// Returns `AppError::TaskFailed` with the stage `Export` when `work` panics.
    pub async fn rebuild_export<T, F>(&self, work: F) -> Result<T, AppError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        joined_or_failed(tokio::task::spawn_blocking(work).await, TaskStage::Export)
    }

    /// Run `future` and wait at most `timeout.export`.
    ///
    /// The future covers the read, the locks, the rebuild in `rebuild_export`, and the hash
    /// compare. The commit of the audit entry runs after this limit (ADR-0029).
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
}

#[cfg(test)]
mod tests;
