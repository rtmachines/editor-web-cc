//! Date, time-of-day, and date-and-time literals (`domain-model.md` section 2.1, A43).

use super::calendar::{civil_from_days, days_from_civil};
use super::{IecLiteral, IecLiteralReason, TimeType};
use crate::plant::Primitive;

const NANOS_PER_MILLI: i128 = 1_000_000;
const NANOS_PER_SECOND: i128 = 1_000_000_000;
const SECONDS_PER_DAY: i128 = 86_400;
const NANOS_PER_DAY: i128 = 86_400_000_000_000;
const FIRST_YEAR: i64 = 0;
const LAST_YEAR: i64 = 9999;

/// The shape of the body of an instant literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    /// `date`, such as `2026-10-12`.
    Date,
    /// `tod`, such as `12:30:00.5`.
    Clock,
    /// `date-tod`, such as `2026-10-12-12:30:00`.
    DateTime,
}

/// The shape and the fraction rule of an instant type.
struct Spec {
    shape: Shape,
    max_fraction: usize,
}

/// A value split into whole days since 1970-01-01 and nanoseconds since midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Parts {
    days: i64,
    nanos_of_day: i128,
}

/// A parsed time of day, with its fraction scaled to nanoseconds.
struct Clock {
    hour: u8,
    minute: u8,
    second: u8,
    fraction_ns: i128,
    fraction_digits: usize,
}

/// Parse the body of an instant literal, the text after the `#`.
///
/// # Errors
///
/// Returns the [`IecLiteralReason`] of the first rule that the body breaks.
pub(super) fn parse(ty: TimeType, body: &str) -> Result<IecLiteral, IecLiteralReason> {
    let primitive = ty.primitive();
    let spec = spec_of(primitive).ok_or(IecLiteralReason::Prefix)?;
    let (date_text, clock_text) = split_body(spec.shape, body)?;
    let date = date_text.map(parse_date_text).transpose()?;
    let clock = clock_text.map(parse_clock_text).transpose()?;
    if clock
        .as_ref()
        .is_some_and(|clock| clock.fraction_digits > spec.max_fraction)
    {
        return Err(IecLiteralReason::Fraction);
    }
    let days = match date {
        Some((year, month, day)) => {
            days_from_civil(year, month, day).ok_or(IecLiteralReason::Calendar)?
        }
        None => 0,
    };
    let nanos_of_day = match clock {
        Some(clock) => nanos_of_clock(&clock).ok_or(IecLiteralReason::Calendar)?,
        None => 0,
    };
    let parts = Parts { days, nanos_of_day };
    let count = join_count(primitive, parts).ok_or(IecLiteralReason::OutOfRange)?;
    Ok(IecLiteral { ty, count })
}

/// Check a stored count of an instant type.
///
/// # Errors
///
/// Returns [`IecLiteralReason::OutOfRange`] for a count that the literal cannot show.
pub(super) fn check_count(ty: TimeType, count: i128) -> Result<(), IecLiteralReason> {
    let parts = split_count(ty.primitive(), count).ok_or(IecLiteralReason::OutOfRange)?;
    let (year, _, _) = civil_from_days(parts.days).ok_or(IecLiteralReason::OutOfRange)?;
    if (FIRST_YEAR..=LAST_YEAR).contains(&year) {
        Ok(())
    } else {
        Err(IecLiteralReason::OutOfRange)
    }
}

/// Render the canonical text of an instant literal. A count that its type cannot show gives
/// `None`.
pub(super) fn render_instant(literal: &IecLiteral) -> Option<String> {
    let primitive = literal.ty.primitive();
    let spec = spec_of(primitive)?;
    let parts = split_count(primitive, literal.count)?;
    let mut text = String::from(prefix_of(primitive)?);
    match spec.shape {
        Shape::Date => text.push_str(&render_date(parts.days)?),
        Shape::Clock => text.push_str(&render_clock(parts.nanos_of_day)?),
        Shape::DateTime => {
            text.push_str(&render_date(parts.days)?);
            text.push('-');
            text.push_str(&render_clock(parts.nanos_of_day)?);
        }
    }
    Some(text)
}

/// Return the shape and the fraction rule of an instant type.
fn spec_of(primitive: Primitive) -> Option<Spec> {
    let (shape, max_fraction) = match primitive {
        Primitive::Date | Primitive::Ldate => (Shape::Date, 0),
        Primitive::TimeOfDay => (Shape::Clock, 3),
        Primitive::Ltod => (Shape::Clock, 9),
        Primitive::DateAndTime => (Shape::DateTime, 0),
        Primitive::Ldt | Primitive::Dtl => (Shape::DateTime, 9),
        _ => return None,
    };
    Some(Spec {
        shape,
        max_fraction,
    })
}

/// Return the literal prefix of an instant type, such as `LTOD#`.
fn prefix_of(primitive: Primitive) -> Option<&'static str> {
    match primitive {
        Primitive::Date => Some("D#"),
        Primitive::Ldate => Some("LD#"),
        Primitive::TimeOfDay => Some("TOD#"),
        Primitive::Ltod => Some("LTOD#"),
        Primitive::DateAndTime => Some("DT#"),
        Primitive::Ldt => Some("LDT#"),
        Primitive::Dtl => Some("DTL#"),
        _ => None,
    }
}

/// Split the body into its date text and its time-of-day text.
fn split_body(shape: Shape, body: &str) -> Result<(Option<&str>, Option<&str>), IecLiteralReason> {
    match shape {
        Shape::Date if body.contains('.') => Err(IecLiteralReason::Fraction),
        Shape::Date => Ok((Some(body), None)),
        Shape::Clock => Ok((None, Some(body))),
        Shape::DateTime => {
            let (date, clock) = body.rsplit_once('-').ok_or(IecLiteralReason::Grammar)?;
            Ok((Some(date), Some(clock)))
        }
    }
}

/// Return `true` when the text has exactly `length` ASCII digits.
fn is_digits(text: &str, length: usize) -> bool {
    text.len() == length && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Parse the grammar of a date, `YYYY-MM-DD`, without the calendar check.
fn parse_date_text(text: &str) -> Result<(i64, u8, u8), IecLiteralReason> {
    let fields: Vec<&str> = text.split('-').collect();
    let [year, month, day] = fields.as_slice() else {
        return Err(IecLiteralReason::Grammar);
    };
    if !is_digits(year, 4) || !is_digits(month, 2) || !is_digits(day, 2) {
        return Err(IecLiteralReason::Grammar);
    }
    let year = year.parse::<i64>().map_err(|_| IecLiteralReason::Grammar)?;
    let month = month.parse::<u8>().map_err(|_| IecLiteralReason::Grammar)?;
    let day = day.parse::<u8>().map_err(|_| IecLiteralReason::Grammar)?;
    Ok((year, month, day))
}

/// Parse the grammar of a time of day, `HH:MM:SS` with an optional fraction.
fn parse_clock_text(text: &str) -> Result<Clock, IecLiteralReason> {
    let (clock, fraction) = match text.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (text, None),
    };
    let fields: Vec<&str> = clock.split(':').collect();
    let [hour, minute, second] = fields.as_slice() else {
        return Err(IecLiteralReason::Grammar);
    };
    if !is_digits(hour, 2) || !is_digits(minute, 2) || !is_digits(second, 2) {
        return Err(IecLiteralReason::Grammar);
    }
    let (fraction_ns, fraction_digits) = match fraction {
        Some(digits) => parse_fraction(digits)?,
        None => (0, 0),
    };
    Ok(Clock {
        hour: hour.parse().map_err(|_| IecLiteralReason::Grammar)?,
        minute: minute.parse().map_err(|_| IecLiteralReason::Grammar)?,
        second: second.parse().map_err(|_| IecLiteralReason::Grammar)?,
        fraction_ns,
        fraction_digits,
    })
}

/// Scale a fraction of one to nine digits to nanoseconds.
fn parse_fraction(digits: &str) -> Result<(i128, usize), IecLiteralReason> {
    let length = digits.len();
    if !(1..=9).contains(&length) || !is_digits(digits, length) {
        return Err(IecLiteralReason::Grammar);
    }
    let value = digits
        .parse::<i128>()
        .map_err(|_| IecLiteralReason::Grammar)?;
    let padding = 9_usize
        .checked_sub(length)
        .ok_or(IecLiteralReason::Grammar)?;
    let scale = 10_i128
        .checked_pow(u32::try_from(padding).map_err(|_| IecLiteralReason::Grammar)?)
        .ok_or(IecLiteralReason::Grammar)?;
    let nanos = value.checked_mul(scale).ok_or(IecLiteralReason::Grammar)?;
    Ok((nanos, length))
}

/// Return the nanoseconds since midnight of a time of day, after the range check.
fn nanos_of_clock(clock: &Clock) -> Option<i128> {
    let hour = i128::from(clock.hour);
    let minute = i128::from(clock.minute);
    let second = i128::from(clock.second);
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let seconds = hour
        .checked_mul(3600)?
        .checked_add(minute.checked_mul(60)?)?
        .checked_add(second)?;
    seconds
        .checked_mul(NANOS_PER_SECOND)?
        .checked_add(clock.fraction_ns)
}

/// Return the count of the base unit of a type from its parts (A43).
fn join_count(primitive: Primitive, parts: Parts) -> Option<i128> {
    let days = i128::from(parts.days);
    match primitive {
        Primitive::Date => Some(days),
        Primitive::Ldate => days.checked_mul(NANOS_PER_DAY),
        Primitive::TimeOfDay => parts.nanos_of_day.checked_div(NANOS_PER_MILLI),
        Primitive::Ltod => Some(parts.nanos_of_day),
        Primitive::DateAndTime => days
            .checked_mul(SECONDS_PER_DAY)?
            .checked_add(parts.nanos_of_day.checked_div(NANOS_PER_SECOND)?),
        Primitive::Ldt | Primitive::Dtl => days
            .checked_mul(NANOS_PER_DAY)?
            .checked_add(parts.nanos_of_day),
        _ => None,
    }
}

/// Split a stored count of a type into its parts. Returns `None` for a count the type cannot hold.
fn split_count(primitive: Primitive, count: i128) -> Option<Parts> {
    match primitive {
        Primitive::Date => Some(Parts {
            days: i64::try_from(count).ok()?,
            nanos_of_day: 0,
        }),
        Primitive::Ldate => {
            if count.checked_rem(NANOS_PER_DAY)? != 0 {
                return None;
            }
            Some(Parts {
                days: i64::try_from(count.checked_div(NANOS_PER_DAY)?).ok()?,
                nanos_of_day: 0,
            })
        }
        Primitive::TimeOfDay => clock_parts(count.checked_mul(NANOS_PER_MILLI)?),
        Primitive::Ltod => clock_parts(count),
        Primitive::DateAndTime => Some(Parts {
            days: i64::try_from(count.checked_div_euclid(SECONDS_PER_DAY)?).ok()?,
            nanos_of_day: count
                .checked_rem_euclid(SECONDS_PER_DAY)?
                .checked_mul(NANOS_PER_SECOND)?,
        }),
        Primitive::Ldt | Primitive::Dtl => Some(Parts {
            days: i64::try_from(count.checked_div_euclid(NANOS_PER_DAY)?).ok()?,
            nanos_of_day: count.checked_rem_euclid(NANOS_PER_DAY)?,
        }),
        _ => None,
    }
}

/// Return the parts of a time of day in nanoseconds. A value of one day or more is `None`.
fn clock_parts(nanos: i128) -> Option<Parts> {
    if (0..NANOS_PER_DAY).contains(&nanos) {
        Some(Parts {
            days: 0,
            nanos_of_day: nanos,
        })
    } else {
        None
    }
}

/// Render a date as `YYYY-MM-DD`.
fn render_date(days: i64) -> Option<String> {
    let (year, month, day) = civil_from_days(days)?;
    if !(FIRST_YEAR..=LAST_YEAR).contains(&year) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

/// Render a time of day as `HH:MM:SS`, with a fraction that has no trailing zeros.
fn render_clock(nanos: i128) -> Option<String> {
    let seconds = nanos.checked_div(NANOS_PER_SECOND)?;
    let fraction = nanos.checked_rem(NANOS_PER_SECOND)?;
    let hour = seconds.checked_div(3600)?;
    let minute = seconds.checked_div(60)?.checked_rem(60)?;
    let second = seconds.checked_rem(60)?;
    let mut text = format!("{hour:02}:{minute:02}:{second:02}");
    if fraction != 0 {
        let digits = format!("{fraction:09}");
        text.push('.');
        text.push_str(digits.trim_end_matches('0'));
    }
    Some(text)
}

#[cfg(test)]
mod tests;
