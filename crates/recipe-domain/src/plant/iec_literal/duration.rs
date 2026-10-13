//! Duration literals `T#`, `LT#`, and `S5T#` (`domain-model.md` section 2.1).

use crate::plant::Primitive;

use super::{IecLiteral, IecLiteralReason, TimeType, canonical};

/// Longest run of digits in one part (`digits := [0-9]{1,19}`).
const MAX_DIGITS: usize = 19;

/// Nanoseconds in one millisecond, the base unit of `TIME` and `S5TIME`.
const NANOS_PER_MILLISECOND: i128 = 1_000_000;

/// Units that `TIME` accepts, from the largest to the smallest.
const TIME_UNITS: &[Unit] = &[
    Unit::Day,
    Unit::Hour,
    Unit::Minute,
    Unit::Second,
    Unit::Millisecond,
];

/// Units that `S5TIME` accepts, from the largest to the smallest.
const S5TIME_UNITS: &[Unit] = &[Unit::Hour, Unit::Minute, Unit::Second, Unit::Millisecond];

/// Units that `LTIME` accepts, from the largest to the smallest.
const LTIME_UNITS: &[Unit] = &[
    Unit::Day,
    Unit::Hour,
    Unit::Minute,
    Unit::Second,
    Unit::Millisecond,
    Unit::Microsecond,
    Unit::Nanosecond,
];

/// A unit of a duration part, in the order of the grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Unit {
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
}

impl Unit {
    /// Parse the letters of a unit, such as `ms`.
    fn parse(letters: &str) -> Option<Self> {
        match letters {
            "d" => Some(Self::Day),
            "h" => Some(Self::Hour),
            "m" => Some(Self::Minute),
            "s" => Some(Self::Second),
            "ms" => Some(Self::Millisecond),
            "us" => Some(Self::Microsecond),
            "ns" => Some(Self::Nanosecond),
            _ => None,
        }
    }

    /// Return the letters of this unit, such as `ms`.
    pub(super) const fn letters(self) -> &'static str {
        match self {
            Self::Day => "d",
            Self::Hour => "h",
            Self::Minute => "m",
            Self::Second => "s",
            Self::Millisecond => "ms",
            Self::Microsecond => "us",
            Self::Nanosecond => "ns",
        }
    }

    /// Return the number of nanoseconds in one unit.
    pub(super) const fn nanos(self) -> i128 {
        match self {
            Self::Day => 86_400_000_000_000,
            Self::Hour => 3_600_000_000_000,
            Self::Minute => 60_000_000_000,
            Self::Second => 1_000_000_000,
            Self::Millisecond => 1_000_000,
            Self::Microsecond => 1_000,
            Self::Nanosecond => 1,
        }
    }

    /// Return `true` when the primitive accepts this unit.
    fn accepted_by(self, primitive: Primitive) -> bool {
        match primitive {
            Primitive::Time => self <= Self::Millisecond,
            Primitive::S5Time => (Self::Hour..=Self::Millisecond).contains(&self),
            Primitive::Ltime => true,
            _ => false,
        }
    }
}

/// One part of a duration, such as `5ms`.
#[derive(Clone, Copy, Debug)]
struct Part {
    value: i128,
    unit: Unit,
}

/// Parse the body of a duration literal, the text after the `#`.
///
/// # Errors
///
/// Returns the [`IecLiteralReason`] of the first rule that the body breaks.
pub(super) fn parse(ty: TimeType, body: &str) -> Result<IecLiteral, IecLiteralReason> {
    let primitive = ty.primitive();
    let (negative, unsigned) = match body.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, body),
    };
    let parts = split_parts(unsigned)?;
    if negative && !matches!(primitive, Primitive::Time | Primitive::Ltime) {
        return Err(IecLiteralReason::Sign);
    }
    check_units(primitive, &parts)?;
    check_part_bounds(&parts)?;
    let total = sum_nanos(&parts)?;
    let base = base_nanos(primitive)?;
    let magnitude = total
        .checked_div(base)
        .ok_or(IecLiteralReason::OutOfRange)?;
    let count = if negative {
        magnitude
            .checked_neg()
            .ok_or(IecLiteralReason::OutOfRange)?
    } else {
        magnitude
    };
    if primitive == Primitive::S5Time {
        canonical::check_s5time_step(count)?;
    }
    Ok(IecLiteral { ty, count })
}

/// Return the units that the duration type accepts, from the largest to the smallest.
pub(super) fn units_of(primitive: Primitive) -> &'static [Unit] {
    match primitive {
        Primitive::S5Time => S5TIME_UNITS,
        Primitive::Ltime => LTIME_UNITS,
        _ => TIME_UNITS,
    }
}

/// Split the unsigned body into its parts. A body has at least one part.
fn split_parts(text: &str) -> Result<Vec<Part>, IecLiteralReason> {
    let mut parts = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let digit_count = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digit_count == 0 || digit_count > MAX_DIGITS {
            return Err(IecLiteralReason::Grammar);
        }
        let (digits, after_digits) = rest
            .split_at_checked(digit_count)
            .ok_or(IecLiteralReason::Grammar)?;
        let letter_count = after_digits
            .bytes()
            .position(|byte| byte.is_ascii_digit())
            .unwrap_or(after_digits.len());
        let (letters, remaining) = after_digits
            .split_at_checked(letter_count)
            .ok_or(IecLiteralReason::Grammar)?;
        let unit = Unit::parse(letters).ok_or(IecLiteralReason::Grammar)?;
        let value = digits
            .parse::<i128>()
            .map_err(|_| IecLiteralReason::Grammar)?;
        parts.push(Part { value, unit });
        rest = remaining;
    }
    if parts.is_empty() {
        return Err(IecLiteralReason::Grammar);
    }
    Ok(parts)
}

/// Check that every unit is accepted by the type, and that the units rise in grammar order.
fn check_units(primitive: Primitive, parts: &[Part]) -> Result<(), IecLiteralReason> {
    let mut previous: Option<Unit> = None;
    for part in parts {
        if !part.unit.accepted_by(primitive) {
            return Err(IecLiteralReason::Unit);
        }
        if previous.is_some_and(|before| part.unit <= before) {
            return Err(IecLiteralReason::Unit);
        }
        previous = Some(part.unit);
    }
    Ok(())
}

/// Check that each later part is less than one unit of the part before it.
fn check_part_bounds(parts: &[Part]) -> Result<(), IecLiteralReason> {
    for pair in parts.windows(2) {
        let [before, after] = pair else {
            return Err(IecLiteralReason::Part);
        };
        let span = after
            .value
            .checked_mul(after.unit.nanos())
            .ok_or(IecLiteralReason::OutOfRange)?;
        if span >= before.unit.nanos() {
            return Err(IecLiteralReason::Part);
        }
    }
    Ok(())
}

/// Sum the parts in nanoseconds.
fn sum_nanos(parts: &[Part]) -> Result<i128, IecLiteralReason> {
    parts.iter().try_fold(0_i128, |total, part| {
        let span = part
            .value
            .checked_mul(part.unit.nanos())
            .ok_or(IecLiteralReason::OutOfRange)?;
        total.checked_add(span).ok_or(IecLiteralReason::OutOfRange)
    })
}

/// Return the nanoseconds in one base unit of the type (A43).
pub(super) fn base_nanos(primitive: Primitive) -> Result<i128, IecLiteralReason> {
    match primitive {
        Primitive::Time | Primitive::S5Time => Ok(NANOS_PER_MILLISECOND),
        Primitive::Ltime => Ok(1),
        _ => Err(IecLiteralReason::Prefix),
    }
}

#[cfg(test)]
mod tests {
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
}
