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
/// `DATE` on S7-1200 and S7-1500: 1990-01-01 to 2169-06-06, in days (ADR-0026).
const DATE_S7: RangeInclusive<i128> = 7_305..=72_840;
/// `DATE` on S7-300: 1990-01-01 to 2168-12-31, in days (ADR-0026).
const DATE_S7_300: RangeInclusive<i128> = 7_305..=72_683;
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

/// Return the range of the count of a type, or `None` when the type is not on the platform.
pub(super) fn range(ty: TimeType) -> Option<RangeInclusive<i128>> {
    use PlcPlatform::{CodesysIec, S7_300, S7_1200, S7_1500, TwinCat3};
    let iec = matches!(ty.platform(), CodesysIec | TwinCat3);
    match (ty.primitive(), ty.platform()) {
        (Primitive::Time, _) if iec => Some(TIME_UNSIGNED),
        (Primitive::Time, _) => Some(TIME_SIGNED),
        (Primitive::Ltime, _) if iec => Some(LTIME_UNSIGNED),
        (Primitive::Ltime, S7_1500) => Some(LTIME_SIGNED),
        (Primitive::S5Time, S7_1500 | S7_300) => Some(S5TIME),
        (Primitive::Date, _) if iec => Some(DATE_UNSIGNED),
        (Primitive::Date, S7_300) => Some(DATE_S7_300),
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
pub(super) fn check(ty: TimeType, count: i128) -> Result<(), IecLiteralReason> {
    let allowed = range(ty).ok_or(IecLiteralReason::NotOnPlatform)?;
    if allowed.contains(&count) {
        Ok(())
    } else {
        Err(IecLiteralReason::OutOfRange)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plant::iec_literal::IecLiteral;
    use crate::plant::iec_literal::calendar::days_from_civil;
    use crate::plant::{PlcPlatform, Primitive};

    const U64_MAX: i128 = 18_446_744_073_709_551_615;
    const I63_MAX: i128 = 9_223_372_036_854_775_807;
    const DAY_NS: i128 = 86_400_000_000_000;
    const ALL: [PlcPlatform; 5] = [
        PlcPlatform::CodesysIec,
        PlcPlatform::TwinCat3,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
        PlcPlatform::S7_300,
    ];
    const IEC: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];
    const S7: [PlcPlatform; 3] = [
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
        PlcPlatform::S7_300,
    ];

    fn ty(primitive: Primitive, platform: PlcPlatform) -> TimeType {
        TimeType::of(primitive, platform).unwrap()
    }

    fn try_count(
        primitive: Primitive,
        platform: PlcPlatform,
        count: i128,
    ) -> Result<(), IecLiteralReason> {
        IecLiteral::try_from((ty(primitive, platform), count)).map(|_| ())
    }

    /// Assert both ends pass and one step outside each end fails, on each platform.
    fn assert_range(
        primitive: Primitive,
        platforms: &[PlcPlatform],
        low: i128,
        high: i128,
        step: i128,
    ) {
        for &platform in platforms {
            let t = ty(primitive, platform);
            let at = format!("{primitive:?} {platform:?}");
            assert_eq!(t.range(), Some((low, high)), "{at}");
            assert_eq!(try_count(primitive, platform, low), Ok(()), "low {at}");
            assert_eq!(try_count(primitive, platform, high), Ok(()), "high {at}");
            assert_eq!(
                try_count(primitive, platform, low.checked_sub(step).unwrap()),
                Err(IecLiteralReason::OutOfRange),
                "below {at}"
            );
            assert_eq!(
                try_count(primitive, platform, high.checked_add(step).unwrap()),
                Err(IecLiteralReason::OutOfRange),
                "above {at}"
            );
        }
    }

    fn assert_absent(primitive: Primitive, platforms: &[PlcPlatform]) {
        for &platform in platforms {
            let at = format!("{primitive:?} {platform:?}");
            assert_eq!(ty(primitive, platform).range(), None, "{at}");
            assert_eq!(
                try_count(primitive, platform, 0),
                Err(IecLiteralReason::NotOnPlatform),
                "{at}"
            );
        }
    }

    fn parse(
        primitive: Primitive,
        platform: PlcPlatform,
        text: &str,
    ) -> Result<i128, IecLiteralReason> {
        IecLiteral::parse(ty(primitive, platform), text).map(|literal| literal.count())
    }

    #[test]
    fn time_range_per_platform() {
        assert_range(Primitive::Time, &IEC, 0, 4_294_967_295, 1);
        assert_range(Primitive::Time, &S7, -2_147_483_648, 2_147_483_647, 1);
    }

    #[test]
    fn ltime_range_per_platform() {
        assert_range(Primitive::Ltime, &IEC, 0, U64_MAX, 1);
        assert_range(
            Primitive::Ltime,
            &[PlcPlatform::S7_1500],
            -I63_MAX - 1,
            I63_MAX,
            1,
        );
        assert_absent(
            Primitive::Ltime,
            &[PlcPlatform::S7_1200, PlcPlatform::S7_300],
        );
    }

    #[test]
    fn s5time_range_exists_on_s7_1500_and_s7_300_only() {
        for platform in [PlcPlatform::S7_1500, PlcPlatform::S7_300] {
            let p = Primitive::S5Time;
            assert_eq!(ty(p, platform).range(), Some((0, 9_990_000)));
            assert_eq!(try_count(p, platform, 0), Ok(()));
            assert_eq!(try_count(p, platform, 9_990_000), Ok(()));
            assert_eq!(
                try_count(p, platform, 10_000_000),
                Err(IecLiteralReason::OutOfRange)
            );
            assert_eq!(
                try_count(p, platform, -10),
                Err(IecLiteralReason::OutOfRange)
            );
        }
        assert_absent(
            Primitive::S5Time,
            &[
                PlcPlatform::CodesysIec,
                PlcPlatform::TwinCat3,
                PlcPlatform::S7_1200,
            ],
        );
    }

    #[test]
    fn date_range_per_platform() {
        assert_range(Primitive::Date, &IEC, 0, 49_710, 1);
        assert_range(
            Primitive::Date,
            &[PlcPlatform::S7_1200, PlcPlatform::S7_1500],
            7_305,
            72_840,
            1,
        );
        assert_range(Primitive::Date, &[PlcPlatform::S7_300], 7_305, 72_683, 1);
    }

    #[test]
    fn date_numbers_match_the_calendar_dates_of_adr_0026() {
        assert_eq!(days_from_civil(1970, 1, 1), Some(0));
        assert_eq!(days_from_civil(2106, 2, 7), Some(49_710));
        assert_eq!(days_from_civil(1990, 1, 1), Some(7_305));
        assert_eq!(days_from_civil(2169, 6, 6), Some(72_840));
        assert_eq!(days_from_civil(2168, 12, 31), Some(72_683));
    }

    #[test]
    fn date_and_time_numbers_match_the_calendar_dates_of_adr_0026() {
        let seconds = |y, m, d, rest: i64| days_from_civil(y, m, d).unwrap() * 86_400 + rest;
        assert_eq!(seconds(1990, 1, 1, 0), 631_152_000);
        assert_eq!(seconds(2089, 12, 31, 86_399), 3_786_911_999);
        assert_eq!(seconds(2106, 2, 7, 6 * 3_600 + 28 * 60 + 15), 4_294_967_295);
    }

    #[test]
    fn dtl_upper_end_is_in_2262_04_11() {
        let day = days_from_civil(2262, 4, 11).unwrap();
        let start = i128::from(day) * DAY_NS;
        assert!(start <= I63_MAX && I63_MAX < start + DAY_NS);
    }

    #[test]
    fn ldate_range_per_platform() {
        for &platform in &IEC {
            let p = Primitive::Ldate;
            assert_eq!(ty(p, platform).range(), Some((0, U64_MAX)));
            assert_eq!(try_count(p, platform, 0), Ok(()));
            assert_eq!(
                try_count(p, platform, -DAY_NS),
                Err(IecLiteralReason::OutOfRange)
            );
        }
        assert_absent(Primitive::Ldate, &S7);
    }

    #[test]
    fn time_of_day_range_is_the_same_on_every_platform() {
        for &platform in &ALL {
            let p = Primitive::TimeOfDay;
            assert_eq!(ty(p, platform).range(), Some((0, 86_399_999)));
            assert_eq!(try_count(p, platform, 0), Ok(()));
            assert_eq!(try_count(p, platform, 86_399_999), Ok(()));
            assert_eq!(
                try_count(p, platform, 86_400_000),
                Err(IecLiteralReason::OutOfRange)
            );
            assert_eq!(
                try_count(p, platform, -1),
                Err(IecLiteralReason::OutOfRange)
            );
        }
    }

    #[test]
    fn ltod_range_is_below_one_day_where_it_exists() {
        assert_range(Primitive::Ltod, &IEC, 0, 86_399_999_999_999, 1);
        assert_range(
            Primitive::Ltod,
            &[PlcPlatform::S7_1500],
            0,
            86_399_999_999_999,
            1,
        );
        assert_absent(
            Primitive::Ltod,
            &[PlcPlatform::S7_1200, PlcPlatform::S7_300],
        );
    }

    #[test]
    fn date_and_time_range_per_platform() {
        assert_range(
            Primitive::DateAndTime,
            &[PlcPlatform::S7_1500, PlcPlatform::S7_300],
            631_152_000,
            3_786_911_999,
            1,
        );
        assert_range(Primitive::DateAndTime, &IEC, 0, 4_294_967_295, 1);
        // ADR-0026 and data-model.md section 7: no DATE_AND_TIME on S7-1200.
        assert_absent(Primitive::DateAndTime, &[PlcPlatform::S7_1200]);
    }

    #[test]
    fn ldt_range_per_platform() {
        let p = Primitive::Ldt;
        assert_eq!(ty(p, PlcPlatform::CodesysIec).range(), Some((0, U64_MAX)));
        assert_eq!(ty(p, PlcPlatform::TwinCat3).range(), Some((0, U64_MAX)));
        assert_eq!(ty(p, PlcPlatform::S7_1500).range(), Some((0, I63_MAX)));
        assert_eq!(try_count(p, PlcPlatform::S7_1500, 0), Ok(()));
        assert_eq!(try_count(p, PlcPlatform::S7_1500, I63_MAX), Ok(()));
        assert_eq!(
            try_count(p, PlcPlatform::S7_1500, -1),
            Err(IecLiteralReason::OutOfRange)
        );
        assert_eq!(
            try_count(p, PlcPlatform::TwinCat3, -1),
            Err(IecLiteralReason::OutOfRange)
        );
        assert_absent(p, &[PlcPlatform::S7_1200, PlcPlatform::S7_300]);
    }

    #[test]
    fn dtl_range_per_platform() {
        let p = Primitive::Dtl;
        for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
            assert_eq!(ty(p, platform).range(), Some((0, I63_MAX)));
            assert_eq!(try_count(p, platform, 0), Ok(()));
            assert_eq!(try_count(p, platform, I63_MAX), Ok(()));
            assert_eq!(
                try_count(p, platform, -1),
                Err(IecLiteralReason::OutOfRange)
            );
        }
        assert_absent(
            p,
            &[
                PlcPlatform::CodesysIec,
                PlcPlatform::TwinCat3,
                PlcPlatform::S7_300,
            ],
        );
    }

    #[test]
    fn negative_time_depends_on_the_platform() {
        let p = Primitive::Time;
        assert_eq!(
            parse(p, PlcPlatform::TwinCat3, "T#-1s"),
            Err(IecLiteralReason::OutOfRange)
        );
        assert_eq!(parse(p, PlcPlatform::S7_1500, "T#-1s"), Ok(-1000));
    }

    #[test]
    fn date_literals_at_the_s7_lower_end() {
        let p = Primitive::Date;
        assert_eq!(
            parse(p, PlcPlatform::S7_1500, "D#1989-12-31"),
            Err(IecLiteralReason::OutOfRange)
        );
        assert_eq!(parse(p, PlcPlatform::S7_1500, "D#1990-01-01"), Ok(7_305));
    }

    #[test]
    fn date_and_time_literals_at_the_s7_upper_end() {
        let p = Primitive::DateAndTime;
        assert_eq!(
            parse(p, PlcPlatform::S7_300, "DT#2089-12-31-23:59:59"),
            Ok(3_786_911_999)
        );
        assert_eq!(
            parse(p, PlcPlatform::S7_300, "DT#2090-01-01-00:00:00"),
            Err(IecLiteralReason::OutOfRange)
        );
    }

    #[test]
    fn literals_of_a_missing_type_are_not_on_the_platform() {
        assert_eq!(
            parse(Primitive::Ltime, PlcPlatform::S7_300, "LT#1s"),
            Err(IecLiteralReason::NotOnPlatform)
        );
        assert_eq!(
            parse(
                Primitive::Dtl,
                PlcPlatform::TwinCat3,
                "DTL#2026-10-12-12:30:00"
            ),
            Err(IecLiteralReason::NotOnPlatform)
        );
    }

    #[test]
    fn not_on_platform_display_is_a_short_phrase() {
        assert_eq!(
            IecLiteralReason::NotOnPlatform.to_string(),
            "the type does not exist on the platform"
        );
    }
}
