//! Property tests of the IEC literals of the time types (`domain-model.md` section 2.1, A43).
//!
//! Every draw picks one time type that exists on a platform, and one count inside its range
//! that the type can show. The counts of S5TIME and LDATE follow their own rules.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::plant::{IecLiteral, PlcPlatform, Primitive, TimeType};

/// The ten time primitives of `domain-model.md` section 2.1.
const TIME_PRIMITIVES: [Primitive; 10] = [
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

/// The nanoseconds of one day. LDATE counts are whole days in nanoseconds.
const NANOS_PER_DAY: i128 = 86_400_000_000_000;

/// Return every time type that exists on a platform and has a range.
fn time_types() -> Vec<TimeType> {
    TIME_PRIMITIVES
        .into_iter()
        .flat_map(|primitive| {
            PlcPlatform::ALL
                .into_iter()
                .filter_map(move |platform| TimeType::of(primitive, platform))
        })
        .filter(|ty| ty.range().is_some())
        .collect()
}

/// Draw one S5TIME count that is a valid step of one of its four tiers.
fn s5time_step() -> BoxedStrategy<i128> {
    prop_oneof![
        (0..=999_i128)
            .prop_map(|k| k.checked_mul(10).unwrap())
            .boxed(),
        (100..=999_i128)
            .prop_map(|k| k.checked_mul(100).unwrap())
            .boxed(),
        (100..=999_i128)
            .prop_map(|k| k.checked_mul(1_000).unwrap())
            .boxed(),
        (100..=999_i128)
            .prop_map(|k| k.checked_mul(10_000).unwrap())
            .boxed(),
    ]
    .boxed()
}

/// Draw one count of a type inside its range, in the form the type can show.
fn count_of(ty: TimeType) -> BoxedStrategy<i128> {
    let (low, high) = ty.range().unwrap_or((0, 0));
    match ty.primitive() {
        Primitive::S5Time => s5time_step(),
        Primitive::Ldate => ((low / NANOS_PER_DAY)..=(high / NANOS_PER_DAY))
            .prop_map(|days| days.checked_mul(NANOS_PER_DAY).unwrap())
            .boxed(),
        _ => (low..=high).boxed(),
    }
}

/// Draw one time type and one count of that type.
fn type_and_count() -> impl Strategy<Value = (TimeType, i128)> {
    prop::sample::select(time_types())
        .prop_flat_map(|ty| count_of(ty).prop_map(move |count| (ty, count)))
}

/// Build the literal of a stored count, and name the type and the count on failure.
fn stored(ty: TimeType, count: i128) -> Result<IecLiteral, TestCaseError> {
    IecLiteral::try_from((ty, count))
        .map_err(|reason| TestCaseError::fail(format!("{ty:?} {count}: {reason}")))
}

/// Parse a text of the type, and name the text on failure.
fn parsed(ty: TimeType, text: &str) -> Result<IecLiteral, TestCaseError> {
    IecLiteral::parse(ty, text).map_err(|reason| TestCaseError::fail(format!("{text}: {reason}")))
}

proptest! {
    /// A count inside the range survives `Display` and `parse`.
    #[test]
    fn stored_count_survives_display_and_parse(pair in type_and_count()) {
        let (ty, count) = pair;
        let text = stored(ty, count)?.to_string();
        let back = parsed(ty, &text)?;
        prop_assert_eq!(back.count(), count, "{}", text);
    }

    /// A canonical literal survives `parse` and `Display` unchanged.
    #[test]
    fn canonical_text_survives_parse_and_display(pair in type_and_count()) {
        let (ty, count) = pair;
        let text = stored(ty, count)?.to_string();
        let shown = parsed(ty, &text)?.to_string();
        prop_assert_eq!(shown, text);
    }
}
