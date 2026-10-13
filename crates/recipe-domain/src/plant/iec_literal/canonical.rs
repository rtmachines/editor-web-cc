//! Canonical text of a duration literal, and the S5TIME steps (`domain-model.md` section 2.1,
//! base-unit table of Assumption A43).

use std::fmt;

use crate::plant::{LeafClass, Primitive};

use super::duration::{base_nanos, units_of};
use super::{IecLiteral, IecLiteralReason, TimeType};

/// Largest S5TIME count in milliseconds (`9 990 s`).
const S5TIME_MAX_MS: i128 = 9_990_000;

/// Check a stored count against the rules of its type, without a parsed text.
///
/// # Errors
///
/// Returns [`IecLiteralReason::Step`] or [`IecLiteralReason::OutOfRange`] for a count
/// that the literal cannot show.
pub(super) fn check_count(ty: TimeType, count: i128) -> Result<(), IecLiteralReason> {
    match ty.primitive() {
        Primitive::S5Time => check_s5time_step(count),
        Primitive::Time | Primitive::Ltime => Ok(()),
        _ => super::instant::check_count(ty, count),
    }
}

/// Check that an S5TIME count in milliseconds is one of its steps.
///
/// # Errors
///
/// Returns [`IecLiteralReason::Step`] between two steps, and
/// [`IecLiteralReason::OutOfRange`] above 9 990 000 ms or below zero.
pub(super) fn check_s5time_step(count: i128) -> Result<(), IecLiteralReason> {
    if !(0..=S5TIME_MAX_MS).contains(&count) {
        return Err(IecLiteralReason::OutOfRange);
    }
    let step = match count {
        0..=9_990 => 10,
        9_991..=99_900 => 100,
        99_901..=999_000 => 1_000,
        _ => 10_000,
    };
    if count.checked_rem(step) == Some(0) {
        Ok(())
    } else {
        Err(IecLiteralReason::Step)
    }
}

/// Return the prefix of a duration type, such as `LT#` for `LTIME`.
fn duration_prefix_text(primitive: Primitive) -> Option<&'static str> {
    match primitive {
        Primitive::Time => Some("T#"),
        Primitive::Ltime => Some("LT#"),
        Primitive::S5Time => Some("S5T#"),
        _ => None,
    }
}

impl TryFrom<(TimeType, i128)> for IecLiteral {
    type Error = IecLiteralReason;

    fn try_from((ty, count): (TimeType, i128)) -> Result<Self, Self::Error> {
        check_count(ty, count)?;
        super::range::check(ty, count)?;
        Ok(IecLiteral { ty, count })
    }
}

/// Render a duration literal. A count that its type cannot show gives `None`.
fn render_duration(literal: &IecLiteral) -> Option<String> {
    let primitive = literal.ty.primitive();
    let mut text = String::from(duration_prefix_text(primitive)?);
    if literal.count < 0 {
        text.push('-');
    }
    let mut remaining = literal
        .count
        .checked_abs()?
        .checked_mul(base_nanos(primitive).ok()?)?;
    let units = units_of(primitive);
    if remaining == 0 {
        text.push('0');
        text.push_str(units.last()?.letters());
        return Some(text);
    }
    for unit in units {
        let span = unit.nanos();
        let amount = remaining.checked_div(span)?;
        if amount > 0 {
            text.push_str(&amount.to_string());
            text.push_str(unit.letters());
        }
        remaining = remaining.checked_rem(span)?;
    }
    Some(text)
}

/// `Display` never returns an error, because `to_string` panics on one. A count that the
/// constructors reject cannot reach this code. If one did, the fallback is the plain count.
impl fmt::Display for IecLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = if self.ty.primitive().leaf_class() == LeafClass::Duration {
            render_duration(self)
        } else {
            super::instant::render_instant(self)
        };
        match text {
            Some(text) => f.write_str(&text),
            None => write!(f, "{}", self.count),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
