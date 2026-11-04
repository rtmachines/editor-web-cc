//! Conversion of one scalar leaf to its canonical stored form (design D8).

use serde_json::{Number, Value};

use super::decimal::{MAX_SIGNIFICANT_DIGITS, canonical, parse_decimal, significant_digits};
use super::expected::Expected;
use super::numeric::{is_integer, numeric_range};
use crate::plant::{IecLiteral, LeafClass, PlcPlatform, Primitive, RuleSet, TimeType};

/// Highest scalar of a `CHAR` and of a `STRING` character (Latin-1, ADR-0028).
const LATIN_1_MAX: u32 = 0xFF;

/// Highest scalar of a `WCHAR` (`domain-model.md` section 2.1).
const WCHAR_MAX: u32 = 0xFFFF;

/// Largest count of significant digits of a `REAL` number or bound (ADR-0117 (a)).
const REAL_MAX_DIGITS: u8 = 7;

/// Largest count of significant digits of an `LREAL` number or bound (ADR-0117 (a)).
const LREAL_MAX_DIGITS: u8 = 15;

/// Why one leaf does not convert.
///
/// The type solves one problem: the conversion of a leaf reports a wrong form and a number with
/// too many digits apart. The second one gives `E-RCP-055` and not `E-RCP-005` (ADR-0117 (d)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::recipe) enum LeafFault {
    /// The value has not the form that the leaf needs.
    Expected(Expected),
    /// A `REAL` or `LREAL` number has more significant digits than its type holds.
    Precision {
        /// The type of the leaf.
        type_name: Primitive,
        /// The largest permitted count of significant digits.
        max_digits: u8,
    },
}

impl From<Expected> for LeafFault {
    fn from(expected: Expected) -> Self {
        Self::Expected(expected)
    }
}

/// Return the largest count of significant digits of a type, or `None` when the type has no
/// such rule.
///
/// The function is the one source of the digit limits of ADR-0117 (a). The value check and the
/// bound check of a limit both call it, so the two checks cannot differ.
/// `domain-model.md` section 2.1 states the rule.
pub(in crate::recipe) const fn max_significant_digits(primitive: Primitive) -> Option<u8> {
    match primitive {
        Primitive::Real => Some(REAL_MAX_DIGITS),
        Primitive::Lreal => Some(LREAL_MAX_DIGITS),
        _ => None,
    }
}

/// The form in which a leaf arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// A request carries a time leaf as an IEC literal.
    Request,
    /// Storage carries a time leaf as a count.
    Stored,
}

/// The type of one leaf and the form in which the leaf arrives.
#[derive(Clone, Copy, Debug)]
pub(super) struct LeafType {
    pub(super) primitive: Primitive,
    pub(super) string_length: Option<u16>,
    pub(super) platform: PlcPlatform,
    pub(super) mode: Mode,
    /// The rule set of the check. A request uses [`RuleSet::Current`] (ADR-0120).
    pub(super) rules: RuleSet,
}

/// Convert a leaf to its canonical stored form.
///
/// The function returns the fault of the leaf when the value does not match.
pub(super) fn convert(value: &Value, leaf: LeafType) -> Result<Value, LeafFault> {
    match leaf.primitive.leaf_class() {
        LeafClass::Numeric => numeric(value, leaf.primitive, leaf.rules),
        LeafClass::Bool => match value {
            Value::Bool(_) => Ok(value.clone()),
            _ => Err(Expected::Bool.into()),
        },
        LeafClass::String => string(value, leaf).map_err(LeafFault::from),
        LeafClass::Char => character(value, leaf.primitive).map_err(LeafFault::from),
        LeafClass::Duration | LeafClass::Instant => time(value, leaf).map_err(LeafFault::from),
    }
}

/// Convert a `STRING` or `WSTRING` leaf. The text must fit the length of the type.
fn string(value: &Value, leaf: LeafType) -> Result<Value, Expected> {
    let is_wide = leaf.primitive == Primitive::Wstring;
    let length = leaf.string_length.unwrap_or_default();
    let expected = if is_wide {
        Expected::Wstring { length }
    } else {
        Expected::String { length }
    };
    let Value::String(text) = value else {
        return Err(expected);
    };
    let is_narrow = text.chars().all(|c| u32::from(c) <= LATIN_1_MAX);
    let units = if is_wide {
        text.encode_utf16().count()
    } else {
        text.chars().count()
    };
    if (is_wide || is_narrow) && units <= usize::from(length) {
        Ok(value.clone())
    } else {
        Err(expected)
    }
}

/// Convert a `CHAR` or `WCHAR` leaf. The text must hold one scalar inside the range of the type.
fn character(value: &Value, primitive: Primitive) -> Result<Value, Expected> {
    let highest = if primitive == Primitive::Wchar {
        WCHAR_MAX
    } else {
        LATIN_1_MAX
    };
    let is_valid = match value {
        Value::String(text) => {
            let mut scalars = text.chars();
            matches!(
                (scalars.next(), scalars.next()),
                (Some(c), None) if u32::from(c) <= highest
            )
        }
        _ => false,
    };
    if is_valid {
        Ok(value.clone())
    } else {
        Err(Expected::Char { max: highest })
    }
}

/// Convert a time leaf. The result is the count.
///
/// A request carries an IEC literal. Storage carries the count as a JSON number.
fn time(value: &Value, leaf: LeafType) -> Result<Value, Expected> {
    let type_name = leaf.primitive;
    let expected = || match leaf.mode {
        Mode::Request => Expected::Literal { type_name },
        Mode::Stored => Expected::Count { type_name },
    };
    let time_type = TimeType::of(leaf.primitive, leaf.platform).ok_or_else(expected)?;
    let literal = match (leaf.mode, value) {
        (Mode::Request, Value::String(text)) => IecLiteral::parse(time_type, text).ok(),
        (Mode::Stored, Value::Number(number)) => number
            .to_string()
            .parse::<i128>()
            .ok()
            .and_then(|count| IecLiteral::from_count(time_type, count, leaf.rules).ok()),
        _ => None,
    };
    literal
        .and_then(|checked| count_value(checked.count()))
        .ok_or_else(expected)
}

/// Write a count as a JSON number.
fn count_value(count: i128) -> Option<Value> {
    count.to_string().parse::<Number>().ok().map(Value::Number)
}

/// Convert a numeric leaf. The result is a JSON number with canonical decimal text.
///
/// The check of the significant digits of a `REAL` or `LREAL` number runs after the range check
/// and only in the current rule set (ADR-0117, ADR-0120 (a)).
fn numeric(value: &Value, primitive: Primitive, rules: RuleSet) -> Result<Value, LeafFault> {
    let expected = || LeafFault::from(numeric_expected(primitive));
    let Value::Number(number) = value else {
        return Err(expected());
    };
    let decimal = parse_decimal(&number.to_string())
        .and_then(canonical)
        .ok_or_else(expected)?;
    if is_integer(primitive) && decimal.scale() != 0 {
        return Err(Expected::Integer {
            type_name: primitive,
        }
        .into());
    }
    let (low, high) = numeric_range(primitive).ok_or_else(expected)?;
    if decimal < low || decimal > high {
        return Err(expected());
    }
    if rules == RuleSet::Current
        && let Some(max) = max_significant_digits(primitive)
        && significant_digits(decimal) > u32::from(max)
    {
        return Err(LeafFault::Precision {
            type_name: primitive,
            max_digits: max,
        });
    }
    decimal
        .to_string()
        .parse::<Number>()
        .map(Value::Number)
        .map_err(|_| expected())
}

/// Return the expected form of a numeric leaf.
fn numeric_expected(primitive: Primitive) -> Expected {
    match numeric_range(primitive) {
        Some((low, high)) if is_integer(primitive) => Expected::NumberRange {
            type_name: primitive,
            low,
            high,
        },
        _ => Expected::NumberDigits {
            type_name: primitive,
            digits: MAX_SIGNIFICANT_DIGITS,
        },
    }
}

#[cfg(test)]
mod tests;
