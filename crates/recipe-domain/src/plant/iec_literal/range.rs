//! Range of the count of each time type (`domain-model.md` section 2.1, ADR-0026).
//!
//! One table gives the range of a type on a platform. The parser, the stored-count check,
//! and the process-limit check use this table.

use std::ops::RangeInclusive;

use super::{IecLiteralReason, TimeType};
use crate::plant::{PlcPlatform, Primitive};

/// `TIME` on CODESYS and TwinCAT 3: unsigned 32 bit, in milliseconds.
const TIME_UNSIGNED: RangeInclusive<i128> = 0..=4_294_967_295;
/// `TIME` on S7: signed 32 bit, in milliseconds.
const TIME_SIGNED: RangeInclusive<i128> = -2_147_483_648..=2_147_483_647;
/// `LTIME` on CODESYS and TwinCAT 3: unsigned 64 bit, in nanoseconds.
const LTIME_UNSIGNED: RangeInclusive<i128> = 0..=18_446_744_073_709_551_615;
/// `LTIME` on S7-1500: signed 64 bit, in nanoseconds.
const LTIME_SIGNED: RangeInclusive<i128> = -9_223_372_036_854_775_808..=9_223_372_036_854_775_807;
/// `S5TIME` on S7-1500 and S7-300: `9 990 s`, in milliseconds.
const S5TIME: RangeInclusive<i128> = 0..=9_990_000;
/// `DATE` on CODESYS and TwinCAT 3: 1970-01-01 to 2106-02-07, in days (ADR-0026).
const DATE_UNSIGNED: RangeInclusive<i128> = 0..=49_710;
/// `DATE` on every S7 platform: 1990-01-01 to 2168-12-31, in days (ADR-0114).
const DATE_S7: RangeInclusive<i128> = 7_305..=72_683;
/// `DATE` on S7-1200 and S7-1500 in the earlier rule set: 1990-01-01 to 2169-06-06, in days.
///
/// ADR-0120 (a) keeps this end for the stored data that a user wrote before ADR-0114.
const DATE_S7_EARLIER: RangeInclusive<i128> = 7_305..=72_840;
/// `LDATE` on CODESYS and TwinCAT 3: unsigned 64 bit, in nanoseconds.
const LDATE: RangeInclusive<i128> = 0..=18_446_744_073_709_551_615;
/// `TIME_OF_DAY` on every platform: below one day, in milliseconds.
const TIME_OF_DAY: RangeInclusive<i128> = 0..=86_399_999;
/// `LTOD` where it exists: the `tod` grammar keeps a literal below one day, in nanoseconds.
const LTOD: RangeInclusive<i128> = 0..=86_399_999_999_999;
/// `DATE_AND_TIME` on S7-1500 and S7-300: 1990-01-01-00:00:00 to 2089-12-31-23:59:59, in seconds (ADR-0026).
const DATE_AND_TIME_S7: RangeInclusive<i128> = 631_152_000..=3_786_911_999;
/// `DATE_AND_TIME` on CODESYS and TwinCAT 3: 1970-01-01 to 2106-02-07-06:28:15, in seconds.
const DATE_AND_TIME_UNSIGNED: RangeInclusive<i128> = 0..=4_294_967_295;
/// `LDT` on CODESYS and TwinCAT 3: unsigned 64 bit, in nanoseconds.
const LDT_UNSIGNED: RangeInclusive<i128> = 0..=18_446_744_073_709_551_615;
/// `LDT` on S7-1500: never negative, 63 bit, in nanoseconds.
const LDT_S7: RangeInclusive<i128> = 0..=9_223_372_036_854_775_807;
/// `DTL` on S7-1200 and S7-1500: 1970-01-01 to 2262-04-11, never negative, in nanoseconds.
const DTL: RangeInclusive<i128> = 0..=9_223_372_036_854_775_807;

/// The set of value rules that a check applies (ADR-0120).
///
/// A command checks the data that it writes with `Current`. It checks a stored part that it
/// only reads with `Earlier`. This solves the lock of a draft by a rule change (ADR-0028).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuleSet {
    /// Every rule in force now.
    Current,
    /// The rules before ADR-0114 and ADR-0117 (ADR-0120 (a)).
    Earlier,
}

/// Return the range of the count of a type, or `None` when the type is not on the platform.
///
/// Only `DATE` on `S7_1200` and `S7_1500` differs between the two rule sets.
pub(super) fn range(ty: TimeType, rules: RuleSet) -> Option<RangeInclusive<i128>> {
    use PlcPlatform::{S7_300, S7_1200, S7_1500};
    let iec = !ty.platform().is_s7();
    match (ty.primitive(), ty.platform()) {
        (Primitive::Time, _) if iec => Some(TIME_UNSIGNED),
        (Primitive::Time, _) => Some(TIME_SIGNED),
        (Primitive::Ltime, _) if iec => Some(LTIME_UNSIGNED),
        (Primitive::Ltime, S7_1500) => Some(LTIME_SIGNED),
        (Primitive::S5Time, S7_1500 | S7_300) => Some(S5TIME),
        (Primitive::Date, _) if iec => Some(DATE_UNSIGNED),
        (Primitive::Date, S7_1200 | S7_1500) if rules == RuleSet::Earlier => Some(DATE_S7_EARLIER),
        (Primitive::Date, _) => Some(DATE_S7),
        (Primitive::Ldate, _) if iec => Some(LDATE),
        (Primitive::TimeOfDay, _) => Some(TIME_OF_DAY),
        (Primitive::Ltod, _) if iec => Some(LTOD),
        (Primitive::Ltod, S7_1500) => Some(LTOD),
        (Primitive::DateAndTime, _) if iec => Some(DATE_AND_TIME_UNSIGNED),
        (Primitive::DateAndTime, S7_1500 | S7_300) => Some(DATE_AND_TIME_S7),
        (Primitive::Ldt, _) if iec => Some(LDT_UNSIGNED),
        (Primitive::Ldt, S7_1500) => Some(LDT_S7),
        (Primitive::Dtl, S7_1200 | S7_1500) => Some(DTL),
        _ => None,
    }
}

/// Check that a count is inside the range of its type.
///
/// # Errors
///
/// Returns [`IecLiteralReason::NotOnPlatform`] when the type does not exist on the platform.
/// Returns [`IecLiteralReason::OutOfRange`] when the count is outside the range.
pub(super) fn check(ty: TimeType, count: i128, rules: RuleSet) -> Result<(), IecLiteralReason> {
    let allowed = range(ty, rules).ok_or(IecLiteralReason::NotOnPlatform)?;
    if allowed.contains(&count) {
        Ok(())
    } else {
        Err(IecLiteralReason::OutOfRange)
    }
}

#[cfg(test)]
mod tests;
