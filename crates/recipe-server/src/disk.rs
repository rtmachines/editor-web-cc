//! Adapter of the port `DiskSpace` (ADR-0060 (c), ADR-0073 (d), ADR-0133).
//!
//! On Unix the adapter calls `statvfs`. On another system it returns `DiskFault::Unsupported`
//! (ADR-0073 (e)). Only a development host on Windows has no `statvfs`.

use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_app::ports::{DiskFault, DiskSpace, FreeSpace};

/// The function that reads the free space of one path. A unit test gives a reader that blocks.
type Reader = Arc<dyn Fn(&Path) -> Result<FreeSpace, DiskFault> + Send + Sync>;

/// The paths whose call reached the limit and has not ended (ADR-0133 (c), (f)).
type Stalled = Arc<Mutex<HashSet<PathBuf>>>;

/// The free space of a file system, read with `statvfs`.
///
/// The type solves one problem: the job `disk_check` needs real measurements in the image and
/// must still build on Windows. The system call runs in a blocking task, because a stalled
/// mount can hold `statvfs` for a long time. The adapter waits at most `limit` for one call
/// (ADR-0133 (a)). A call that reaches the limit marks its path as stalled. While that call
/// still runs, a new call on the path gives `DiskFault::Unreadable` at once and starts no
/// second blocking task. Calls below the limit run side by side (ADR-0133 (c), (d)).
#[derive(Clone)]
pub struct SystemDiskSpace {
    limit: Duration,
    stalled: Stalled,
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
            stalled: Arc::new(Mutex::new(HashSet::new())),
            reader,
        }
    }

    fn is_stalled(&self, path: &Path) -> bool {
        self.stalled.lock().contains(path)
    }

    fn stalled_handle(&self) -> Stalled {
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

/// One call of the adapter: its path and its state, `Running` or `Done`.
///
/// The type solves one problem: a call that ends at the limit must leave no mark. The blocking
/// task and the timeout branch both change the state and the set under the one mutex of the
/// set, so only one order is possible (ADR-0133 (e)).
struct Call {
    path: PathBuf,
    stalled: Stalled,
    done: AtomicBool,
}

impl Call {
    fn new(path: PathBuf, stalled: Stalled) -> Self {
        Self {
            path,
            stalled,
            done: AtomicBool::new(false),
        }
    }

    /// The blocking task ended: set `Done` and take the path out of the set.
    fn finish(&self) {
        let mut stalled = self.stalled.lock();
        self.done.store(true, Ordering::SeqCst);
        stalled.remove(&self.path);
    }

    /// The call reached the limit: add the path only while the state is `Running`.
    fn mark_if_running(&self) {
        let mut stalled = self.stalled.lock();
        if !self.done.load(Ordering::SeqCst) {
            stalled.insert(self.path.clone());
        }
    }
}

/// Run `finish` of a call when the blocking task ends, also after a panic of the reader.
struct FinishOnDrop(Arc<Call>);

impl Drop for FinishOnDrop {
    fn drop(&mut self) {
        self.0.finish();
    }
}

#[async_trait]
impl DiskSpace for SystemDiskSpace {
    async fn free_space(&self, path: &Path) -> Result<FreeSpace, DiskFault> {
        if self.is_stalled(path) {
            return Err(timed_out("a read of the path stalled"));
        }
        let call = Arc::new(Call::new(path.to_path_buf(), self.stalled_handle()));
        let reader = Arc::clone(&self.reader);
        let task_call = Arc::clone(&call);
        let task = tokio::task::spawn_blocking(move || {
            let guard = FinishOnDrop(task_call);
            reader(&guard.0.path)
        });
        match tokio::time::timeout(self.limit, task).await {
            Ok(Ok(reading)) => reading,
            Ok(Err(join_fault)) => Err(DiskFault::Unreadable(io::Error::other(join_fault))),
            Err(_elapsed) => {
                call.mark_if_running();
                Err(timed_out("the read of the path took too long"))
            }
        }
    }
}

/// A fault of the kind `TimedOut` with a fixed text. The text names no path.
fn timed_out(text: &'static str) -> DiskFault {
    DiskFault::Unreadable(io::Error::new(io::ErrorKind::TimedOut, text))
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
