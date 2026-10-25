//! Port of the backup directory (`reliability.md` section 3.7, ADR-0060 (f)).

use std::fmt;
use std::io;

use async_trait::async_trait;
use recipe_domain::timestamp::Timestamp;

/// Why the backup directory cannot be read.
///
/// This is a reason type. It owns no error code, like `ValueFaults` (ADR-0025, ADR-0073 (b)).
/// The text names no path.
#[derive(Debug)]
pub enum BackupFault {
    /// The directory or a manifest in it cannot be read.
    /// The `Display` text does not show the cause.
    Unreadable(io::Error),
}

impl fmt::Display for BackupFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(_) => f.write_str("The backup directory cannot be read."),
        }
    }
}

/// Port of the read-only mount of the `backups` volume.
///
/// The port solves one problem: an unreadable directory must not look like "no set". The first
/// case is a fault. It raises the alert of the dump age, also in the start grace (ADR-0073 (c)).
/// The second case is normal on a new installation.
#[async_trait]
pub trait BackupDirectory: Send + Sync {
    /// Return the time of the newest complete set, or `None` when no complete set exists.
    ///
    /// A set is complete when its manifest exists (`reliability.md` section 7.7).
    ///
    /// # Errors
    ///
    /// Returns [`BackupFault::Unreadable`] when the directory cannot be read.
    async fn newest_complete_set(&self) -> Result<Option<Timestamp>, BackupFault>;
}
