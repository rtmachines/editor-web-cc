//! The closed set of the state keys of `app_setting` (`data-model.md` section 3.17).
//!
//! `app_setting` holds configuration keys and three state keys. A job writes a state key with no
//! audit entry (`data-model.md` section 3.17). A job must never write a configuration key this
//! way. Both types are closed, so a wrong key fails to compile and not at run time.
//!
//! This port holds the one text form of the values (ADR-0073 (i)). [`StateValue::to_text`] writes
//! it. [`TimeStateKey::parse_text`] reads a time and [`parse_chain_seq_text`] reads a `chain_seq`.
//! The adapter of `recipe-db` and the memory adapter both call these functions, so the writer and
//! the readers share one form. Each adapter maps `None` to its own `StorageError`.

use recipe_domain::timestamp::Timestamp;

use super::ChainSeq;

mod text;

/// The key of the last `chain_seq` that the audit chain check verified.
const AUDIT_LAST_CHECK_SEQ: &str = "audit.last_check_seq";
/// The key of the time of the last completed audit chain check.
const AUDIT_LAST_CHECK_AT: &str = "audit.last_check_at";
/// The key of the time of the last run of the job `heartbeat`.
const HEARTBEAT_LAST_AT: &str = "heartbeat.last_at";

/// The three state keys of `app_setting`.
///
/// The list solves one problem: the literals of the keys have one definition. `StateValue::key`
/// and `TimeStateKey::key` read the three constants. The port `SettingRepository` reads the list.
pub(super) const STATE_KEYS: [&str; 3] =
    [AUDIT_LAST_CHECK_SEQ, AUDIT_LAST_CHECK_AT, HEARTBEAT_LAST_AT];

/// One state key with its typed value, for the write of a job.
///
/// The type solves one problem: a job could pass any `SettingKey` to the write of a state key.
/// It could then change a configuration key with no `SettingChanged` entry (A33,
/// `reliability.md` section 7.12). Each variant names one key.
///
/// An adapter writes the text form of ADR-0073 (i) through [`StateValue::to_text`]. The `chain_seq`
/// is decimal text with no sign and no leading zero. A time is RFC 3339 in UTC with `Z` and six
/// fraction digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateValue {
    /// The highest `chain_seq` that a completed audit chain check confirmed
    /// (`audit.last_check_seq`). The value never decreases (ADR-0086 (e)). It is an anchor and not
    /// a start position, because each check starts at the first row.
    AuditLastCheckSeq(ChainSeq),
    /// The time of the last completed audit chain check (`audit.last_check_at`).
    AuditLastCheckAt(Timestamp),
    /// The time of the last run of the job `heartbeat` (`heartbeat.last_at`).
    HeartbeatLastAt(Timestamp),
}

impl StateValue {
    /// Return the key of the `app_setting` row that holds this value.
    #[must_use]
    pub const fn key(&self) -> &'static str {
        match self {
            Self::AuditLastCheckSeq(_) => AUDIT_LAST_CHECK_SEQ,
            Self::AuditLastCheckAt(_) => AUDIT_LAST_CHECK_AT,
            Self::HeartbeatLastAt(_) => HEARTBEAT_LAST_AT,
        }
    }

    /// Return the text that the row of this value holds (ADR-0073 (i)).
    ///
    /// The `chain_seq` gives decimal text with no sign and no leading zero. A time gives RFC 3339
    /// in UTC with `Z` and exactly six fraction digits. The text cuts the time below one
    /// microsecond.
    ///
    /// Returns `None` for a negative `chain_seq`. It also returns `None` for a time whose UTC year
    /// is outside `0..=9999`, because the text has four digits for the year.
    #[must_use]
    pub fn to_text(&self) -> Option<String> {
        match self {
            Self::AuditLastCheckSeq(seq) => text::chain_seq_text(*seq),
            Self::AuditLastCheckAt(at) | Self::HeartbeatLastAt(at) => text::instant_text(*at),
        }
    }
}

/// Parse the stored text of a `chain_seq` (ADR-0073 (i)).
///
/// The function is the reader of [`StateValue::to_text`] for [`StateValue::AuditLastCheckSeq`].
/// It accepts only decimal digits with no sign and no leading zero, and a value that fits `i64`.
/// It returns `None` for every other text. The adapter maps `None` to a `StorageError`.
#[must_use]
pub fn parse_chain_seq_text(text: &str) -> Option<ChainSeq> {
    text::parse_chain_seq(text)
}

/// One state key that holds a time, for the read of the job `alerts` and of its catch-up.
///
/// The type solves one problem: a read of any `SettingKey` could name `audit.last_check_seq`,
/// which is a `chain_seq` and not a time. This type lists only the two keys that hold a time.
///
/// An adapter reads a stored time through [`TimeStateKey::parse_text`]. The form is RFC 3339 in UTC
/// with `Z` and six fraction digits (ADR-0073 (i)). The adapter maps a bad form to a
/// `StorageError` and not to "no value".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeStateKey {
    /// The time of the last completed audit chain check (`audit.last_check_at`).
    AuditLastCheckAt,
    /// The time of the last run of the job `heartbeat` (`heartbeat.last_at`).
    HeartbeatLastAt,
}

impl TimeStateKey {
    /// Return the key of the `app_setting` row that holds this time.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::AuditLastCheckAt => AUDIT_LAST_CHECK_AT,
            Self::HeartbeatLastAt => HEARTBEAT_LAST_AT,
        }
    }

    /// Parse the stored text of a time (ADR-0073 (i)).
    ///
    /// The function accepts only RFC 3339 in UTC with `Z` and exactly six fraction digits, for
    /// example `2026-10-23T02:00:00.000000Z`. It returns `None` for every other text. It is the
    /// reader of [`StateValue::to_text`].
    #[must_use]
    pub fn parse_text(text: &str) -> Option<Timestamp> {
        text::parse_instant(text)
    }
}

#[cfg(test)]
mod tests;
