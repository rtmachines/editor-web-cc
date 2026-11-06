//! Adapter of the port `DiskSpace` (ADR-0060 (c), ADR-0073 (d), ADR-0133).
//!
//! On Unix the adapter calls `statvfs`. On another system it returns `DiskFault::Unsupported`
//! (ADR-0073 (e)). Only a development host on Windows has no `statvfs`.

use std::collections::HashMap;
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

/// The stalled paths with the number of marks on each (ADR-0133 (c), (f)).
///
/// Several calls on one path can reach the limit, and each of them adds one mark. A call takes
/// away only its own mark, so the path stays stalled while any marking call still runs.
type Stalled = Arc<Mutex<HashMap<PathBuf, usize>>>;

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
            stalled: Arc::new(Mutex::new(HashMap::new())),
            reader,
        }
    }

    fn is_stalled(&self, path: &Path) -> bool {
        self.stalled.lock().contains_key(path)
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

/// One call of the adapter: its path, its state `Running` or `Done`, and its own mark.
///
/// The type solves two problems. A call that ends at the limit must leave no mark. A call that
/// added no mark must not take the mark of another call away. The blocking task and the timeout
/// branch change the state, the flag `marked`, and the map under the one mutex of the map, so
/// only one order is possible (ADR-0133 (c), (e)).
struct Call {
    path: PathBuf,
    stalled: Stalled,
    done: AtomicBool,
    marked: AtomicBool,
}

impl Call {
    fn new(path: PathBuf, stalled: Stalled) -> Self {
        Self {
            path,
            stalled,
            done: AtomicBool::new(false),
            marked: AtomicBool::new(false),
        }
    }

    /// The blocking task ended: set `Done` and take away the mark of this call, if it added one.
    fn finish(&self) {
        let mut stalled = self.stalled.lock();
        self.done.store(true, Ordering::SeqCst);
        if !self.marked.load(Ordering::SeqCst) {
            return;
        }
        let Some(marks) = stalled.get_mut(&self.path) else {
            return;
        };
        *marks = marks.saturating_sub(1);
        if *marks == 0 {
            stalled.remove(&self.path);
        }
    }

    /// The call reached the limit: add one mark only while the state is `Running`.
    fn mark_if_running(&self) {
        let mut stalled = self.stalled.lock();
        if self.done.load(Ordering::SeqCst) || self.marked.swap(true, Ordering::SeqCst) {
            return;
        }
        let marks = stalled.entry(self.path.clone()).or_insert(0);
        *marks = marks.saturating_add(1);
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
