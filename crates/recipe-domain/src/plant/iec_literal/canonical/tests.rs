use crate::plant::{IecLiteral, IecLiteralReason, PlcPlatform, Primitive, TimeType};

fn time_type(primitive: Primitive) -> TimeType {
    TimeType::of(primitive, PlcPlatform::S7_1500).expect("a time primitive")
}

fn literal(primitive: Primitive, text: &str) -> Result<IecLiteral, IecLiteralReason> {
    IecLiteral::parse(time_type(primitive), text)
}

fn stored(primitive: Primitive, count: i128) -> Result<IecLiteral, IecLiteralReason> {
    IecLiteral::try_from((time_type(primitive), count))
}

fn shown(primitive: Primitive, text: &str) -> String {
    literal(primitive, text)
        .expect("a valid literal")
        .to_string()
}

/// Build a literal that no constructor can build, to reach the fallback of `Display`.
fn impossible(primitive: Primitive, count: i128) -> IecLiteral {
    IecLiteral {
        ty: time_type(primitive),
        count,
    }
}

#[test]
fn display_of_an_impossible_duration_writes_the_count() {
    let value = impossible(Primitive::Time, i128::MIN);
    assert_eq!(value.to_string(), i128::MIN.to_string());
}

#[test]
fn display_of_an_impossible_instant_writes_the_count() {
    let value = impossible(Primitive::Date, i128::MAX);
    assert_eq!(value.to_string(), i128::MAX.to_string());
    let value = impossible(Primitive::TimeOfDay, -1);
    assert_eq!(value.to_string(), "-1");
}

#[test]
fn ninety_minutes_shows_as_one_hour_thirty_minutes() {
    assert_eq!(shown(Primitive::Time, "T#90m"), "T#1h30m");
}

#[test]
fn stored_count_shows_every_part_from_days_to_ms() {
    let text = stored(Primitive::Time, 93_784_005).map(|value| value.to_string());
    assert_eq!(text, Ok("T#1d2h3m4s5ms".to_owned()));
}

#[test]
fn zero_time_shows_as_zero_ms() {
    assert_eq!(
        stored(Primitive::Time, 0).map(|v| v.to_string()),
        Ok("T#0ms".to_owned())
    );
}

#[test]
fn zero_ltime_shows_as_zero_ns() {
    assert_eq!(
        stored(Primitive::Ltime, 0).map(|v| v.to_string()),
        Ok("LT#0ns".to_owned())
    );
}

#[test]
fn zero_s5time_shows_as_zero_ms() {
    assert_eq!(
        stored(Primitive::S5Time, 0).map(|v| v.to_string()),
        Ok("S5T#0ms".to_owned())
    );
}

#[test]
fn negative_zero_shows_as_zero_ms() {
    assert_eq!(shown(Primitive::Time, "T#-0ms"), "T#0ms");
}

#[test]
fn negative_thousand_ms_shows_as_minus_one_second() {
    assert_eq!(
        stored(Primitive::Time, -1000).map(|v| v.to_string()),
        Ok("T#-1s".to_owned())
    );
}

#[test]
fn ltime_one_day_one_ns_round_trips() {
    assert_eq!(shown(Primitive::Ltime, "LT#1d1ns"), "LT#1d1ns");
}

#[test]
fn s5time_zero_is_valid() {
    assert!(stored(Primitive::S5Time, 0).is_ok());
}

#[test]
fn s5time_nine_seconds_990_ms_is_valid() {
    assert!(literal(Primitive::S5Time, "S5T#9s990ms").is_ok());
}

#[test]
fn s5time_12345_ms_is_a_step_error() {
    assert_eq!(
        literal(Primitive::S5Time, "S5T#12345ms"),
        Err(IecLiteralReason::Step)
    );
}

#[test]
fn s5time_12300_ms_is_valid_multiple_of_100() {
    assert!(literal(Primitive::S5Time, "S5T#12300ms").is_ok());
}

#[test]
fn s5time_maximum_is_valid() {
    assert!(literal(Primitive::S5Time, "S5T#2h46m30s").is_ok());
}

#[test]
fn s5time_above_maximum_is_out_of_range() {
    assert_eq!(
        literal(Primitive::S5Time, "S5T#2h46m40s"),
        Err(IecLiteralReason::OutOfRange)
    );
}

#[test]
fn stored_s5time_between_steps_is_a_step_error() {
    assert_eq!(
        stored(Primitive::S5Time, 12_345),
        Err(IecLiteralReason::Step)
    );
}

#[test]
fn stored_counts_round_trip_through_display_and_parse() {
    for (primitive, count) in [
        (Primitive::Time, 93_784_005),
        (Primitive::Time, 5),
        (Primitive::Time, 0),
        (Primitive::Time, -1000),
        (Primitive::Ltime, 1005),
        (Primitive::Ltime, 86_400_000_000_001),
        (Primitive::S5Time, 7_200_000),
        (Primitive::S5Time, 9_990_000),
    ] {
        let value = stored(primitive, count).expect("a valid count");
        let text = value.to_string();
        assert_eq!(literal(primitive, &text), Ok(value), "{text}");
        assert_eq!(value.count(), count, "{text}");
    }
}

/// S5TIME counts in milliseconds on a step at the tier edges, with their canonical text.
const S5_TIER_ACCEPTED: [(i128, &str); 6] = [
    (9_990, "S5T#9s990ms"),
    (10_000, "S5T#10s"),
    (99_900, "S5T#1m39s900ms"),
    (100_000, "S5T#1m40s"),
    (999_000, "S5T#16m39s"),
    (1_000_000, "S5T#16m40s"),
];

/// S5TIME counts one step of the lower tier above an edge, between two steps of the upper tier.
const S5_TIER_REFUSED: [(i128, &str); 3] = [
    (10_010, "S5T#10s10ms"),
    (100_100, "S5T#1m40s100ms"),
    (1_001_000, "S5T#16m41s"),
];

#[test]
fn from_count_accepts_the_s5time_counts_at_every_tier_edge() {
    for (count, text) in S5_TIER_ACCEPTED {
        let from_count = IecLiteral::from_count(
            time_type(Primitive::S5Time),
            count,
            crate::plant::RuleSet::Current,
        );
        let value = from_count.expect("a count on a step");
        assert_eq!(value.count(), count, "{count}");
        assert_eq!(value.to_string(), text, "{count}");
    }
}

#[test]
fn from_count_refuses_the_s5time_counts_between_steps_above_an_edge() {
    for (count, _) in S5_TIER_REFUSED {
        let from_count = IecLiteral::from_count(
            time_type(Primitive::S5Time),
            count,
            crate::plant::RuleSet::Current,
        );
        assert_eq!(from_count, Err(IecLiteralReason::Step), "{count}");
    }
}

#[test]
fn parse_accepts_the_s5time_literals_at_every_tier_edge() {
    for (count, text) in S5_TIER_ACCEPTED {
        let value = literal(Primitive::S5Time, text).expect("a literal on a step");
        assert_eq!(value.count(), count, "{text}");
    }
}

#[test]
fn parse_refuses_the_s5time_literals_between_steps_above_an_edge() {
    for (_, text) in S5_TIER_REFUSED {
        assert_eq!(
            literal(Primitive::S5Time, text),
            Err(IecLiteralReason::Step),
            "{text}"
        );
    }
}

/// DATE counts days since 1970-01-01 (A43). Count 2 932 897 is 10000-01-01.
const FIRST_DAY_OF_YEAR_10000: i128 = 2_932_897;

#[test]
fn time_type_of_a_literal_keeps_the_type_of_its_prefix() {
    // domain-model.md section 2.1 IecLiteral: the prefix `LT#` names `LTIME`.
    let ty = time_type(Primitive::Ltime);
    assert_eq!(
        literal(Primitive::Ltime, "LT#5ns").map(|value| value.time_type()),
        Ok(ty)
    );
}

#[test]
fn stored_date_in_year_10000_is_out_of_range() {
    // A43: a DATE count beyond year 9999 is outside the range of the type.
    assert_eq!(
        stored(Primitive::Date, FIRST_DAY_OF_YEAR_10000),
        Err(IecLiteralReason::OutOfRange)
    );
}

#[test]
fn display_of_an_impossible_date_in_year_10000_writes_the_count() {
    // Fault injection: no constructor builds this literal (`Display` fallback, `canonical.rs`).
    let value = impossible(Primitive::Date, FIRST_DAY_OF_YEAR_10000);
    assert_eq!(value.to_string(), FIRST_DAY_OF_YEAR_10000.to_string());
}

/// Build a literal of a type that is not a time type, to reach the arms that no constructor
/// reaches.
fn not_a_time_type(primitive: Primitive) -> TimeType {
    TimeType {
        primitive,
        platform: PlcPlatform::S7_1500,
    }
}

#[test]
fn display_of_a_literal_of_a_non_time_primitive_writes_the_count() {
    // Fault injection: `TimeType::of` never builds this type (`instant.rs` `spec_of`).
    let value = IecLiteral {
        ty: not_a_time_type(Primitive::Int),
        count: 42,
    };
    assert_eq!(value.to_string(), "42");
}

#[test]
fn a_stored_count_of_a_non_time_primitive_is_out_of_range() {
    // Fault injection: `TimeType::of` never builds this type (`instant.rs` `split_count`).
    assert_eq!(
        IecLiteral::try_from((not_a_time_type(Primitive::Int), 0)),
        Err(IecLiteralReason::OutOfRange)
    );
}
