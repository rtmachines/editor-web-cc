//! The time limit of a blocking read on a volume (ADR-0133, ADR-0151 (o), (p), design D10).
//!
//! A stalled mount can hold a system call for a long time. The function `read_with_limit` runs
//! the call in a blocking task and waits at most `limit` for it. The adapters of the ports
//! `DiskSpace`, `BackupDirectory`, and `CheckResults` share this code. Each adapter owns its own
//! set of stalled paths and maps the `LimitFault` to the fault of its port.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;

/// The stalled paths with the number of marks on each (ADR-0133 (c), (f)).
///
/// Several calls on one path can reach the limit, and each of them adds one mark. A call takes
/// away only its own mark, so the path stays stalled while any marking call still runs.
/// A path holds at most one mark for each pair of a job and an adapter (ADR-0161).
pub(crate) type StalledPaths = Arc<Mutex<HashMap<PathBuf, usize>>>;

/// Return `true` while a call that reached the limit still runs on `path`.
pub(crate) fn is_stalled(stalled: &StalledPaths, path: &Path) -> bool {
    stalled.lock().contains_key(path)
}

/// The reason why `read_with_limit` gives no value.
///
/// The type solves one problem: each adapter must map the four reasons to the fault of its own
/// port. The type owns no error code, because the adapter turns it into a port fault at once.
#[derive(Debug)]
pub(crate) enum LimitFault {
    /// An earlier call on the path reached the limit and still runs. The new call started nothing.
    Stalled,
    /// The call reached the limit and marked the path.
    TimedOut,
    /// The blocking task ended without a value, for example after a panic of the reader.
    Join(tokio::task::JoinError),
    /// The call reached the limit, but the count of marks of the path is at its maximum.
    /// The call therefore keeps no mark.
    MarkOverflow,
}

impl LimitFault {
    /// Convert the fault into an I/O error. The kind of a limit fault is `TimedOut`.
    ///
    /// The text of the error names no path.
    pub(crate) fn into_io_error(self) -> io::Error {
        match self {
            Self::Stalled => timed_out("a read of the path stalled"),
            Self::TimedOut => timed_out("the read of the path took too long"),
            Self::Join(join_fault) => io::Error::other(join_fault),
            Self::MarkOverflow => {
                timed_out("the read of the path took too long, and the path has too many marks")
            }
        }
    }
}

/// An I/O error of the kind `TimedOut` with a fixed text.
fn timed_out(text: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, text)
}

/// One call of the function: its path, its state `Running` or `Done`, and its own mark.
///
/// The type solves two problems. A call that ends at the limit must leave no mark. A call that
/// added no mark must not take the mark of another call away. The blocking task and the timeout
/// branch change the state, the flag `marked`, and the map under the one mutex of the map, so
/// only one order is possible (ADR-0133 (c), (e)).
pub(crate) struct Call {
    path: PathBuf,
    stalled: StalledPaths,
    done: AtomicBool,
    marked: AtomicBool,
}

impl Call {
    /// Build a running call on `path` that has no mark.
    pub(crate) fn new(path: PathBuf, stalled: StalledPaths) -> Self {
        Self {
            path,
            stalled,
            done: AtomicBool::new(false),
            marked: AtomicBool::new(false),
        }
    }

    /// The blocking task ended: set `Done` and take away the mark of this call, if it added one.
    ///
    /// A marked call always has its mark in the count, so a count that cannot go down (`None`)
    /// cannot occur. The entry then goes away, like a count of zero. This function never
    /// panics, because it runs in `Drop`.
    pub(crate) fn finish(&self) {
        let mut stalled = self.stalled.lock();
        self.done.store(true, Ordering::SeqCst);
        if !self.marked.load(Ordering::SeqCst) {
            return;
        }
        let Some(marks) = stalled.get_mut(&self.path) else {
            return;
        };
        match marks.checked_sub(1) {
            Some(remaining) if remaining > 0 => *marks = remaining,
            Some(_) | None => {
                stalled.remove(&self.path);
            }
        }
    }

    /// The call reached the limit: add one mark only while the state is `Running`.
    ///
    /// A call that is done or already marked changes nothing and gives `Ok`.
    ///
    /// # Errors
    ///
    /// Gives `LimitFault::MarkOverflow` if the count cannot go up (`None`). The count stays and
    /// this call keeps no mark.
    pub(crate) fn mark_if_running(&self) -> Result<(), LimitFault> {
        let mut stalled = self.stalled.lock();
        if self.done.load(Ordering::SeqCst) || self.marked.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let marks = stalled.entry(self.path.clone()).or_insert(0);
        let Some(added) = marks.checked_add(1) else {
            self.marked.store(false, Ordering::SeqCst);
            return Err(LimitFault::MarkOverflow);
        };
        *marks = added;
        Ok(())
    }
}

/// Run `finish` of a call when the blocking task ends, also after a panic of the reader.
struct FinishOnDrop(Arc<Call>);

impl Drop for FinishOnDrop {
    fn drop(&mut self) {
        self.0.finish();
    }
}

/// Run `read` on `path` in a blocking task and wait at most `limit` for its value.
///
/// A path with a mark gives `LimitFault::Stalled` at once and starts no task. A call that reaches
/// the limit marks its path and gives `LimitFault::TimedOut`. If the count of marks is at its
/// maximum, the call gives `LimitFault::MarkOverflow`. The blocking task clears that mark
/// when it ends. Calls below the limit run side by side (ADR-0133 (c) to (e)).
///
/// A caller that drops this future before `limit` leaves its running call without a mark
/// (ADR-0172 (ai)). The operator needs no step for this case (ADR-0172 (aj)).
pub(crate) async fn read_with_limit<T, F>(
    limit: Duration,
    stalled: &StalledPaths,
    path: PathBuf,
    read: F,
) -> Result<T, LimitFault>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> T + Send + 'static,
{
    if is_stalled(stalled, &path) {
        return Err(LimitFault::Stalled);
    }
    let call = Arc::new(Call::new(path, Arc::clone(stalled)));
    let task_call = Arc::clone(&call);
    let task = tokio::task::spawn_blocking(move || {
        let guard = FinishOnDrop(task_call);
        read(&guard.0.path)
    });
    match tokio::time::timeout(limit, task).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(join_fault)) => Err(LimitFault::Join(join_fault)),
        Err(_elapsed) => {
            call.mark_if_running()?;
            Err(LimitFault::TimedOut)
        }
    }
}

#[cfg(test)]
mod tests;
