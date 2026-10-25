//! Tests of the backup directory adapter.
//!
//! Each test builds a directory of empty files in the temporary folder of the host. The time of
//! a set is the modification time of its manifest, so a test sets that time to whole seconds.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use recipe_app::ports::{BackupDirectory, BackupFault};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::{FileBackupDirectory, is_manifest_name};

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
        FileBackupDirectory::new(&self.0)
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

#[tokio::test]
async fn a_missing_directory_is_a_fault_and_not_an_empty_answer() {
    let sandbox = Sandbox::new();
    let missing = sandbox.0.join("absent");

    let fault = FileBackupDirectory::new(&missing)
        .newest_complete_set()
        .await
        .unwrap_err();

    assert!(matches!(fault, BackupFault::Unreadable(_)), "got {fault:?}");
}

#[tokio::test]
async fn a_file_in_place_of_the_directory_is_a_fault() {
    let sandbox = Sandbox::new();
    sandbox.file_at("not-a-directory", 1_000_000);

    let fault = FileBackupDirectory::new(sandbox.0.join("not-a-directory"))
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
        .build()
        .unwrap();
    let fault = runtime
        .block_on(FileBackupDirectory::new(missing).newest_complete_set())
        .unwrap_err();

    assert!(!fault.to_string().contains("secret-directory-name"));
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
