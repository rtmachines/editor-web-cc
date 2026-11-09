//! Port of the result files of the weekly backup checks (ADR-0151 (j3), (r), design D5).

use std::error::Error;
use std::fmt;
use std::io;

use async_trait::async_trait;
use recipe_domain::timestamp::Timestamp;

/// A check that runs each week and leaves a result file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScheduledCheck {
    /// The restore test of the newest backup set.
    RestoreTest,
    /// The base backup of the WAL archive.
    BaseBackup,
}

/// How a run of a scheduled check ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    /// The run ended with the status 0.
    Passed,
    /// The run ended with any other status.
    Failed,
}

/// The newest result of one scheduled check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckResult {
    /// How the run ended.
    pub outcome: CheckOutcome,
    /// The time at which the run ended.
    pub finished_at: Timestamp,
}

impl fmt::Display for ScheduledCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RestoreTest => "restore test",
            Self::BaseBackup => "base backup",
        })
    }
}

/// Why the result of a scheduled check cannot be read.
///
/// This is a reason type. It owns no error code, like `BackupFault` (ADR-0025, ADR-0073 (b),
/// ADR-0151 (j3)). Each variant names its check, so a log line of the fault tells the restore
/// test from the base backup. The text names the check and no path.
#[derive(Debug)]
pub enum CheckResultFault {
    /// The result file exists and cannot be read.
    /// The `Display` text does not show the cause. `Error::source` returns it.
    Unreadable {
        /// The check whose result file failed.
        check: ScheduledCheck,
        /// The cause of the failed read.
        source: io::Error,
    },
    /// The result file can be read and does not hold a valid result.
    Malformed {
        /// The check whose result file is malformed.
        check: ScheduledCheck,
    },
}

impl fmt::Display for CheckResultFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { check, .. } => {
                write!(f, "The result file of the {check} cannot be read.")
            }
            Self::Malformed { check } => write!(f, "The result file of the {check} is malformed."),
        }
    }
}

impl Error for CheckResultFault {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Unreadable { source, .. } => Some(source),
            Self::Malformed { .. } => None,
        }
    }
}

/// Port of the read-only mount that holds the result files of the scheduled checks.
///
/// The port solves one problem: a missing result file, a failed run, and an unreadable file need
/// three different alert rules. `None` means that the check never ran. The two other cases are
/// not the same (ADR-0151 (i), (j1), (j2)).
#[async_trait]
pub trait CheckResults: Send + Sync {
    /// Return the newest result of `check`, or `None` when no result file exists.
    ///
    /// # Errors
    ///
    /// Returns [`CheckResultFault::Unreadable`] when the file cannot be read. Returns
    /// [`CheckResultFault::Malformed`] when the file does not hold a valid result.
    async fn newest(&self, check: ScheduledCheck) -> Result<Option<CheckResult>, CheckResultFault>;
}
