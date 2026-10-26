//! Test of the `ERROR` line of a failed run: it holds the `Display` chain (`reliability.md` 1.1).
//!
//! The test uses paused time, so it never waits for a real interval.

use std::fmt;
use std::io;
use std::sync::Arc;

use recipe_app::error::{AppError, StorageError};
use recipe_app::jobs::RunKind;
use recipe_app::ports::Clock;
use recipe_domain::audit::RequestId;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use super::support::{LogCapture, PausedClock, bounded, secs, settings};
use super::{JobName, JobRun, Supervisor};
use crate::schedule::Schedule;

/// The text that only the first source of the storage error holds.
const SOURCE_TEXT: &str = "distinctive-cause-of-the-storage-fault";
/// The text that only the source of that first source holds.
const ROOT_TEXT: &str = "distinctive-root-of-the-storage-fault";

/// The deepest error of the chain. It has no source.
#[derive(Debug)]
struct RootCause;

impl fmt::Display for RootCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(ROOT_TEXT)
    }
}

impl std::error::Error for RootCause {}

/// The error inside the `io::Error`. Its `Display` hides `RootCause`, which is its source.
#[derive(Debug)]
struct MiddleCause(RootCause);

impl fmt::Display for MiddleCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(SOURCE_TEXT)
    }
}

impl std::error::Error for MiddleCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// A job that fails with a storage error. `Display` of that error hides its source chain.
struct StorageFault;

impl JobRun for StorageFault {
    async fn run(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        Err(AppError::Storage(StorageError::Other {
            sqlstate: None,
            source: Box::new(io::Error::other(MiddleCause(RootCause))),
        }))
    }
}

#[tokio::test(start_paused = true)]
async fn the_error_line_of_a_run_holds_the_display_chain_with_the_source() {
    bounded(async {
        let logs = LogCapture::start();
        let clock: Arc<dyn Clock> = PausedClock::at("2026-10-24T00:00:00.000000Z");
        let stop = CancellationToken::new();
        let mut supervisor = Supervisor::new(settings(), clock, stop.clone());
        let schedule = Schedule::every(secs(60)).unwrap();
        supervisor.spawn(JobName::Cleanup, schedule, StorageFault);
        sleep(secs(90)).await;
        stop.cancel();
        assert!(supervisor.drain(secs(5)).await.is_complete());
        let errors: Vec<String> = logs
            .lines()
            .into_iter()
            .filter(|line| line.contains("\"level\":\"ERROR\""))
            .collect();
        assert_eq!(errors.len(), 1, "{errors:?}");
        // Both texts must appear, and the nearer cause comes first. A chain that stops at the
        // first source would lack `ROOT_TEXT`.
        let source_at = errors[0].find(SOURCE_TEXT);
        let root_at = errors[0].find(ROOT_TEXT);
        assert!(source_at.is_some(), "{errors:?}");
        assert!(root_at.is_some(), "{errors:?}");
        assert!(source_at < root_at, "{errors:?}");
        assert!(errors[0].contains("\"job\":\"cleanup\""), "{errors:?}");
        assert!(
            errors[0].contains("\"error_code\":\"E-DB-008\""),
            "{errors:?}"
        );
        assert!(!errors[0].contains("request_id"), "{errors:?}");
    })
    .await;
}
