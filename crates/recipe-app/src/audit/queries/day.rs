//! The UTC day of the audit filters `from` and `to` (ADR-0051 (d), design D6).
//!
//! The module solves one problem: the filters take dates `YYYY-MM-DD` in UTC, and `recipe-web` has
//! no dependency on the `time` crate. The type `UtcDay` parses a date once, and `day_range` turns
//! two days into the `TimeRange` that the ports read. The crate `time` stays off the web layer.

use recipe_domain::timestamp::Timestamp;
use time::{Date, Month, PrimitiveDateTime, Time};

use crate::ports::TimeRange;

/// The length of a date `YYYY-MM-DD`.
const DAY_LENGTH: usize = 10;
/// The first year that a day accepts.
const FIRST_YEAR: i32 = 1;

/// A day of the calendar in UTC, from 0001-01-01 to 9999-12-31.
///
/// The type solves one problem: a bad date must not reach a query. The only constructor is
/// `UtcDay::parse`, so a value holds a day that exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtcDay(Date);

/// The reason why a text is no `UtcDay`. The text is not part of the reason.
///
/// This type is a field reason (`reliability.md` 1.1) and has no `code()`. The web layer wraps
/// it into `E-WEB-008` (`WebError::BadRequest`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayReason {
    /// The text is not four digits, a hyphen, two digits, a hyphen, and two digits.
    Grammar,
    /// The year is before 1, or the calendar has no such day.
    Calendar,
}

/// The reason why two days make no range.
///
/// This type is a field reason (`reliability.md` 1.1) and has no `code()`. The web layer wraps
/// it into `E-WEB-008` (`WebError::BadRequest`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayRangeReason {
    /// The last day is before the first day.
    ToBeforeFrom,
}

impl UtcDay {
    /// Parse a day written `YYYY-MM-DD`.
    ///
    /// # Errors
    ///
    /// Returns `DayReason::Grammar` for any other text shape. Returns `DayReason::Calendar` for the
    /// year 0 and for a day that the calendar does not have, for example 2023-02-29.
    pub fn parse(text: &str) -> Result<Self, DayReason> {
        let bytes = text.as_bytes();
        let dashes = bytes.get(4) == Some(&b'-') && bytes.get(7) == Some(&b'-');
        if text.len() != DAY_LENGTH || !dashes {
            return Err(DayReason::Grammar);
        }
        let (Some(year), Some(month), Some(day)) =
            (digits(text, 0, 4), digits(text, 5, 7), digits(text, 8, 10))
        else {
            return Err(DayReason::Grammar);
        };
        let year = i32::try_from(year).map_err(|_overflow| DayReason::Calendar)?;
        let month = u8::try_from(month)
            .ok()
            .and_then(|number| Month::try_from(number).ok())
            .ok_or(DayReason::Calendar)?;
        let day = u8::try_from(day).map_err(|_overflow| DayReason::Calendar)?;
        if year < FIRST_YEAR {
            return Err(DayReason::Calendar);
        }
        Date::from_calendar_date(year, month, day)
            .map(Self)
            .map_err(|_invalid| DayReason::Calendar)
    }

    /// Return the first instant of the day in UTC.
    fn start(self) -> Timestamp {
        instant_at_midnight(self.0)
    }

    /// Return the first instant of the next day, or `None` after 9999-12-31.
    fn next_start(self) -> Option<Timestamp> {
        self.0.next_day().map(instant_at_midnight)
    }
}

/// Return the number written by the digits `text[start..end]`, or `None` for any other character.
fn digits(text: &str, start: usize, end: usize) -> Option<u32> {
    let part = text.get(start..end)?;
    part.bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| part.parse().ok())
        .flatten()
}

/// Return the instant at 00:00:00 UTC of `date`.
fn instant_at_midnight(date: Date) -> Timestamp {
    Timestamp::from(PrimitiveDateTime::new(date, Time::MIDNIGHT).assume_utc())
}

/// Make the range of the filters `from` and `to`. Both days are inside the range.
///
/// The range starts at the start of `from` in UTC and ends at the start of the day after `to`. The
/// day 9999-12-31 has no next day, so a `to` of that day sets no upper bound. A missing day sets no
/// bound on its side.
///
/// # Errors
///
/// Returns `DayRangeReason::ToBeforeFrom` when `to` is before `from`.
pub fn day_range(from: Option<UtcDay>, to: Option<UtcDay>) -> Result<TimeRange, DayRangeReason> {
    if from.zip(to).is_some_and(|(first, last)| last < first) {
        return Err(DayRangeReason::ToBeforeFrom);
    }
    Ok(TimeRange {
        from: from.map(UtcDay::start),
        until: to.and_then(UtcDay::next_start),
    })
}
