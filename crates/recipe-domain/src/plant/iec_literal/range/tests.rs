use super::*;
use crate::plant::iec_literal::IecLiteral;
use crate::plant::iec_literal::calendar::days_from_civil;
use crate::plant::{PlcPlatform, Primitive};

const U64_MAX: i128 = 18_446_744_073_709_551_615;
const I63_MAX: i128 = 9_223_372_036_854_775_807;
const DAY_NS: i128 = 86_400_000_000_000;
/// 2262-04-11 in nanoseconds: the last whole day of `LDATE` (ADR-0139 (b)).
const LDATE_LAST_DAY_NS: i128 = 106_751 * DAY_NS;
/// 2262-04-12 in nanoseconds: the first whole day above the current `LDATE` range.
const LDATE_FIRST_DAY_AFTER_NS: i128 = 106_752 * DAY_NS;
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

fn count_under(
    primitive: Primitive,
    platform: PlcPlatform,
    count: i128,
    rules: RuleSet,
) -> Result<(), IecLiteralReason> {
    IecLiteral::from_count(ty(primitive, platform), count, rules).map(|_| ())
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
    assert_range(Primitive::Date, &S7, 7_305, 72_683, 1);
}

#[test]
fn date_numbers_match_the_calendar_dates_of_adr_0026() {
    assert_eq!(days_from_civil(1970, 1, 1), Some(0));
    assert_eq!(days_from_civil(2106, 2, 7), Some(49_710));
    assert_eq!(days_from_civil(1990, 1, 1), Some(7_305));
    assert_eq!(days_from_civil(2169, 6, 6), Some(72_840));
    assert_eq!(days_from_civil(2168, 12, 31), Some(72_683));
}

fn check_date(platform: PlcPlatform, count: i128, rules: RuleSet) -> Result<(), IecLiteralReason> {
    check(ty(Primitive::Date, platform), count, rules)
}

#[test]
fn adr_0114_current_set_ends_date_at_72_683_on_s7_1200_and_s7_1500() {
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        assert_eq!(check_date(platform, 72_683, RuleSet::Current), Ok(()));
        assert_eq!(
            check_date(platform, 72_684, RuleSet::Current),
            Err(IecLiteralReason::OutOfRange)
        );
    }
}

#[test]
fn adr_0120_earlier_set_ends_date_at_72_840_on_s7_1200_and_s7_1500() {
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        assert_eq!(check_date(platform, 72_840, RuleSet::Earlier), Ok(()));
        assert_eq!(
            check_date(platform, 72_841, RuleSet::Earlier),
            Err(IecLiteralReason::OutOfRange)
        );
        assert_eq!(check_date(platform, 7_305, RuleSet::Earlier), Ok(()));
        assert_eq!(
            check_date(platform, 7_304, RuleSet::Earlier),
            Err(IecLiteralReason::OutOfRange)
        );
    }
}

#[test]
fn adr_0120_s7_300_date_ends_at_72_683_in_both_sets() {
    for rules in [RuleSet::Current, RuleSet::Earlier] {
        assert_eq!(check_date(PlcPlatform::S7_300, 72_683, rules), Ok(()));
        assert_eq!(
            check_date(PlcPlatform::S7_300, 72_684, rules),
            Err(IecLiteralReason::OutOfRange)
        );
    }
}

#[test]
fn adr_0120_and_adr_0139_the_sets_differ_only_for_date_ldate_and_ldt() {
    let primitives = [
        Primitive::Time,
        Primitive::Ltime,
        Primitive::S5Time,
        Primitive::Date,
        Primitive::Ldate,
        Primitive::TimeOfDay,
        Primitive::Ltod,
        Primitive::DateAndTime,
        Primitive::Ldt,
        Primitive::Dtl,
    ];
    for primitive in primitives {
        for platform in ALL {
            let Some(t) = TimeType::of(primitive, platform) else {
                continue;
            };
            let differs = range(t, RuleSet::Current) != range(t, RuleSet::Earlier);
            let is_date_on_s7 = primitive == Primitive::Date
                && matches!(platform, PlcPlatform::S7_1200 | PlcPlatform::S7_1500);
            let is_ldate_or_ldt_on_iec = matches!(primitive, Primitive::Ldate | Primitive::Ldt)
                && matches!(platform, PlcPlatform::CodesysIec | PlcPlatform::TwinCat3);
            assert_eq!(
                differs,
                is_date_on_s7 || is_ldate_or_ldt_on_iec,
                "{primitive:?} {platform:?}"
            );
        }
    }
}

#[test]
fn adr_0114_the_public_range_and_the_literal_use_the_current_set() {
    let t = ty(Primitive::Date, PlcPlatform::S7_1500);
    assert_eq!(t.range(), Some((7_305, 72_683)));
    assert_eq!(
        parse(Primitive::Date, PlcPlatform::S7_1500, "D#2168-12-31"),
        Ok(72_683)
    );
    assert_eq!(
        parse(Primitive::Date, PlcPlatform::S7_1500, "D#2169-01-01"),
        Err(IecLiteralReason::OutOfRange)
    );
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
    let p = Primitive::Ldate;
    for &platform in &IEC {
        let t = ty(p, platform);
        assert_eq!(t.range(), Some((0, I63_MAX)), "{platform:?}");
        assert_eq!(
            t.range_under(RuleSet::Current),
            Some((0, I63_MAX)),
            "{platform:?}"
        );
        assert_eq!(
            t.range_under(RuleSet::Earlier),
            Some((0, U64_MAX)),
            "{platform:?}"
        );
        assert_eq!(try_count(p, platform, 0), Ok(()));
        assert_eq!(
            try_count(p, platform, -DAY_NS),
            Err(IecLiteralReason::OutOfRange)
        );
        // 2262-04-11 is the last whole day below 2^63 ns (ADR-0139 (b)).
        assert_eq!(try_count(p, platform, LDATE_LAST_DAY_NS), Ok(()));
        assert_eq!(
            try_count(p, platform, LDATE_FIRST_DAY_AFTER_NS),
            Err(IecLiteralReason::OutOfRange)
        );
        assert_eq!(
            count_under(p, platform, LDATE_FIRST_DAY_AFTER_NS, RuleSet::Earlier),
            Ok(())
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
    for platform in IEC {
        let t = ty(p, platform);
        assert_eq!(t.range(), Some((0, I63_MAX)), "{platform:?}");
        assert_eq!(
            t.range_under(RuleSet::Current),
            Some((0, I63_MAX)),
            "{platform:?}"
        );
        assert_eq!(
            t.range_under(RuleSet::Earlier),
            Some((0, U64_MAX)),
            "{platform:?}"
        );
        assert_eq!(try_count(p, platform, 0), Ok(()));
        assert_eq!(try_count(p, platform, I63_MAX), Ok(()));
        assert_eq!(
            try_count(p, platform, I63_MAX + 1),
            Err(IecLiteralReason::OutOfRange)
        );
        assert_eq!(
            try_count(p, platform, -1),
            Err(IecLiteralReason::OutOfRange)
        );
        assert_eq!(
            count_under(p, platform, I63_MAX + 1, RuleSet::Earlier),
            Ok(())
        );
        assert_eq!(count_under(p, platform, U64_MAX, RuleSet::Earlier), Ok(()));
        assert_eq!(
            count_under(p, platform, U64_MAX + 1, RuleSet::Earlier),
            Err(IecLiteralReason::OutOfRange)
        );
    }
    assert_eq!(ty(p, PlcPlatform::S7_1500).range(), Some((0, I63_MAX)));
    assert_eq!(try_count(p, PlcPlatform::S7_1500, 0), Ok(()));
    assert_eq!(try_count(p, PlcPlatform::S7_1500, I63_MAX), Ok(()));
    assert_eq!(
        try_count(p, PlcPlatform::S7_1500, -1),
        Err(IecLiteralReason::OutOfRange)
    );
    for rules in [RuleSet::Current, RuleSet::Earlier] {
        assert_eq!(
            count_under(p, PlcPlatform::S7_1500, I63_MAX + 1, rules),
            Err(IecLiteralReason::OutOfRange),
            "{rules:?}"
        );
    }
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
fn adr_0115_date_and_time_with_milliseconds_is_refused() {
    for platform in [PlcPlatform::S7_1500, PlcPlatform::S7_300] {
        assert_eq!(
            parse(
                Primitive::DateAndTime,
                platform,
                "DT#2026-10-12-12:30:00.500"
            ),
            Err(IecLiteralReason::Fraction),
            "{platform:?}"
        );
    }
    assert_eq!(
        parse(
            Primitive::DateAndTime,
            PlcPlatform::S7_1500,
            "DT#2026-10-12-12:30:00"
        ),
        Ok(1_791_808_200)
    );
}

#[test]
fn adr_0115_last_dtl_instant_is_accepted() {
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        assert_eq!(
            parse(
                Primitive::Dtl,
                platform,
                "DTL#2262-04-11-23:47:16.854775807"
            ),
            Ok(9_223_372_036_854_775_807),
            "{platform:?}"
        );
    }
}

#[test]
fn adr_0115_dtl_next_nanosecond_after_the_last_instant_is_refused() {
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        assert_eq!(
            parse(
                Primitive::Dtl,
                platform,
                "DTL#2262-04-11-23:47:16.854775808"
            ),
            Err(IecLiteralReason::OutOfRange),
            "{platform:?}"
        );
    }
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

#[test]
fn adr_0139_the_last_ldt_and_ldate_literals_pass_on_both_sets() {
    for platform in IEC {
        assert_eq!(
            parse(
                Primitive::Ldt,
                platform,
                "LDT#2262-04-11-23:47:16.854775807"
            ),
            Ok(I63_MAX),
            "{platform:?}"
        );
        assert_eq!(
            parse(Primitive::Ldate, platform, "LD#2262-04-11"),
            Ok(LDATE_LAST_DAY_NS),
            "{platform:?}"
        );
        for rules in [RuleSet::Current, RuleSet::Earlier] {
            assert_eq!(
                count_under(Primitive::Ldt, platform, I63_MAX, rules),
                Ok(()),
                "{platform:?} {rules:?}"
            );
            assert_eq!(
                count_under(Primitive::Ldate, platform, LDATE_LAST_DAY_NS, rules),
                Ok(()),
                "{platform:?} {rules:?}"
            );
        }
    }
}

#[test]
fn adr_0139_the_literal_after_the_last_ldt_and_ldate_is_out_of_range() {
    for platform in IEC {
        assert_eq!(
            parse(
                Primitive::Ldt,
                platform,
                "LDT#2262-04-11-23:47:16.854775808"
            ),
            Err(IecLiteralReason::OutOfRange),
            "{platform:?}"
        );
        assert_eq!(
            parse(Primitive::Ldate, platform, "LD#2262-04-12"),
            Err(IecLiteralReason::OutOfRange),
            "{platform:?}"
        );
    }
}

#[test]
fn adr_0139_the_ldt_literal_after_the_last_s7_1500_count_is_out_of_range() {
    assert_eq!(
        parse(
            Primitive::Ldt,
            PlcPlatform::S7_1500,
            "LDT#2262-04-11-23:47:16.854775808"
        ),
        Err(IecLiteralReason::OutOfRange)
    );
}

#[test]
fn adr_0139_the_earlier_set_keeps_the_old_end_of_ldt_and_ldate() {
    for platform in IEC {
        assert_eq!(
            count_under(Primitive::Ldt, platform, I63_MAX + 1, RuleSet::Earlier),
            Ok(()),
            "{platform:?}"
        );
        assert_eq!(
            count_under(Primitive::Ldt, platform, I63_MAX + 1, RuleSet::Current),
            Err(IecLiteralReason::OutOfRange),
            "{platform:?}"
        );
        assert_eq!(
            count_under(
                Primitive::Ldate,
                platform,
                LDATE_FIRST_DAY_AFTER_NS,
                RuleSet::Earlier
            ),
            Ok(()),
            "{platform:?}"
        );
        assert_eq!(
            count_under(
                Primitive::Ldate,
                platform,
                LDATE_FIRST_DAY_AFTER_NS,
                RuleSet::Current
            ),
            Err(IecLiteralReason::OutOfRange),
            "{platform:?}"
        );
    }
}

/// ADR-0115, review item R09: each literal at an end of the base unit table
/// (`domain-model.md` section 2.1) gives the exact count of the base unit.
/// One step outside each end gives `OutOfRange`. The ends hold on `CodesysIec`
/// and `TwinCat3`.
#[test]
fn adr_0115_iec_ends_of_time_literals() {
    // (type, literal at an end, count of the base unit)
    let at_ends: [(Primitive, &str, i128); 8] = [
        (Primitive::Time, "T#0ms", 0),
        (Primitive::Time, "T#49d17h2m47s295ms", 4_294_967_295),
        (Primitive::Ltime, "LT#0ns", 0),
        (
            Primitive::Ltime,
            "LT#213503d23h34m33s709ms551us615ns",
            U64_MAX,
        ),
        (Primitive::Date, "D#1970-01-01", 0),
        (Primitive::Date, "D#2106-02-07", 49_710),
        (Primitive::DateAndTime, "DT#1970-01-01-00:00:00", 0),
        (
            Primitive::DateAndTime,
            "DT#2106-02-07-06:28:15",
            4_294_967_295,
        ),
    ];
    // (type, literal one step outside an end)
    let outside: [(Primitive, &str); 8] = [
        (Primitive::Time, "T#-1ms"),
        (Primitive::Time, "T#49d17h2m47s296ms"),
        (Primitive::Ltime, "LT#-1ns"),
        (Primitive::Ltime, "LT#213503d23h34m33s709ms551us616ns"),
        (Primitive::Date, "D#1969-12-31"),
        (Primitive::Date, "D#2106-02-08"),
        (Primitive::DateAndTime, "DT#1969-12-31-23:59:59"),
        (Primitive::DateAndTime, "DT#2106-02-07-06:28:16"),
    ];
    for platform in IEC {
        for (primitive, text, count) in at_ends {
            assert_eq!(
                parse(primitive, platform, text),
                Ok(count),
                "{text} on {platform:?}"
            );
        }
        for (primitive, text) in outside {
            assert_eq!(
                parse(primitive, platform, text),
                Err(IecLiteralReason::OutOfRange),
                "{text} on {platform:?}"
            );
        }
    }
}

/// The 8 long prefixes of ADR-0205 (a), each with its short prefix, its type, and one body.
const LONG_PREFIXES: [(&str, &str, Primitive, &str); 8] = [
    ("TIME#", "T#", Primitive::Time, "1h30m"),
    ("LTIME#", "LT#", Primitive::Ltime, "1us"),
    ("DATE#", "D#", Primitive::Date, "2026-10-12"),
    ("LDATE#", "LD#", Primitive::Ldate, "2026-10-12"),
    ("TIME_OF_DAY#", "TOD#", Primitive::TimeOfDay, "12:30:00"),
    ("LTIME_OF_DAY#", "LTOD#", Primitive::Ltod, "12:30:00.5"),
    (
        "DATE_AND_TIME#",
        "DT#",
        Primitive::DateAndTime,
        "2026-10-12-12:30:00",
    ),
    (
        "LDATE_AND_TIME#",
        "LDT#",
        Primitive::Ldt,
        "2026-10-12-12:30:00.000000001",
    ),
];

#[test]
fn adr_0205_long_prefixes_give_the_short_count() {
    for platform in IEC {
        for (long, short, primitive, body) in LONG_PREFIXES {
            let at = format!("{long}{body} on {platform:?}");
            let long_text = format!("{long}{body}");
            let short_text = format!("{short}{body}");
            let from_long = IecLiteral::parse(ty(primitive, platform), &long_text).unwrap();
            let from_short = IecLiteral::parse(ty(primitive, platform), &short_text).unwrap();
            assert_eq!(from_long.count(), from_short.count(), "{at}");
            assert_eq!(from_long, from_short, "{at}");
            assert_eq!(from_long.to_string(), from_short.to_string(), "{at}");
            assert!(from_long.to_string().starts_with(short), "{at}");
        }
    }
}

#[test]
fn adr_0205_a_long_prefix_displays_the_short_prefix() {
    let literal =
        IecLiteral::parse(ty(Primitive::Ltime, PlcPlatform::CodesysIec), "LTIME#1us").unwrap();
    assert_eq!(literal.count(), 1_000);
    assert_eq!(literal.to_string(), "LT#1us");
}

#[test]
fn adr_0205_a_long_prefix_applies_on_every_platform_that_has_the_type() {
    for platform in S7 {
        assert_eq!(
            parse(Primitive::Time, platform, "TIME#1s"),
            parse(Primitive::Time, platform, "T#1s"),
            "{platform:?}"
        );
        assert_eq!(
            parse(Primitive::TimeOfDay, platform, "TIME_OF_DAY#12:30:00"),
            parse(Primitive::TimeOfDay, platform, "TOD#12:30:00"),
            "{platform:?}"
        );
    }
    assert_eq!(
        parse(Primitive::Ltime, PlcPlatform::S7_300, "LTIME#1s"),
        Err(IecLiteralReason::NotOnPlatform)
    );
}

#[test]
fn adr_0205_a_lower_case_long_prefix_is_a_grammar_error() {
    assert_eq!(
        parse(Primitive::Ltime, PlcPlatform::CodesysIec, "ltime#1us"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn adr_0205_s5time_has_no_long_prefix() {
    assert_eq!(
        parse(Primitive::S5Time, PlcPlatform::S7_1500, "S5TIME#1s"),
        Err(IecLiteralReason::Grammar)
    );
    assert_eq!(
        parse(Primitive::S5Time, PlcPlatform::S7_1500, "S5T#1s"),
        Ok(1_000)
    );
}

#[test]
fn adr_0205_dtl_has_no_long_prefix_and_date_and_time_is_not_its_alias() {
    assert_eq!(
        parse(
            Primitive::Dtl,
            PlcPlatform::S7_1500,
            "DATE_AND_TIME#2024-01-01-00:00:00"
        ),
        Err(IecLiteralReason::Prefix)
    );
    assert_eq!(
        parse(
            Primitive::Dtl,
            PlcPlatform::S7_1500,
            "DTL#2024-01-01-00:00:00"
        )
        .map(|_| ()),
        Ok(())
    );
}

#[test]
fn adr_0205_a_long_prefix_of_another_type_gives_the_prefix_reason() {
    assert_eq!(
        parse(Primitive::Time, PlcPlatform::CodesysIec, "LTIME#1us"),
        Err(IecLiteralReason::Prefix)
    );
}
