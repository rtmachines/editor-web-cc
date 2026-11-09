//! Tests of the backup directory adapter.
//!
//! Each test builds a directory of empty files in the temporary folder of the host. The time of
//! a set is the modification time of its manifest, so a test sets that time to whole seconds.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::{Condvar, Mutex};
use recipe_app::ports::{BackupDirectory, BackupFault};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::{FileBackupDirectory, Reader, is_manifest_name};

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// A temporary directory of one test. The drop removes it.
struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recipe-server-backups-{}-{number}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn adapter(&self) -> FileBackupDirectory {
        FileBackupDirectory::new(&self.0, LONG_LIMIT)
    }

    /// Create an empty file with the modification time `seconds` after the Unix epoch.
    fn file_at(&self, name: &str, seconds: u64) {
        let file = File::create(self.0.join(name)).unwrap();
        let moment = UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap();
        file.set_modified(moment).unwrap();
    }

    /// Create the four files of one set. The manifest gets the modification time.
    fn complete_set(&self, stem: &str, seconds: u64) {
        for extension in [".dump", ".counts"] {
            self.file_at(&format!("{stem}{extension}"), seconds);
        }
        self.file_at(
            &format!("globals_{}.sql", &stem["recipe_".len()..]),
            seconds,
        );
        self.file_at(&format!("{stem}.dump.sha256"), seconds);
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // A leftover directory in the temporary folder does no harm.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn timestamp(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

#[tokio::test]
async fn an_empty_directory_has_no_set() {
    let sandbox = Sandbox::new();

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, None);
}

#[tokio::test]
async fn a_complete_set_gives_the_time_of_its_manifest() {
    let sandbox = Sandbox::new();
    sandbox.complete_set("recipe_20261023_0200", 1_000_000);

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, Some(timestamp(1_000_000)));
}

#[tokio::test]
async fn the_newest_of_two_sets_wins() {
    let sandbox = Sandbox::new();
    sandbox.complete_set("recipe_20261022_0200", 1_000_000);
    sandbox.complete_set("recipe_20261023_0200", 1_086_400);

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, Some(timestamp(1_086_400)));
}

#[tokio::test]
async fn the_newest_set_is_the_one_with_the_newest_manifest_time_and_not_the_largest_name() {
    // The middle name has the newest time. The largest name has an older time than it.
    // So neither "the largest name" nor "the last listed entry" gives the right answer.
    let sandbox = Sandbox::new();
    sandbox.complete_set("recipe_20261021_0200", 1_000_000);
    sandbox.complete_set("recipe_20261022_0200", 1_300_000);
    sandbox.complete_set("recipe_20261023_0200", 1_100_000);

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, Some(timestamp(1_300_000)));
}

#[tokio::test]
async fn the_suffixes_premigrate_and_now_count_as_sets() {
    let sandbox = Sandbox::new();
    sandbox.complete_set("recipe_20261022_0200", 1_000_000);
    sandbox.complete_set("recipe_20261023_0930_premigrate", 1_100_000);
    sandbox.complete_set("recipe_20261023_1015_now", 1_200_000);

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, Some(timestamp(1_200_000)));
}

#[tokio::test]
async fn a_set_without_a_manifest_is_not_a_set() {
    let sandbox = Sandbox::new();
    sandbox.file_at("recipe_20261023_0200.dump", 1_000_000);
    sandbox.file_at("globals_20261023_0200.sql", 1_000_000);
    sandbox.file_at("recipe_20261023_0200.counts", 1_000_000);

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, None);
}

#[tokio::test]
async fn a_temp_manifest_is_not_a_set() {
    let sandbox = Sandbox::new();
    sandbox.file_at(".tmp-recipe_20261023_0200.dump.sha256", 1_000_000);

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, None);
}

#[tokio::test]
async fn a_temp_manifest_does_not_hide_an_older_complete_set() {
    let sandbox = Sandbox::new();
    sandbox.complete_set("recipe_20261022_0200", 1_000_000);
    sandbox.file_at(".tmp-recipe_20261023_0200.dump.sha256", 1_086_400);

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, Some(timestamp(1_000_000)));
}

#[tokio::test]
async fn a_manifest_alone_makes_a_set_complete() {
    // ADR-0074 (c): a set is complete when its manifest exists. The adapter checks no other file.
    let sandbox = Sandbox::new();
    sandbox.file_at("recipe_20261023_0200.dump.sha256", 1_000_000);

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, Some(timestamp(1_000_000)));
}

#[tokio::test]
async fn a_directory_with_the_name_of_a_manifest_is_not_a_set() {
    let sandbox = Sandbox::new();
    fs::create_dir(sandbox.0.join("recipe_20261023_0200.dump.sha256")).unwrap();

    let newest = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(newest, None);
}

#[cfg(unix)]
#[tokio::test]
async fn a_link_with_the_name_of_a_manifest_is_not_a_set() {
    use std::os::unix::fs::symlink;

    let sandbox = Sandbox::new();
    // The target is a real manifest in a sub-directory. The adapter does not scan sub-directories.
    fs::create_dir(sandbox.0.join("elsewhere")).unwrap();
    sandbox.file_at("elsewhere/recipe_20261023_0200.dump.sha256", 2_000_000);
    symlink(
        sandbox.0.join("elsewhere/recipe_20261023_0200.dump.sha256"),
        sandbox.0.join("recipe_20261023_0200.dump.sha256"),
    )
    .unwrap();

    let only_link = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(only_link, None);

    // A link does not hide an older real set and does not give its own time.
    sandbox.complete_set("recipe_20261022_0200", 1_000_000);

    let with_real_set = sandbox.adapter().newest_complete_set().await.unwrap();

    assert_eq!(with_real_set, Some(timestamp(1_000_000)));
}

#[tokio::test]
async fn a_missing_directory_is_a_fault_and_not_an_empty_answer() {
    let sandbox = Sandbox::new();
    let missing = sandbox.0.join("absent");

    let fault = FileBackupDirectory::new(&missing, LONG_LIMIT)
        .newest_complete_set()
        .await
        .unwrap_err();

    assert!(matches!(fault, BackupFault::Unreadable(_)), "got {fault:?}");
}

#[tokio::test]
async fn a_file_in_place_of_the_directory_is_a_fault() {
    let sandbox = Sandbox::new();
    sandbox.file_at("not-a-directory", 1_000_000);

    let fault = FileBackupDirectory::new(sandbox.0.join("not-a-directory"), LONG_LIMIT)
        .newest_complete_set()
        .await
        .unwrap_err();

    assert!(matches!(fault, BackupFault::Unreadable(_)), "got {fault:?}");
}

#[cfg(unix)]
#[tokio::test]
async fn a_directory_without_the_read_right_is_a_fault() {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new();
    sandbox.complete_set("recipe_20261023_0200", 1_000_000);
    fs::set_permissions(&sandbox.0, fs::Permissions::from_mode(0o000)).unwrap();
    // A process that runs as root reads the directory anyway, so the test proves nothing there.
    let root_reads_it = fs::read_dir(&sandbox.0).is_ok();

    let result = sandbox.adapter().newest_complete_set().await;

    fs::set_permissions(&sandbox.0, fs::Permissions::from_mode(0o700)).unwrap();
    if !root_reads_it {
        assert!(
            matches!(result, Err(BackupFault::Unreadable(_))),
            "got {result:?}"
        );
    }
}

#[test]
fn the_fault_text_names_no_path() {
    let sandbox = Sandbox::new();
    let missing: &Path = &sandbox.0.join("secret-directory-name");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let fault = runtime
        .block_on(FileBackupDirectory::new(missing, LONG_LIMIT).newest_complete_set())
        .unwrap_err();

    // The `Display` text is a fixed sentence. The `Debug` text shows the wrapped error.
    // It shows the marker when the adapter puts the path into that error.
    assert!(matches!(fault, BackupFault::Unreadable(_)), "got {fault:?}");
    assert!(!fault.to_string().contains("secret-directory-name"));
    assert!(
        !format!("{fault:?}").contains("secret-directory-name"),
        "got {fault:?}"
    );
}

#[test]
fn the_grammar_accepts_the_names_of_a_manifest_only() {
    let accepted = [
        "recipe_20261023_0200.dump.sha256",
        "recipe_20261023_0200_premigrate.dump.sha256",
        "recipe_20261023_0200_now.dump.sha256",
    ];
    let refused = [
        "",
        ".tmp-recipe_20261023_0200.dump.sha256",
        "xrecipe_20261023_0200.dump.sha256",
        "recipe_20261023_0200.dump.sha256.bak",
        "recipe_20261023_0200.dump.sha256\n",
        "recipe_20261023_0200.dump",
        "recipe_20261023_0200.counts",
        "globals_20261023_0200.sql",
        "recipe_2026102_0200.dump.sha256",
        "recipe_202610231_0200.dump.sha256",
        "recipe_20261023_020.dump.sha256",
        "recipe_20261023_02000.dump.sha256",
        "recipe_20261023-0200.dump.sha256",
        "recipe_2026102\u{ff13}_0200.dump.sha256",
        "recipe_20261023_0200_daily.dump.sha256",
        "recipe_20261023_0200_premigrate_now.dump.sha256",
        "recipe_20261023_0200__now.dump.sha256",
        "Recipe_20261023_0200.dump.sha256",
        "recipe_20261023_0200.DUMP.sha256",
    ];
    for name in accepted {
        assert!(is_manifest_name(name), "{name:?} must match");
    }
    for name in refused {
        assert!(!is_manifest_name(name), "{name:?} must not match");
    }
}

#[test]
fn the_time_of_a_set_keeps_the_nanoseconds_of_the_file_system() {
    // The conversion must not lose the sub-second part that some file systems store.
    let moment: SystemTime = UNIX_EPOCH
        .checked_add(Duration::new(1_000_000, 500_000_000))
        .unwrap();

    let converted = super::timestamp_of(moment).unwrap();

    let expected = OffsetDateTime::from_unix_timestamp_nanos(1_000_000_500_000_000).unwrap();
    assert_eq!(converted, Timestamp::from(expected));
}

// Tests of the time limit of the scan (ADR-0151 (o), design D10). The hook `with_reader`
// gives the adapter a reader that blocks, as `SystemDiskSpace::with_reader` does.

/// A limit that no test call reaches.
const LONG_LIMIT: Duration = Duration::from_secs(30);
/// A real limit of some milliseconds for the tests of a stalled reader.
const SHORT_LIMIT: Duration = Duration::from_millis(50);
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

    fn read(&self) -> io::Result<Option<Timestamp>> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let mut open = self.open.lock();
        while !*open {
            self.changed.wait(&mut open);
        }
        Ok(Some(timestamp(1_000_000)))
    }

    fn adapter(self: &Arc<Self>, limit: Duration) -> FileBackupDirectory {
        let probe = Arc::clone(self);
        let reader: Reader = Arc::new(move |_path| probe.read());
        FileBackupDirectory::with_reader(PathBuf::from("backups"), limit, reader)
    }
}

fn is_timed_out(fault: &BackupFault) -> bool {
    matches!(fault, BackupFault::Unreadable(cause) if cause.kind() == io::ErrorKind::TimedOut)
}

#[tokio::test]
async fn a_reader_that_blocks_gives_unreadable_after_the_limit() {
    let probe = Probe::blocked();
    let backups = probe.adapter(SHORT_LIMIT);

    let fault = backups.newest_complete_set().await.unwrap_err();
    probe.release();

    assert!(is_timed_out(&fault), "got {fault:?}");
    assert_eq!(probe.starts(), 1);
}

#[tokio::test]
async fn a_call_while_the_blocked_scan_runs_gives_unreadable_at_once_and_starts_no_reader() {
    let probe = Probe::blocked();
    let backups = probe.adapter(SHORT_LIMIT);
    let first = backups.newest_complete_set().await.unwrap_err();
    assert!(is_timed_out(&first), "got {first:?}");

    let started = tokio::time::Instant::now();
    let second = backups.newest_complete_set().await.unwrap_err();
    let waited = started.elapsed();
    probe.release();

    assert!(
        matches!(second, BackupFault::Unreadable(_)),
        "got {second:?}"
    );
    assert!(waited < SHORT_LIMIT, "the second call waited {waited:?}");
    assert_eq!(probe.starts(), 1, "the second call started a reader");
}

#[tokio::test]
async fn a_call_after_the_end_of_the_blocked_scan_runs_the_reader_again() {
    let probe = Probe::blocked();
    let backups = probe.adapter(SHORT_LIMIT);
    let first = backups.newest_complete_set().await.unwrap_err();
    assert!(is_timed_out(&first), "got {first:?}");

    probe.release();
    // The mark of the ended reader clears a moment after the reader ends. Until then, a call
    // gives `Unreadable` and starts no reader. The first call that succeeds is the new run.
    let mut second = backups.newest_complete_set().await;
    for _ in 0..POLL_ATTEMPTS {
        if second.is_ok() {
            break;
        }
        tokio::time::sleep(POLL_STEP).await;
        second = backups.newest_complete_set().await;
    }

    assert_eq!(second.unwrap(), Some(timestamp(1_000_000)));
    assert_eq!(probe.starts(), 2);
}
