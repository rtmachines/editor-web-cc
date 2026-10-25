//! Adapter of the port `BackupDirectory` (ADR-0074 (c)).
//!
//! The adapter finds a complete set only by its manifest. The anchored grammar of the whole
//! file name decides which file is a manifest. A temp name such as `.tmp-recipe_...` never
//! matches. The time of a set is the modification time of its manifest. The script
//! `backup-health` reads the same time.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use recipe_app::ports::{BackupDirectory, BackupFault};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

/// The start of the name of a manifest.
const MANIFEST_PREFIX: &str = "recipe_";

/// The end of the name of a manifest.
const MANIFEST_SUFFIX: &str = ".dump.sha256";

/// The suffixes that a run of `backup-once` can add to the time of a set (ADR-0066 (e)).
const SET_SUFFIXES: [&str; 3] = ["", "_premigrate", "_now"];

/// The number of digits of the date `YYYYMMDD` in a name.
const DATE_DIGITS: usize = 8;

/// The number of digits of the time `HHMM` in a name.
const MINUTE_DIGITS: usize = 4;

/// The read-only mount of the `backups` volume.
///
/// The type solves one problem: the job `alerts` must tell a directory without a complete set
/// from a directory that it cannot read. The first case is normal on a new installation. The
/// second case is a fault (ADR-0073 (c)).
#[derive(Clone, Debug)]
pub struct FileBackupDirectory {
    /// The directory that holds the files of the sets.
    directory: PathBuf,
}

impl FileBackupDirectory {
    /// Build the adapter for `directory`. The call reads nothing.
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }
}

#[async_trait]
impl BackupDirectory for FileBackupDirectory {
    async fn newest_complete_set(&self) -> Result<Option<Timestamp>, BackupFault> {
        let directory = self.directory.clone();
        let scan = tokio::task::spawn_blocking(move || newest_manifest_time(&directory)).await;
        match scan {
            Ok(Ok(newest)) => Ok(newest),
            Ok(Err(fault)) => Err(BackupFault::Unreadable(fault)),
            Err(join_fault) => Err(BackupFault::Unreadable(io::Error::other(join_fault))),
        }
    }
}

/// Return the newest modification time of a manifest in `directory`, or `None` without one.
fn newest_manifest_time(directory: &Path) -> io::Result<Option<Timestamp>> {
    let mut newest: Option<SystemTime> = None;
    for entry in fs::read_dir(directory)? {
        let Some(modified) = manifest_time(&entry?)? else {
            continue;
        };
        newest = Some(newest.map_or(modified, |current| current.max(modified)));
    }
    newest.map(timestamp_of).transpose()
}

/// Return the modification time of `entry` when it is a manifest, else `None`.
///
/// A file that a retention run deletes between the listing and this call is not a manifest.
fn manifest_time(entry: &fs::DirEntry) -> io::Result<Option<SystemTime>> {
    let name = entry.file_name();
    if !name.to_str().is_some_and(is_manifest_name) {
        return Ok(None);
    }
    let metadata = match entry.metadata() {
        Ok(metadata) => metadata,
        Err(fault) if fault.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(fault) => return Err(fault),
    };
    // The metadata of a directory entry does not follow a link, so a link is no manifest.
    if !metadata.is_file() {
        return Ok(None);
    }
    metadata.modified().map(Some)
}

/// Check the name against `^recipe_[0-9]{8}_[0-9]{4}(_premigrate|_now)?\.dump\.sha256$`.
fn is_manifest_name(name: &str) -> bool {
    let Some(middle) = name
        .strip_prefix(MANIFEST_PREFIX)
        .and_then(|rest| rest.strip_suffix(MANIFEST_SUFFIX))
    else {
        return false;
    };
    SET_SUFFIXES
        .iter()
        .any(|suffix| middle.strip_suffix(suffix).is_some_and(is_date_and_minute))
}

/// Check the text `YYYYMMDD_HHMM` with ASCII digits only.
fn is_date_and_minute(text: &str) -> bool {
    let Some((date, minute)) = text.split_once('_') else {
        return false;
    };
    is_digits(date, DATE_DIGITS) && is_digits(minute, MINUTE_DIGITS)
}

/// Check that `text` holds exactly `count` ASCII digits.
fn is_digits(text: &str, count: usize) -> bool {
    text.len() == count && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Convert a modification time into a domain instant without a panic path.
fn timestamp_of(moment: SystemTime) -> io::Result<Timestamp> {
    let nanos = match moment.duration_since(UNIX_EPOCH) {
        Ok(after) => i128::try_from(after.as_nanos()).ok(),
        Err(before) => i128::try_from(before.duration().as_nanos())
            .ok()
            .and_then(i128::checked_neg),
    };
    nanos
        .and_then(|nanos| OffsetDateTime::from_unix_timestamp_nanos(nanos).ok())
        .map(Timestamp::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "manifest time out of range"))
}

#[cfg(test)]
mod tests;
