//! Port of the free disk space (`reliability.md` section 7.8, ADR-0060 (c)).

use std::fmt;
use std::io;
use std::path::Path;

use async_trait::async_trait;

/// The free space of one file system, in blocks (`f_bavail` and `f_blocks` of `statvfs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FreeSpace {
    /// The blocks that an unprivileged process can use (`f_bavail`).
    pub available_blocks: u64,
    /// The blocks of the whole file system (`f_blocks`).
    pub total_blocks: u64,
}

/// Why the free space is not known.
///
/// This is a reason type. It owns no error code, like `ValueFaults` (ADR-0025, ADR-0073 (b)).
#[derive(Debug)]
pub enum DiskFault {
    /// The system has no call for the free space. The adapter on Windows returns it.
    Unsupported,
    /// The call failed for this path. On `pgdata`, the job `disk_check` then turns the read-only
    /// mode on (ADR-0073 (d)). The `Display` text does not show the cause.
    Unreadable(io::Error),
}

impl fmt::Display for DiskFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("The free disk space cannot be read on this system."),
            Self::Unreadable(_) => f.write_str("The free disk space cannot be read."),
        }
    }
}

/// Port of the free space of a file system.
///
/// The port solves one problem: the job `disk_check` must run on a test system and on a
/// system without `statvfs`. The method is `async`, so an adapter can move the system call
/// to a blocking task.
#[async_trait]
pub trait DiskSpace: Send + Sync {
    /// Read the free space of the file system that holds `path`.
    async fn free_space(&self, path: &Path) -> Result<FreeSpace, DiskFault>;
}
