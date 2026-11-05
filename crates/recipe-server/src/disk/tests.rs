//! Tests of the free-space adapter.
//!
//! The Unix tests call `statvfs` on the temporary directory of the host. They do not run on
//! Windows. The CI job on Linux runs them. The Windows test checks the value `Unsupported`.
//! The tests of the time limit give the adapter a reader that blocks, so they run on every system.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use parking_lot::{Condvar, Mutex};
use recipe_app::ports::{DiskFault, DiskSpace, FreeSpace};

use super::{Call, SystemDiskSpace};

/// A limit that no test call reaches.
const LONG_LIMIT: Duration = Duration::from_secs(30);
/// A real limit of some milliseconds for the tests of a stalled reader.
const SHORT_LIMIT: Duration = Duration::from_millis(40);
/// The time between two checks of a background condition.
const POLL_STEP: Duration = Duration::from_millis(5);
/// The number of checks of a background condition: a wait of about ten seconds.
const POLL_ATTEMPTS: u32 = 2000;

/// A reader that counts its starts and blocks until the test opens the gate.
struct Probe {
    starts: AtomicUsize,
    open: Mutex<bool>,
    changed: Condvar,
}

impl Probe {
    fn blocked() -> Arc<Self> {
        Arc::new(Self {
            starts: AtomicUsize::new(0),
            open: Mutex::new(false),
            changed: Condvar::new(),
        })
    }

    /// Let every blocked and every later reader end.
    fn release(&self) {
        *self.open.lock() = true;
        self.changed.notify_all();
    }

    fn starts(&self) -> usize {
        self.starts.load(Ordering::SeqCst)
    }

    fn read(&self) -> Result<FreeSpace, DiskFault> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let mut open = self.open.lock();
        while !*open {
            self.changed.wait(&mut open);
        }
        Ok(FreeSpace {
            available_blocks: 1,
            total_blocks: 2,
        })
    }

    fn adapter(self: &Arc<Self>, limit: Duration) -> SystemDiskSpace {
        let probe = Arc::clone(self);
        SystemDiskSpace::with_reader(limit, Arc::new(move |_path| probe.read()))
    }
}

/// Wait until `condition` holds. The test fails after `POLL_ATTEMPTS` checks.
async fn wait_until(what: &str, condition: impl Fn() -> bool) {
    for _ in 0..POLL_ATTEMPTS {
        if condition() {
            return;
        }
        tokio::time::sleep(POLL_STEP).await;
    }
    assert!(condition(), "timed out: {what}");
}

fn is_timed_out(fault: &DiskFault) -> bool {
    matches!(fault, DiskFault::Unreadable(cause) if cause.kind() == io::ErrorKind::TimedOut)
}

fn data_path() -> PathBuf {
    PathBuf::from("data")
}

#[tokio::test]
async fn a_reader_that_blocks_gives_unreadable_after_the_limit() {
    let probe = Probe::blocked();
    let space = probe.adapter(SHORT_LIMIT);

    let fault = space.free_space(&data_path()).await.unwrap_err();
    probe.release();

    assert!(is_timed_out(&fault), "got {fault:?}");
    assert_eq!(probe.starts(), 1);
}

#[tokio::test]
async fn a_call_on_a_stalled_path_gives_unreadable_at_once_and_starts_no_reader() {
    let probe = Probe::blocked();
    let space = probe.adapter(SHORT_LIMIT);
    let first = space.free_space(&data_path()).await.unwrap_err();
    assert!(is_timed_out(&first), "got {first:?}");

    let started = tokio::time::Instant::now();
    let second = space.free_space(&data_path()).await.unwrap_err();
    let waited = started.elapsed();
    probe.release();

    assert!(matches!(second, DiskFault::Unreadable(_)), "got {second:?}");
    assert!(waited < SHORT_LIMIT, "the second call waited {waited:?}");
    assert_eq!(probe.starts(), 1, "the second call started a reader");
}

#[tokio::test]
async fn a_call_after_the_end_of_the_first_reader_runs_a_new_reader() {
    let probe = Probe::blocked();
    let space = probe.adapter(SHORT_LIMIT);
    let first = space.free_space(&data_path()).await.unwrap_err();
    assert!(is_timed_out(&first), "got {first:?}");

    probe.release();
    wait_until("the mark of the ended reader clears", || {
        !space.is_stalled(&data_path())
    })
    .await;
    let second = space.free_space(&data_path()).await;

    assert!(second.is_ok(), "got {second:?}");
    assert_eq!(probe.starts(), 2);
}

#[tokio::test]
async fn two_concurrent_fast_calls_of_one_path_both_give_a_reading() {
    let probe = Probe::blocked();
    let space = probe.adapter(LONG_LIMIT);
    let path = data_path();

    let (first, second, ()) =
        tokio::join!(space.free_space(&path), space.free_space(&path), async {
            wait_until("both readers started", || probe.starts() == 2).await;
            probe.release();
        });

    assert!(first.is_ok(), "got {first:?}");
    assert!(second.is_ok(), "got {second:?}");
    assert_eq!(probe.starts(), 2);
}

#[tokio::test]
async fn a_reader_that_ends_at_the_limit_leaves_no_mark() {
    let probe = Probe::blocked();
    probe.release();
    let space = probe.adapter(LONG_LIMIT);
    let call = Call::new(data_path(), space.stalled_handle());

    // The reader ends first. The timeout branch looks at the state afterwards.
    call.finish();
    call.mark_if_running();

    assert!(
        !space.is_stalled(&data_path()),
        "the ended call left a mark"
    );
    let next = space.free_space(&data_path()).await;
    assert!(next.is_ok(), "got {next:?}");
    assert_eq!(probe.starts(), 1);
}

#[tokio::test]
async fn a_running_call_that_reaches_the_limit_leaves_a_mark_until_it_ends() {
    let probe = Probe::blocked();
    let space = probe.adapter(LONG_LIMIT);
    let call = Call::new(data_path(), space.stalled_handle());

    call.mark_if_running();
    assert!(space.is_stalled(&data_path()));

    call.finish();
    assert!(!space.is_stalled(&data_path()));
}

#[cfg(unix)]
#[tokio::test]
async fn free_space_of_the_temporary_directory_is_consistent() {
    let space = SystemDiskSpace::new(LONG_LIMIT)
        .free_space(&std::env::temp_dir())
        .await
        .unwrap();

    assert!(space.total_blocks > 0, "a file system has blocks");
    assert!(space.available_blocks <= space.total_blocks);
}

#[cfg(unix)]
#[tokio::test]
async fn a_missing_path_is_unreadable() {
    let missing = std::env::temp_dir().join("recipe-server-disk-missing-path");

    let fault = SystemDiskSpace::new(LONG_LIMIT)
        .free_space(&missing)
        .await
        .unwrap_err();

    assert!(matches!(fault, DiskFault::Unreadable(_)), "got {fault:?}");
}

#[cfg(unix)]
#[tokio::test]
async fn the_text_of_an_unreadable_fault_names_no_path() {
    let marker = "recipe-server-disk-secret-name";
    let missing = std::env::temp_dir().join(marker);

    let fault = SystemDiskSpace::new(LONG_LIMIT)
        .free_space(&missing)
        .await
        .unwrap_err();

    // The `Display` text is a fixed sentence. The `Debug` text shows the wrapped error.
    // It shows the marker when the adapter puts the path into that error.
    assert!(matches!(fault, DiskFault::Unreadable(_)), "got {fault:?}");
    assert!(!fault.to_string().contains(marker));
    assert!(!format!("{fault:?}").contains(marker), "got {fault:?}");
}

#[cfg(not(unix))]
#[tokio::test]
async fn a_system_without_statvfs_gives_unsupported() {
    let fault = SystemDiskSpace::new(LONG_LIMIT)
        .free_space(&std::env::temp_dir())
        .await
        .unwrap_err();

    assert!(matches!(fault, DiskFault::Unsupported), "got {fault:?}");
}

#[cfg(not(unix))]
#[tokio::test]
async fn a_system_without_statvfs_gives_unsupported_also_for_a_missing_path() {
    let fault = SystemDiskSpace::new(LONG_LIMIT)
        .free_space(&std::env::temp_dir().join("recipe-server-disk-missing-path"))
        .await
        .unwrap_err();

    assert!(matches!(fault, DiskFault::Unsupported), "got {fault:?}");
}
