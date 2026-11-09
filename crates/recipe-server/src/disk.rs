//! Adapter of the port `DiskSpace` (ADR-0060 (c), ADR-0073 (d), ADR-0133).
//!
//! On Unix the adapter calls `statvfs`. On another system it returns `DiskFault::Unsupported`
//! (ADR-0073 (e)). Only a development host on Windows has no `statvfs`.

use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use recipe_app::ports::{DiskFault, DiskSpace, FreeSpace};

#[cfg(test)]
use crate::limited_read::{Call, is_stalled};
use crate::limited_read::{StalledPaths, read_with_limit};

/// The function that reads the free space of one path. A unit test gives a reader that blocks.
type Reader = Arc<dyn Fn(&Path) -> Result<FreeSpace, DiskFault> + Send + Sync>;

/// The free space of a file system, read with `statvfs`.
///
/// The type solves one problem: the job `disk_check` needs real measurements in the image and
/// must still build on Windows. The system call runs in a blocking task, because a stalled
/// mount can hold `statvfs` for a long time. The adapter waits at most `limit` for one call
/// (ADR-0133 (a)). A call that reaches the limit marks its path as stalled. While that call
/// still runs, a new call on the path gives `DiskFault::Unreadable` at once and starts no
/// second blocking task. Calls below the limit run side by side (ADR-0133 (c), (d)).
/// The module `limited_read` holds this code.
#[derive(Clone)]
pub struct SystemDiskSpace {
    limit: Duration,
    stalled: StalledPaths,
    reader: Reader,
}

impl SystemDiskSpace {
    /// Build the adapter. `limit` is the setting `health.disk_check_interval` (ADR-0133 (a)).
    #[must_use]
    pub fn new(limit: Duration) -> Self {
        Self::with_reader(limit, Arc::new(read_free_space))
    }

    /// Build the adapter with another reader. The unit tests use it.
    fn with_reader(limit: Duration, reader: Reader) -> Self {
        Self {
            limit,
            stalled: StalledPaths::default(),
            reader,
        }
    }

    #[cfg(test)]
    fn is_stalled(&self, path: &Path) -> bool {
        is_stalled(&self.stalled, path)
    }

    #[cfg(test)]
    fn stalled_handle(&self) -> StalledPaths {
        Arc::clone(&self.stalled)
    }
}

impl fmt::Debug for SystemDiskSpace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SystemDiskSpace")
            .field("limit", &self.limit)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl DiskSpace for SystemDiskSpace {
    async fn free_space(&self, path: &Path) -> Result<FreeSpace, DiskFault> {
        let reader = Arc::clone(&self.reader);
        let reading = read_with_limit(self.limit, &self.stalled, path.to_path_buf(), move |path| {
            reader(path)
        })
        .await;
        match reading {
            Ok(reading) => reading,
            Err(fault) => Err(DiskFault::Unreadable(fault.into_io_error())),
        }
    }
}

/// Read `f_bavail` and `f_blocks` of the file system that holds `path`.
#[cfg(unix)]
fn read_free_space(path: &Path) -> Result<FreeSpace, DiskFault> {
    let stat = rustix::fs::statvfs(path).map_err(|errno| DiskFault::Unreadable(errno.into()))?;
    Ok(FreeSpace {
        available_blocks: stat.f_bavail,
        total_blocks: stat.f_blocks,
    })
}

/// A system without `statvfs` has no measurement.
#[cfg(not(unix))]
fn read_free_space(_path: &Path) -> Result<FreeSpace, DiskFault> {
    Err(DiskFault::Unsupported)
}

#[cfg(test)]
mod tests;
