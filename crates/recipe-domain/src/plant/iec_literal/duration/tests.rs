use crate::plant::{IecLiteral, IecLiteralReason, PlcPlatform, Primitive, TimeType};

fn time_type(primitive: Primitive) -> TimeType {
    TimeType::of(primitive, PlcPlatform::S7_1500).expect("a time primitive")
}

fn count(primitive: Primitive, text: &str) -> Result<i128, IecLiteralReason> {
    IecLiteral::parse(time_type(primitive), text).map(|literal| literal.count())
}

#[test]
fn time_type_of_int_is_none() {
    assert_eq!(TimeType::of(Primitive::Int, PlcPlatform::TwinCat3), None);
}

#[test]
fn time_type_of_time_is_some() {
    let ty = time_type(Primitive::Time);
    assert_eq!(ty.primitive(), Primitive::Time);
    assert_eq!(ty.platform(), PlcPlatform::S7_1500);
}

#[test]
fn time_type_of_time_of_day_is_some_because_it_is_an_instant() {
    assert!(TimeType::of(Primitive::TimeOfDay, PlcPlatform::TwinCat3).is_some());
}

#[test]
fn one_day_two_hours_three_minutes_four_seconds_five_ms_is_93784005() {
    assert_eq!(count(Primitive::Time, "T#1d2h3m4s5ms"), Ok(93_784_005));
}

#[test]
fn five_ms_is_five() {
    assert_eq!(count(Primitive::Time, "T#5ms"), Ok(5));
}

#[test]
fn one_minute_five_seconds_is_65000() {
    assert_eq!(count(Primitive::Time, "T#1m5s"), Ok(65_000));
}

#[test]
fn ninety_minutes_is_unbounded_first_part() {
    assert_eq!(count(Primitive::Time, "T#90m"), Ok(5_400_000));
}

#[test]
fn negative_one_second_is_minus_one_thousand() {
    assert_eq!(count(Primitive::Time, "T#-1s"), Ok(-1000));
}

#[test]
fn ltime_one_second_is_one_billion_nanoseconds() {
    assert_eq!(count(Primitive::Ltime, "LT#1s"), Ok(1_000_000_000));
}

#[test]
fn ltime_one_us_five_ns_is_1005() {
    assert_eq!(count(Primitive::Ltime, "LT#1us5ns"), Ok(1005));
}

#[test]
fn s5time_two_hours_is_7200000() {
    assert_eq!(count(Primitive::S5Time, "S5T#2h"), Ok(7_200_000));
}

#[test]
fn unit_order_wrong_is_unit_error() {
    assert_eq!(
        count(Primitive::Time, "T#2h1d"),
        Err(IecLiteralReason::Unit)
    );
}

#[test]
fn repeated_unit_is_unit_error() {
    assert_eq!(
        count(Primitive::Time, "T#1h1h"),
        Err(IecLiteralReason::Unit)
    );
}

#[test]
fn microseconds_are_not_accepted_by_time() {
    assert_eq!(count(Primitive::Time, "T#1us"), Err(IecLiteralReason::Unit));
}

#[test]
fn days_are_not_accepted_by_s5time() {
    assert_eq!(
        count(Primitive::S5Time, "S5T#1d"),
        Err(IecLiteralReason::Unit)
    );
}

#[test]
fn sign_on_s5time_is_sign_error() {
    assert_eq!(
        count(Primitive::S5Time, "S5T#-1s"),
        Err(IecLiteralReason::Sign)
    );
}

#[test]
fn sixty_minutes_after_hours_is_part_error() {
    assert_eq!(
        count(Primitive::Time, "T#1h60m"),
        Err(IecLiteralReason::Part)
    );
}

#[test]
fn fourteen_hundred_forty_minutes_after_days_is_part_error() {
    assert_eq!(
        count(Primitive::Time, "T#1d1440m"),
        Err(IecLiteralReason::Part)
    );
}

#[test]
fn one_thousand_nine_hundred_thirty_nine_minutes_after_days_passes() {
    assert_eq!(count(Primitive::Time, "T#1d1439m"), Ok(172_740_000));
}

#[test]
fn one_thousand_ms_after_seconds_is_part_error() {
    assert_eq!(
        count(Primitive::Time, "T#1s1000ms"),
        Err(IecLiteralReason::Part)
    );
}

#[test]
fn ltime_prefix_on_time_type_is_prefix_error() {
    assert_eq!(
        count(Primitive::Time, "LT#5s"),
        Err(IecLiteralReason::Prefix)
    );
}

#[test]
fn empty_body_is_grammar_error() {
    assert_eq!(count(Primitive::Time, "T#"), Err(IecLiteralReason::Grammar));
}

#[test]
fn digits_without_unit_are_grammar_error() {
    assert_eq!(
        count(Primitive::Time, "T#5"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn unit_without_digits_is_grammar_error() {
    assert_eq!(
        count(Primitive::Time, "T#ms"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn unknown_unit_is_grammar_error() {
    assert_eq!(
        count(Primitive::Time, "T#5x"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn space_after_prefix_is_grammar_error() {
    assert_eq!(
        count(Primitive::Time, "T# 5s"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn lower_case_prefix_is_grammar_error() {
    assert_eq!(
        count(Primitive::Time, "t#5s"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn twenty_digits_are_grammar_error() {
    assert_eq!(
        count(Primitive::Time, "T#12345678901234567890s"),
        Err(IecLiteralReason::Grammar)
    );
}

#[test]
fn reason_display_is_a_short_phrase() {
    assert_eq!(
        IecLiteralReason::Unit.to_string(),
        "a unit is not accepted, repeated, or out of order"
    );
}

// domain-model.md section 2.1: every later part is less than one unit of the part before it.
#[test]
fn part_text_names_the_rule_of_the_part_before() {
    let reason = count(Primitive::Time, "T#2h70m").expect_err("70 minutes is not below one hour");
    assert_eq!(reason, IecLiteralReason::Part);
    assert!(reason.to_string().contains("one unit of"), "{reason}");
}

fn assert_fragment(text: &str) {
    assert!(
        text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "{text}"
    );
    assert!(!text.ends_with('.'), "{text}");
}

// domain-model.md section 2.1: each of the ten reasons gives its own fragment.
#[test]
fn reason_texts_are_distinct_lower_case_fragments_without_a_final_period() {
    let reasons = [
        IecLiteralReason::Grammar,
        IecLiteralReason::Prefix,
        IecLiteralReason::Unit,
        IecLiteralReason::Sign,
        IecLiteralReason::Part,
        IecLiteralReason::Fraction,
        IecLiteralReason::Calendar,
        IecLiteralReason::Step,
        IecLiteralReason::OutOfRange,
        IecLiteralReason::NotOnPlatform,
    ];
    let texts: Vec<String> = reasons.iter().map(ToString::to_string).collect();
    for text in &texts {
        assert_fragment(text);
    }
    let distinct: std::collections::BTreeSet<&String> = texts.iter().collect();
    assert_eq!(distinct.len(), reasons.len());
}

// domain-model.md section 2.1: a part has at most 19 digits (`digits := [0-9]{1,19}`).
#[test]
fn nineteen_digits_in_one_part_are_accepted() {
    assert_eq!(count(Primitive::Time, "T#0000000000000000001ms"), Ok(1));
}
