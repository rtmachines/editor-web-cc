//! Adapter of the port `DiskSpace` (ADR-0060 (c), ADR-0073 (d)).
//!
//! On Unix the adapter calls `statvfs`. On another system it returns `DiskFault::Unsupported`
//! (ADR-0073 (e)). Only a development host on Windows has no `statvfs`.

use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use recipe_app::ports::{DiskFault, DiskSpace, FreeSpace};

/// The free space of a file system, read with `statvfs`.
///
/// The type solves one problem: the job `disk_check` needs real measurements in the image and
/// must still build on Windows. The system call runs in a blocking task, because a stalled
/// mount can hold `statvfs` for a long time.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemDiskSpace;

#[async_trait]
impl DiskSpace for SystemDiskSpace {
    async fn free_space(&self, path: &Path) -> Result<FreeSpace, DiskFault> {
        let path: PathBuf = path.to_path_buf();
        match tokio::task::spawn_blocking(move || read_free_space(&path)).await {
            Ok(reading) => reading,
            Err(join_fault) => Err(DiskFault::Unreadable(io::Error::other(join_fault))),
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
