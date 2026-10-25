//! Test of the `ERROR` line of a failed run: it holds the `Display` chain (`reliability.md` 1.1).
//!
//! The test uses paused time, so it never waits for a real interval.

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

/// The text that only the source of the storage error holds.
const SOURCE_TEXT: &str = "distinctive-cause-of-the-storage-fault";

/// A job that fails with a storage error. `Display` of that error hides its source.
struct StorageFault;

impl JobRun for StorageFault {
    async fn run(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        Err(AppError::Storage(StorageError::Other {
            sqlstate: None,
            source: Box::new(io::Error::other(SOURCE_TEXT)),
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
        assert!(errors[0].contains(SOURCE_TEXT), "{errors:?}");
        assert!(errors[0].contains("\"job\":\"cleanup\""), "{errors:?}");
        assert!(
            errors[0].contains("\"error_code\":\"E-DB-008\""),
            "{errors:?}"
        );
        assert!(!errors[0].contains("request_id"), "{errors:?}");
    })
    .await;
}
