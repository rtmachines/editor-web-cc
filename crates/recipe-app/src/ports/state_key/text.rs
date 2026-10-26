//! The one text form of the state values of `app_setting` (ADR-0073 (i)).
//!
//! The crate `time` has no `parsing` feature here, so the parser of an instant is written by hand.
//! Every function returns `None` for a value or a text that has no form. The adapter maps `None`
//! to its own `StorageError`.

use recipe_domain::timestamp::Timestamp;
use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use crate::ports::ChainSeq;

/// The length in bytes of `2001-09-09T01:46:40.000000Z`.
const TIME_TEXT_LEN: usize = 27;

/// The first year that the text form of an instant holds, with four digits.
const FIRST_YEAR: i32 = 0;

/// The last year that the text form of an instant holds, with four digits.
const LAST_YEAR: i32 = 9999;

/// The position and the byte of each separator of the text form of an instant.
const SEPARATORS: [(usize, u8); 7] = [
    (4, b'-'),
    (7, b'-'),
    (10, b'T'),
    (13, b':'),
    (16, b':'),
    (19, b'.'),
    (26, b'Z'),
];

/// Return the text of a `chain_seq`: decimal, no sign, no leading zero.
/// A negative number has none.
pub(super) fn chain_seq_text(seq: ChainSeq) -> Option<String> {
    u64::try_from(seq.get())
        .ok()
        .map(|number| number.to_string())
}

/// Parse the text of a `chain_seq`, or return `None` when it breaks the form of the writer.
///
/// The form is decimal digits with no sign, no space, and no leading zero. The value fits `i64`.
pub(super) fn parse_chain_seq(text: &str) -> Option<ChainSeq> {
    let digits_only = text.bytes().all(|byte| byte.is_ascii_digit());
    let has_leading_zero = text.len() > 1 && text.starts_with('0');
    if text.is_empty() || !digits_only || has_leading_zero {
        return None;
    }
    text.parse::<i64>().ok().map(ChainSeq::from)
}

/// Return the text of an instant: RFC 3339 in UTC with `Z` and six fraction digits.
///
/// The text drops the time below one microsecond. An instant whose UTC year is outside
/// `0..=9999` has no text.
pub(super) fn instant_text(at: Timestamp) -> Option<String> {
    let utc = at
        .as_offset_date_time()
        .checked_to_offset(UtcOffset::UTC)
        .filter(|utc| (FIRST_YEAR..=LAST_YEAR).contains(&utc.year()))?;
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",
        utc.year(),
        u8::from(utc.month()),
        utc.day(),
        utc.hour(),
        utc.minute(),
        utc.second(),
        utc.microsecond(),
    ))
}

/// Parse the text of an instant, or return `None` when one byte breaks the form.
pub(super) fn parse_instant(text: &str) -> Option<Timestamp> {
    let bytes = text.as_bytes();
    if bytes.len() != TIME_TEXT_LEN {
        return None;
    }
    if !SEPARATORS
        .iter()
        .all(|&(position, byte)| bytes.get(position) == Some(&byte))
    {
        return None;
    }
    let year = i32::try_from(digits(bytes, 0, 4)?).ok()?;
    let month = Month::try_from(small(digits(bytes, 5, 2)?)?).ok()?;
    let day = small(digits(bytes, 8, 2)?)?;
    let hour = small(digits(bytes, 11, 2)?)?;
    let minute = small(digits(bytes, 14, 2)?)?;
    let second = small(digits(bytes, 17, 2)?)?;
    let microsecond = digits(bytes, 20, 6)?;
    let date = Date::from_calendar_date(year, month, day).ok()?;
    let time = Time::from_hms_micro(hour, minute, second, microsecond).ok()?;
    Some(Timestamp::from(
        PrimitiveDateTime::new(date, time).assume_utc(),
    ))
}

/// Narrow a field of two digits to a byte.
fn small(value: u32) -> Option<u8> {
    u8::try_from(value).ok()
}

/// Read `len` ASCII digits that start at `start` as a number, or return `None`.
fn digits(bytes: &[u8], start: usize, len: usize) -> Option<u32> {
    let end = start.checked_add(len)?;
    bytes
        .get(start..end)?
        .iter()
        .try_fold(0_u32, |number, byte| {
            let digit = char::from(*byte).to_digit(10)?;
            number.checked_mul(10)?.checked_add(digit)
        })
}
