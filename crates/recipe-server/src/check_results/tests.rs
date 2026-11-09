//! Tests of the result file reader (ADR-0151 (b), (b1), (c), (o), design D4 and D10).
//!
//! Each test builds two directories in the temporary folder of the host. The directory
//! `backups` holds the file of the restore test. The directory `wal` holds the file of the base
//! backup.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::Duration;

use parking_lot::{Condvar, Mutex};
use recipe_app::ports::{
    CheckOutcome, CheckResult, CheckResultFault, CheckResults, ScheduledCheck,
};
use recipe_domain::timestamp::Timestamp;
use time::macros::datetime;

use super::{FileCheckResults, MAX_RESULT_BYTES, Reader, Reading};

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// A limit that no test call reaches.
const LONG_LIMIT: Duration = Duration::from_secs(30);
/// A real limit of some milliseconds for the tests of a stalled reader.
const SHORT_LIMIT: Duration = Duration::from_millis(50);
/// The time between two checks of a background condition.
const POLL_STEP: Duration = Duration::from_millis(5);
/// The number of checks of a background condition: a wait of about ten seconds.
const POLL_ATTEMPTS: u32 = 2000;
/// A valid result line of the script `check-result.sh`, with the end of the line.
const PASSED_LINE: &str = "{\"result\":\"passed\",\"finished_at\":\"2026-11-08T05:41:07Z\"}\n";

/// The time in `PASSED_LINE`.
fn finished_at() -> Timestamp {
    Timestamp::from(datetime!(2026-11-08 05:41:07 UTC))
}

/// A result line with the given result word and time text.
fn line(result: &str, time: &str) -> String {
    format!("{{\"result\":\"{result}\",\"finished_at\":\"{time}\"}}\n")
}

/// Two temporary directories of one test. The drop removes them.
struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recipe-server-check-results-{}-{number}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("backups")).unwrap();
        fs::create_dir_all(path.join("wal")).unwrap();
        Self(path)
    }

    fn adapter(&self) -> FileCheckResults {
        FileCheckResults::new(self.0.join("backups"), self.0.join("wal"), LONG_LIMIT)
    }

    /// The path of the result file of `check`.
    fn file(&self, check: ScheduledCheck) -> PathBuf {
        match check {
            ScheduledCheck::RestoreTest => self.0.join("backups").join("restore-test.result"),
            ScheduledCheck::BaseBackup => self.0.join("wal").join("base-backup.result"),
        }
    }

    fn write(&self, check: ScheduledCheck, content: impl AsRef<[u8]>) {
        fs::write(self.file(check), content).unwrap();
    }

    /// Read the restore test with `content` in its file.
    async fn read_restore_test(
        &self,
        content: impl AsRef<[u8]>,
    ) -> Result<Option<CheckResult>, CheckResultFault> {
        self.write(ScheduledCheck::RestoreTest, content);
        self.adapter().newest(ScheduledCheck::RestoreTest).await
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // A test that sets a mode of 000 restores it before the end. The error stays unseen.
        drop(fs::remove_dir_all(&self.0));
    }
}

/// Assert that a file with `content` gives `Malformed` for the restore test.
async fn assert_malformed(content: impl AsRef<[u8]>) {
    let shown = String::from_utf8_lossy(content.as_ref()).into_owned();
    let sandbox = Sandbox::new();

    let answer = sandbox.read_restore_test(content).await;

    assert!(
        matches!(
            answer,
            Err(CheckResultFault::Malformed {
                check: ScheduledCheck::RestoreTest
            })
        ),
        "content {shown:?} gave {answer:?}"
    );
}

#[tokio::test]
async fn a_file_with_passed_and_a_valid_time_gives_a_passed_result() {
    let sandbox = Sandbox::new();

    let answer = sandbox.read_restore_test(PASSED_LINE).await.unwrap();

    assert_eq!(
        answer,
        Some(CheckResult {
            outcome: CheckOutcome::Passed,
            finished_at: finished_at(),
        })
    );
}

#[tokio::test]
async fn a_file_with_failed_gives_a_failed_result() {
    let sandbox = Sandbox::new();

    let answer = sandbox
        .read_restore_test(line("failed", "2026-11-08T05:41:07Z"))
        .await
        .unwrap();

    assert_eq!(
        answer,
        Some(CheckResult {
            outcome: CheckOutcome::Failed,
            finished_at: finished_at(),
        })
    );
}

#[tokio::test]
async fn a_file_without_the_end_of_line_gives_a_result() {
    let sandbox = Sandbox::new();

    let answer = sandbox
        .read_restore_test(PASSED_LINE.trim_end())
        .await
        .unwrap();

    assert_eq!(
        answer.map(|result| result.outcome),
        Some(CheckOutcome::Passed)
    );
}

#[tokio::test]
async fn a_missing_file_gives_none_for_both_checks() {
    let sandbox = Sandbox::new();
    let adapter = sandbox.adapter();

    let restore_test = adapter.newest(ScheduledCheck::RestoreTest).await.unwrap();
    let base_backup = adapter.newest(ScheduledCheck::BaseBackup).await.unwrap();

    assert_eq!(restore_test, None);
    assert_eq!(base_backup, None);
}

#[tokio::test]
async fn a_missing_directory_gives_none() {
    let sandbox = Sandbox::new();
    let adapter = FileCheckResults::new(
        sandbox.0.join("absent-backups"),
        sandbox.0.join("absent-wal"),
        LONG_LIMIT,
    );

    let answer = adapter.newest(ScheduledCheck::RestoreTest).await.unwrap();

    assert_eq!(answer, None);
}

#[tokio::test]
async fn each_check_reads_its_own_file() {
    let sandbox = Sandbox::new();
    sandbox.write(ScheduledCheck::RestoreTest, PASSED_LINE);
    sandbox.write(
        ScheduledCheck::BaseBackup,
        line("failed", "2026-11-01T01:02:03Z"),
    );
    let adapter = sandbox.adapter();

    let restore_test = adapter
        .newest(ScheduledCheck::RestoreTest)
        .await
        .unwrap()
        .unwrap();
    let base_backup = adapter
        .newest(ScheduledCheck::BaseBackup)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(restore_test.outcome, CheckOutcome::Passed);
    assert_eq!(restore_test.finished_at, finished_at());
    assert_eq!(base_backup.outcome, CheckOutcome::Failed);
    assert_eq!(
        base_backup.finished_at,
        Timestamp::from(datetime!(2026-11-01 01:02:03 UTC))
    );
}

#[tokio::test]
async fn an_unknown_key_is_malformed() {
    assert_malformed(
        "{\"result\":\"passed\",\"finished_at\":\"2026-11-08T05:41:07Z\",\"set\":\"x\"}\n",
    )
    .await;
}

#[tokio::test]
async fn a_missing_key_is_malformed() {
    assert_malformed("{\"result\":\"passed\"}\n").await;
    assert_malformed("{\"finished_at\":\"2026-11-08T05:41:07Z\"}\n").await;
    assert_malformed("{}\n").await;
}

#[tokio::test]
async fn a_repeated_key_is_malformed() {
    assert_malformed(
        "{\"result\":\"passed\",\"result\":\"failed\",\"finished_at\":\"2026-11-08T05:41:07Z\"}\n",
    )
    .await;
}

#[tokio::test]
async fn a_result_word_other_than_passed_or_failed_is_malformed() {
    for word in ["ok", "OK", "Passed", "FAILED", "pass", "", "passed "] {
        assert_malformed(line(word, "2026-11-08T05:41:07Z")).await;
    }
}

#[tokio::test]
async fn a_value_that_is_not_text_is_malformed() {
    assert_malformed("{\"result\":true,\"finished_at\":\"2026-11-08T05:41:07Z\"}\n").await;
    assert_malformed("{\"result\":\"passed\",\"finished_at\":1794030067}\n").await;
    assert_malformed("{\"result\":\"passed\",\"finished_at\":null}\n").await;
}

#[tokio::test]
async fn a_time_in_another_form_is_malformed() {
    let times = [
        // No `Z`.
        "2026-11-08T05:41:07",
        // An offset in place of the `Z`.
        "2026-11-08T05:41:07+00:00",
        "2026-11-08T07:41:07+02:00",
        "2026-11-08T05:41:07-00:00",
        // A lowercase `t` or `z`.
        "2026-11-08t05:41:07Z",
        "2026-11-08T05:41:07z",
        // A fraction of a second.
        "2026-11-08T05:41:07.5Z",
        "2026-11-08T05:41:07.000Z",
        "2026-11-08T05:41:07,5Z",
        // A space in place of the `T`.
        "2026-11-08 05:41:07Z",
        // Other forms and values.
        "2026-11-08T05:41Z",
        "2026-11-08Z",
        "20261108T054107Z",
        "2026-1-8T5:41:07Z",
        "2026-13-08T05:41:07Z",
        "2026-11-31T05:41:07Z",
        "2026-11-08T24:00:00Z",
        "2026-11-08T05:60:07Z",
        "2026-11-08T05:41:60Z",
        "+2026-11-08T05:41:07Z",
        " 2026-11-08T05:41:07Z",
        "2026-11-08T05:41:07Z ",
        "2026-11-08T05:41:07ZZ",
        "",
        "now",
    ];
    for time in times {
        assert_malformed(line("passed", time)).await;
    }
}

#[tokio::test]
async fn a_second_line_is_malformed() {
    assert_malformed(format!("{PASSED_LINE}{PASSED_LINE}")).await;
    assert_malformed(format!("{PASSED_LINE}\n")).await;
    assert_malformed(format!("\n{PASSED_LINE}")).await;
    assert_malformed("{\"result\":\"passed\",\n\"finished_at\":\"2026-11-08T05:41:07Z\"}\n").await;
}

#[tokio::test]
async fn a_line_end_with_a_carriage_return_is_malformed() {
    assert_malformed(PASSED_LINE.replace('\n', "\r\n")).await;
}

#[tokio::test]
async fn an_empty_file_or_text_that_is_not_json_is_malformed() {
    assert_malformed("").await;
    assert_malformed("\n").await;
    assert_malformed("passed").await;
    assert_malformed("[\"passed\",\"2026-11-08T05:41:07Z\"]\n").await;
    assert_malformed(format!("{}x", PASSED_LINE.trim_end())).await;
    assert_malformed(&PASSED_LINE.as_bytes()[..20]).await;
}

#[tokio::test]
async fn a_file_that_is_not_utf8_is_malformed() {
    assert_malformed(b"{\"result\":\"passed\",\"finished_at\":\"\xff\"}\n").await;
}

/// A valid line that carries `padding` spaces before the closing brace.
fn padded_line(padding: usize) -> String {
    let body = PASSED_LINE.trim_end().trim_end_matches('}');
    format!("{body}{}}}\n", " ".repeat(padding))
}

#[tokio::test]
async fn a_file_above_the_size_limit_is_malformed() {
    let base = PASSED_LINE.len();
    let limit = usize::try_from(MAX_RESULT_BYTES).unwrap();
    let content = padded_line(limit + 1 - base);
    assert_eq!(content.len(), limit + 1);

    assert_malformed(content).await;
}

#[tokio::test]
async fn a_file_of_exactly_the_size_limit_is_read() {
    let base = PASSED_LINE.len();
    let limit = usize::try_from(MAX_RESULT_BYTES).unwrap();
    let content = padded_line(limit - base);
    assert_eq!(content.len(), limit);
    let sandbox = Sandbox::new();

    let answer = sandbox.read_restore_test(content).await.unwrap();

    assert_eq!(
        answer.map(|result| result.outcome),
        Some(CheckOutcome::Passed)
    );
}

#[tokio::test]
async fn a_huge_file_is_malformed() {
    assert_malformed(vec![b' '; 1024 * 1024]).await;
}

#[tokio::test]
async fn a_directory_with_the_name_of_the_file_is_malformed() {
    let sandbox = Sandbox::new();
    fs::create_dir(sandbox.file(ScheduledCheck::RestoreTest)).unwrap();

    let answer = sandbox.adapter().newest(ScheduledCheck::RestoreTest).await;

    assert!(
        matches!(answer, Err(CheckResultFault::Malformed { .. })),
        "got {answer:?}"
    );
}

/// How the test made the link.
#[derive(Debug)]
enum LinkKind {
    /// A real symbolic link to a file.
    Symlink,
    /// A Windows junction, because the account has no right to create a symbolic link.
    #[cfg(windows)]
    Junction,
}

/// Create a link at `link` that points to `target`.
///
/// A symbolic link needs a right that a plain Windows account may lack. A junction needs none, and
/// the standard library also reports it as a link. The reader rejects any entry that is no
/// regular file, so both kinds exercise the same rule.
fn make_link(target: &Path, link: &Path) -> LinkKind {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).unwrap();
        LinkKind::Symlink
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_file(target, link).is_ok() {
            return LinkKind::Symlink;
        }
        let directory = target.parent().unwrap();
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(directory)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "no link could be created");
        LinkKind::Junction
    }
}

#[tokio::test]
async fn a_link_with_the_name_of_the_file_is_malformed() {
    let sandbox = Sandbox::new();
    // The target is a valid passed result, so a reader that follows the link would pass.
    let target = sandbox.0.join("elsewhere").join("restore-test.result");
    fs::create_dir(sandbox.0.join("elsewhere")).unwrap();
    fs::write(&target, PASSED_LINE).unwrap();
    let kind = make_link(&target, &sandbox.file(ScheduledCheck::RestoreTest));

    let answer = sandbox.adapter().newest(ScheduledCheck::RestoreTest).await;

    assert!(
        matches!(answer, Err(CheckResultFault::Malformed { .. })),
        "{kind:?} gave {answer:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_file_without_the_read_right_is_unreadable() {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new();
    sandbox.write(ScheduledCheck::RestoreTest, PASSED_LINE);
    let path = sandbox.file(ScheduledCheck::RestoreTest);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    // A process that runs as root reads the file anyway, so the test proves nothing there.
    let root_reads_it = fs::File::open(&path).is_ok();

    let answer = sandbox.adapter().newest(ScheduledCheck::RestoreTest).await;

    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    if !root_reads_it {
        assert!(
            matches!(
                answer,
                Err(CheckResultFault::Unreadable {
                    check: ScheduledCheck::RestoreTest,
                    ..
                })
            ),
            "got {answer:?}"
        );
    }
}

// Tests of the hook `with_reader`. It gives the adapter a reader that fails or blocks.

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

    fn read(&self) -> io::Result<Reading> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let mut open = self.open.lock();
        while !*open {
            self.changed.wait(&mut open);
        }
        Ok(Reading::Content(PASSED_LINE.as_bytes().to_vec()))
    }

    fn adapter(self: &Arc<Self>, limit: Duration) -> FileCheckResults {
        let probe = Arc::clone(self);
        let reader: Reader = Arc::new(move |_path| probe.read());
        FileCheckResults::with_reader(
            PathBuf::from("backups"),
            PathBuf::from("wal"),
            limit,
            reader,
        )
    }
}

fn is_timed_out(answer: &Result<Option<CheckResult>, CheckResultFault>) -> bool {
    matches!(
        answer,
        Err(CheckResultFault::Unreadable { source, .. })
            if source.kind() == io::ErrorKind::TimedOut
    )
}

#[tokio::test]
async fn a_reader_that_fails_gives_unreadable_with_its_cause_and_its_check() {
    let reader: Reader = Arc::new(|_path| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
    let adapter = FileCheckResults::with_reader(
        PathBuf::from("backups"),
        PathBuf::from("wal"),
        LONG_LIMIT,
        reader,
    );

    let answer = adapter.newest(ScheduledCheck::BaseBackup).await;

    assert!(
        matches!(
            &answer,
            Err(CheckResultFault::Unreadable { check: ScheduledCheck::BaseBackup, source })
                if source.kind() == io::ErrorKind::PermissionDenied
        ),
        "got {answer:?}"
    );
}

#[tokio::test]
async fn a_reader_that_blocks_gives_unreadable_with_the_kind_timed_out() {
    let probe = Probe::blocked();
    let adapter = probe.adapter(SHORT_LIMIT);

    let answer = adapter.newest(ScheduledCheck::RestoreTest).await;
    probe.release();

    assert!(is_timed_out(&answer), "got {answer:?}");
    assert_eq!(probe.starts(), 1);
}

#[tokio::test]
async fn a_call_while_the_blocked_read_runs_gives_unreadable_at_once_and_starts_no_reader() {
    let probe = Probe::blocked();
    let adapter = probe.adapter(SHORT_LIMIT);
    let first = adapter.newest(ScheduledCheck::RestoreTest).await;
    assert!(is_timed_out(&first), "got {first:?}");

    let started = tokio::time::Instant::now();
    let second = adapter.newest(ScheduledCheck::RestoreTest).await;
    let waited = started.elapsed();
    probe.release();

    assert!(is_timed_out(&second), "got {second:?}");
    assert!(waited < SHORT_LIMIT, "the second call waited {waited:?}");
    assert_eq!(probe.starts(), 1, "the second call started a reader");
}

#[tokio::test]
async fn a_blocked_read_of_one_file_does_not_stop_the_read_of_the_other_file() {
    let probe = Probe::blocked();
    let adapter = probe.adapter(SHORT_LIMIT);
    let first = adapter.newest(ScheduledCheck::RestoreTest).await;
    assert!(is_timed_out(&first), "got {first:?}");

    let other = adapter.newest(ScheduledCheck::BaseBackup).await;
    probe.release();

    // The other file has its own mark, so its reader starts and reaches the limit by itself.
    assert!(is_timed_out(&other), "got {other:?}");
    assert_eq!(probe.starts(), 2, "the other file started no reader");
}

#[tokio::test]
async fn a_call_after_the_end_of_the_blocked_read_runs_the_reader_again() {
    let probe = Probe::blocked();
    let adapter = probe.adapter(SHORT_LIMIT);
    let first = adapter.newest(ScheduledCheck::RestoreTest).await;
    assert!(is_timed_out(&first), "got {first:?}");

    probe.release();
    // The mark of the ended reader clears a moment after the reader ends. Until then, a call
    // gives `Unreadable` and starts no reader. The first call that succeeds is the new run.
    let mut second = adapter.newest(ScheduledCheck::RestoreTest).await;
    for _ in 0..POLL_ATTEMPTS {
        if second.is_ok() {
            break;
        }
        tokio::time::sleep(POLL_STEP).await;
        second = adapter.newest(ScheduledCheck::RestoreTest).await;
    }

    assert_eq!(
        second.unwrap().map(|result| result.outcome),
        Some(CheckOutcome::Passed)
    );
    assert_eq!(probe.starts(), 2);
}

#[test]
fn the_fault_text_names_the_check_and_no_path() {
    let sandbox = Sandbox::new();
    let secret = sandbox.0.join("secret-directory-name");
    fs::create_dir(&secret).unwrap();
    fs::create_dir(secret.join("restore-test.result")).unwrap();
    let adapter = FileCheckResults::new(&secret, &secret, LONG_LIMIT);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    let fault = runtime
        .block_on(adapter.newest(ScheduledCheck::RestoreTest))
        .unwrap_err();

    assert!(fault.to_string().contains("restore test"), "got {fault}");
    assert!(!fault.to_string().contains("secret-directory-name"));
    assert!(!format!("{fault:?}").contains("secret-directory-name"));
}
