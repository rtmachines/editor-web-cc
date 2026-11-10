use crate::plant::{IecLiteral, IecLiteralReason, PlcPlatform, Primitive, TimeType};

fn time_type(primitive: Primitive) -> TimeType {
    // DTL exists only on S7-1200 and S7-1500 (ADR-0026 table, `range.rs`).
    let platform = match primitive {
        Primitive::Dtl => PlcPlatform::S7_1500,
        _ => PlcPlatform::TwinCat3,
    };
    TimeType::of(primitive, platform).expect("an instant primitive")
}

fn literal(primitive: Primitive, text: &str) -> Result<IecLiteral, IecLiteralReason> {
    IecLiteral::parse(time_type(primitive), text)
}

fn count(primitive: Primitive, text: &str) -> Result<i128, IecLiteralReason> {
    literal(primitive, text).map(|value| value.count())
}

fn shown(primitive: Primitive, text: &str) -> String {
    literal(primitive, text)
        .expect("a valid literal")
        .to_string()
}

const DAY_NANOS: i128 = 86_400_000_000_000;

#[test]
fn tod_half_second_counts_45000500_ms() {
    assert_eq!(
        count(Primitive::TimeOfDay, "TOD#12:30:00.500"),
        Ok(45_000_500)
    );
}

#[test]
fn tod_half_second_displays_without_trailing_zeros() {
    assert_eq!(
        shown(Primitive::TimeOfDay, "TOD#12:30:00.500"),
        "TOD#12:30:00.5"
    );
}

#[test]
fn date_october_twelfth_2026_counts_20738_days() {
    assert_eq!(count(Primitive::Date, "D#2026-10-12"), Ok(20738));
}

#[test]
fn date_october_twelfth_2026_displays_its_canonical_literal() {
    assert_eq!(shown(Primitive::Date, "D#2026-10-12"), "D#2026-10-12");
}

#[test]
fn date_and_time_counts_seconds_since_epoch() {
    assert_eq!(
        count(Primitive::DateAndTime, "DT#2026-10-12-12:30:00"),
        Ok(1_791_808_200)
    );
}

#[test]
fn date_and_time_displays_its_canonical_literal() {
    assert_eq!(
        shown(Primitive::DateAndTime, "DT#2026-10-12-12:30:00"),
        "DT#2026-10-12-12:30:00"
    );
}

#[test]
fn ldate_counts_nanoseconds_of_20738_days() {
    assert_eq!(
        count(Primitive::Ldate, "LD#2026-10-12"),
        Ok(20738 * DAY_NANOS)
    );
}

#[test]
fn ltod_last_nanosecond_of_the_day_counts_86399999999999() {
    assert_eq!(
        count(Primitive::Ltod, "LTOD#23:59:59.999999999"),
        Ok(86_399_999_999_999)
    );
}

#[test]
fn ldt_first_nanosecond_after_epoch_counts_one() {
    assert_eq!(
        count(Primitive::Ldt, "LDT#1970-01-01-00:00:00.000000001"),
        Ok(1)
    );
}

#[test]
fn ldt_first_nanosecond_displays_nine_fraction_digits() {
    assert_eq!(
        shown(Primitive::Ldt, "LDT#2026-10-12-12:30:00.000000001"),
        "LDT#2026-10-12-12:30:00.000000001"
    );
}

#[test]
fn dtl_quarter_second_round_trips() {
    assert_eq!(
        shown(Primitive::Dtl, "DTL#2026-10-12-12:30:00.25"),
        "DTL#2026-10-12-12:30:00.25"
    );
}

#[test]
fn hour_twenty_four_is_a_calendar_error() {
    assert_eq!(
        count(Primitive::TimeOfDay, "TOD#24:00:00"),
        Err(IecLiteralReason::Calendar)
    );
}

#[test]
fn minute_sixty_is_a_calendar_error() {
    assert_eq!(
        count(Primitive::TimeOfDay, "TOD#12:60:00"),
        Err(IecLiteralReason::Calendar)
    );
}

#[test]
fn leap_day_2026_is_a_calendar_error() {
    assert_eq!(
        count(Primitive::Date, "D#2026-02-29"),
        Err(IecLiteralReason::Calendar)
    );
}

#[test]
fn four_fraction_digits_on_tod_are_a_fraction_error() {
    assert_eq!(
        count(Primitive::TimeOfDay, "TOD#12:30:00.5000"),
        Err(IecLiteralReason::Fraction)
    );
}

#[test]
fn fraction_on_date_and_time_is_a_fraction_error() {
    assert_eq!(
        count(Primitive::DateAndTime, "DT#2026-10-12-12:30:00.1"),
        Err(IecLiteralReason::Fraction)
    );
}

#[test]
fn ten_fraction_digits_on_ltod_are_a_grammar_error() {
    assert_eq!(
        count(Primitive::Ltod, "LTOD#00:00:00.1234567890"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn date_prefix_on_tod_type_is_a_prefix_error() {
    assert_eq!(
        count(Primitive::TimeOfDay, "D#2026-10-12"),
        Err(IecLiteralReason::Prefix)
    );
}

#[test]
fn short_year_is_a_grammar_error() {
    assert_eq!(
        count(Primitive::Date, "D#26-10-12"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn one_digit_hour_is_a_grammar_error() {
    assert_eq!(
        count(Primitive::TimeOfDay, "TOD#1:30:00"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn space_instead_of_dash_is_a_grammar_error() {
    assert_eq!(
        count(Primitive::DateAndTime, "DT#2026-10-12 12:30:00"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn ldate_count_of_one_nanosecond_is_out_of_range() {
    assert_eq!(
        IecLiteral::try_from((time_type(Primitive::Ldate), 1)),
        Err(IecLiteralReason::OutOfRange)
    );
}

#[test]
fn tod_count_of_one_day_is_out_of_range() {
    assert_eq!(
        IecLiteral::try_from((time_type(Primitive::TimeOfDay), 86_400_000)),
        Err(IecLiteralReason::OutOfRange)
    );
}

#[test]
fn stored_date_round_trips_through_display_and_parse() {
    let value = IecLiteral::try_from((time_type(Primitive::Date), 20738)).expect("a valid count");
    assert_eq!(value.to_string(), "D#2026-10-12");
    assert_eq!(literal(Primitive::Date, "D#2026-10-12"), Ok(value));
}

#[test]
fn time_of_day_without_seconds_is_grammar_error() {
    // domain-model.md section 2.1: `tod := [0-9]{2} ":" [0-9]{2} ":" [0-9]{2} [fraction]`.
    for text in ["TOD#12:30", "TOD#12:30:00:00"] {
        assert_eq!(
            count(Primitive::TimeOfDay, text),
            Err(IecLiteralReason::Grammar),
            "{text}"
        );
    }
}

// domain-model.md section 2.1: a date has no fraction. The reason is `Fraction`, not `Grammar`.
#[test]
fn fraction_on_date_is_a_fraction_error() {
    assert_eq!(
        count(Primitive::Date, "D#2026-10-12.5"),
        Err(IecLiteralReason::Fraction)
    );
}

// domain-model.md section 2.1: April has 30 days.
#[test]
fn april_thirty_first_is_a_calendar_error() {
    assert_eq!(
        count(Primitive::Date, "D#2024-04-31"),
        Err(IecLiteralReason::Calendar)
    );
}

// domain-model.md section 2.1: the date grammar is `YYYY-MM-DD` with two digits each.
#[test]
fn one_digit_month_is_a_grammar_error() {
    assert_eq!(
        count(Primitive::Date, "D#2026-1-12"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn one_digit_day_is_a_grammar_error() {
    assert_eq!(
        count(Primitive::Date, "D#2026-10-1"),
        Err(IecLiteralReason::Grammar)
    );
}

// domain-model.md section 2.1: the time grammar is `HH:MM:SS` with two digits each.
#[test]
fn one_digit_minute_is_a_grammar_error() {
    assert_eq!(
        count(Primitive::TimeOfDay, "TOD#12:3:00"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn one_digit_second_is_a_grammar_error() {
    assert_eq!(
        count(Primitive::TimeOfDay, "TOD#12:30:0"),
        Err(IecLiteralReason::Grammar)
    );
}

// domain-model.md section 2.1: a fraction holds digits only. A plus sign is not a digit.
#[test]
fn signed_fraction_is_a_grammar_error() {
    assert_eq!(
        count(Primitive::TimeOfDay, "TOD#12:30:00.+5"),
        Err(IecLiteralReason::Grammar)
    );
}
