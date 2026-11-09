//! Adapter of the port `CheckResults` (ADR-0151 (a) to (c), (o), design D4 and D10).
//!
//! Each scheduled backup check leaves one result file. The restore test writes
//! `restore-test.result` in the `backups` volume. The base backup writes `base-backup.result` in
//! the `walarchive` volume. A file holds one JSON line, for example
//! `{"result":"passed","finished_at":"2026-11-08T05:41:07Z"}`. The reader accepts exactly that
//! content. Any other content is the fault `Malformed`, and it never gives a passed result.
//! Each read runs under the time limit of the module `limited_read`.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use recipe_app::ports::{
    CheckOutcome, CheckResult, CheckResultFault, CheckResults, ScheduledCheck,
};
use recipe_domain::timestamp::Timestamp;
use serde::Deserialize;
use time::PrimitiveDateTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;

use crate::limited_read::{StalledPaths, read_with_limit};

/// The name of the result file of the restore test.
const RESTORE_TEST_FILE: &str = "restore-test.result";

/// The name of the result file of the base backup.
const BASE_BACKUP_FILE: &str = "base-backup.result";

/// The largest size of a result file in bytes (`reliability.md` section 7.11, `check_result.max_bytes`).
const MAX_RESULT_BYTES: u64 = 256;

/// The one accepted form of `finished_at` (design D4).
///
/// The `T` and the `Z` are literals. The RFC 3339 parser is no choice here, because it also
/// accepts an offset, a lowercase `t` or `z`, and a fraction of a second.
const TIME_FORMAT: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");

/// The length in bytes of `finished_at`, for example `2026-11-08T05:41:07Z`.
///
/// The time parser accepts a sign before the year. The fixed length refuses it.
const FINISHED_AT_LENGTH: usize = "2026-11-08T05:41:07Z".len();

/// What the reader finds at the path of a result file.
#[derive(Debug)]
enum Reading {
    /// No entry has the name of the file.
    Missing,
    /// The entry is a regular file within the size limit. The bytes are its content.
    Content(Vec<u8>),
    /// The entry is a link, a directory, another kind of entry, or a file above the size limit.
    Rejected,
}

/// The function that reads one path. A unit test gives a reader that fails or blocks.
type Reader = Arc<dyn Fn(&Path) -> io::Result<Reading> + Send + Sync>;

/// The read-only mounts that hold the result files of the scheduled checks.
///
/// The type solves one problem: the job `alerts` must tell a check that never ran from a check
/// that failed and from a result file that it cannot trust. A missing file is `None`. A file that
/// cannot be read is `Unreadable`. A file with other content than the one line is `Malformed`
/// (ADR-0151 (i)). A stalled mount must not block the job, so each read waits at most `limit`
/// (ADR-0151 (o)).
#[derive(Clone)]
pub struct FileCheckResults {
    restore_test: PathBuf,
    base_backup: PathBuf,
    limit: Duration,
    stalled: StalledPaths,
    reader: Reader,
}

impl FileCheckResults {
    /// Build the adapter for the volumes `backups` and `walarchive`. The call reads nothing.
    ///
    /// `limit` is the setting `health.disk_check_interval` (ADR-0151 (o)).
    #[must_use]
    pub fn new(
        backups: impl Into<PathBuf>,
        walarchive: impl Into<PathBuf>,
        limit: Duration,
    ) -> Self {
        Self::with_reader(
            backups.into(),
            walarchive.into(),
            limit,
            Arc::new(read_result_file),
        )
    }

    /// Build the adapter with another reader. The unit tests use it.
    fn with_reader(backups: PathBuf, walarchive: PathBuf, limit: Duration, reader: Reader) -> Self {
        Self {
            restore_test: backups.join(RESTORE_TEST_FILE),
            base_backup: walarchive.join(BASE_BACKUP_FILE),
            limit,
            stalled: StalledPaths::default(),
            reader,
        }
    }

    /// The path of the result file of `check`. It is also the key of the stalled paths.
    fn path_of(&self, check: ScheduledCheck) -> &Path {
        match check {
            ScheduledCheck::RestoreTest => &self.restore_test,
            ScheduledCheck::BaseBackup => &self.base_backup,
        }
    }
}

impl fmt::Debug for FileCheckResults {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileCheckResults")
            .field("restore_test", &self.restore_test)
            .field("base_backup", &self.base_backup)
            .field("limit", &self.limit)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl CheckResults for FileCheckResults {
    async fn newest(&self, check: ScheduledCheck) -> Result<Option<CheckResult>, CheckResultFault> {
        let reader = Arc::clone(&self.reader);
        let read = read_with_limit(
            self.limit,
            &self.stalled,
            self.path_of(check).to_path_buf(),
            move |path| reader(path),
        )
        .await;
        match read {
            Ok(Ok(Reading::Missing)) => Ok(None),
            Ok(Ok(Reading::Content(bytes))) => parse_result(&bytes)
                .map(Some)
                .ok_or(CheckResultFault::Malformed { check }),
            Ok(Ok(Reading::Rejected)) => Err(CheckResultFault::Malformed { check }),
            Ok(Err(source)) => Err(CheckResultFault::Unreadable { check, source }),
            Err(fault) => Err(CheckResultFault::Unreadable {
                check,
                source: fault.into_io_error(),
            }),
        }
    }
}

/// Read the file at `path` without a link and with at most `MAX_RESULT_BYTES` bytes.
///
/// A missing file is `Reading::Missing`. A link, a directory, and a file above the limit are
/// `Reading::Rejected`. The metadata of a link does not follow it, so a link never gives the
/// content of its target.
fn read_result_file(path: &Path) -> io::Result<Reading> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(fault) if fault.kind() == io::ErrorKind::NotFound => return Ok(Reading::Missing),
        Err(fault) => return Err(fault),
    };
    if !metadata.is_file() || metadata.len() > MAX_RESULT_BYTES {
        return Ok(Reading::Rejected);
    }
    let file = match File::open(path) {
        Ok(file) => file,
        // A run of a script can replace the file between the two calls.
        Err(fault) if fault.kind() == io::ErrorKind::NotFound => return Ok(Reading::Missing),
        Err(fault) => return Err(fault),
    };
    let mut content = Vec::new();
    // One byte above the limit shows a file that grew after the metadata call.
    let read = file
        .take(MAX_RESULT_BYTES.saturating_add(1))
        .read_to_end(&mut content)?;
    if u64::try_from(read).is_ok_and(|size| size <= MAX_RESULT_BYTES) {
        Ok(Reading::Content(content))
    } else {
        Ok(Reading::Rejected)
    }
}

/// The two keys of a result line and nothing else (ADR-0151 (b)).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultLine {
    result: ResultWord,
    finished_at: String,
}

/// The two words that `result` accepts. The match is exact and uses lowercase.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ResultWord {
    Passed,
    Failed,
}

/// Parse the content of a result file, or return `None` when it is not a valid result.
///
/// The content is one JSON line with at most one `\n` at its end. The time has the fixed form of
/// `TIME_FORMAT` and is UTC.
fn parse_result(content: &[u8]) -> Option<CheckResult> {
    let line = content.strip_suffix(b"\n").unwrap_or(content);
    if line.iter().any(|byte| matches!(byte, b'\n' | b'\r')) {
        return None;
    }
    // The derive also reads a JSON array as the two fields. The result line is an object.
    if line.trim_ascii_start().first() != Some(&b'{') {
        return None;
    }
    let parsed: ResultLine = serde_json::from_slice(line).ok()?;
    if parsed.finished_at.len() != FINISHED_AT_LENGTH {
        return None;
    }
    let finished_at = PrimitiveDateTime::parse(&parsed.finished_at, TIME_FORMAT).ok()?;
    Some(CheckResult {
        outcome: match parsed.result {
            ResultWord::Passed => CheckOutcome::Passed,
            ResultWord::Failed => CheckOutcome::Failed,
        },
        finished_at: Timestamp::from(finished_at.assume_utc()),
    })
}

#[cfg(test)]
mod tests;
