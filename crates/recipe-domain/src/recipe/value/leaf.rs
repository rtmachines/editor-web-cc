//! Conversion of one scalar leaf to its canonical stored form (design D8).

use serde_json::{Number, Value};

use super::decimal::{MAX_SIGNIFICANT_DIGITS, canonical, parse_decimal};
use super::numeric::{is_integer, numeric_range};
use crate::plant::{IecLiteral, LeafClass, PlcPlatform, Primitive, TimeType};

/// Highest scalar of a `CHAR` and of a `STRING` character (Latin-1, ADR-0028).
const LATIN_1_MAX: u32 = 0xFF;

/// Highest scalar of a `WCHAR` (`domain-model.md` section 2.1).
const WCHAR_MAX: u32 = 0xFFFF;

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
}

/// Convert a leaf to its canonical stored form.
///
/// The function returns the short text of the expected form when the value does not match.
pub(super) fn convert(value: &Value, leaf: LeafType) -> Result<Value, String> {
    match leaf.primitive.leaf_class() {
        LeafClass::Numeric => numeric(value, leaf.primitive),
        LeafClass::Bool => match value {
            Value::Bool(_) => Ok(value.clone()),
            _ => Err(String::from("true or false")),
        },
        LeafClass::String => string(value, leaf),
        LeafClass::Char => character(value, leaf.primitive),
        LeafClass::Duration | LeafClass::Instant => time(value, leaf),
    }
}

/// Convert a `STRING` or `WSTRING` leaf. The text must fit the length of the type.
fn string(value: &Value, leaf: LeafType) -> Result<Value, String> {
    let is_wide = leaf.primitive == Primitive::Wstring;
    let length = leaf.string_length.unwrap_or_default();
    let expected = if is_wide {
        format!("a string of at most {length} UTF-16 units")
    } else {
        format!("a string of at most {length} characters from Latin-1")
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
fn character(value: &Value, primitive: Primitive) -> Result<Value, String> {
    let (highest, label) = if primitive == Primitive::Wchar {
        (WCHAR_MAX, "U+FFFF")
    } else {
        (LATIN_1_MAX, "U+00FF")
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
        Err(format!("one character from U+0000 to {label}"))
    }
}

/// Convert a time leaf. The result is the count.
///
/// A request carries an IEC literal. Storage carries the count as a JSON number.
fn time(value: &Value, leaf: LeafType) -> Result<Value, String> {
    let name = leaf.primitive.name();
    let expected = || match leaf.mode {
        Mode::Request => format!("an IEC literal of type {name}"),
        Mode::Stored => format!("a count of type {name}"),
    };
    let time_type = TimeType::of(leaf.primitive, leaf.platform).ok_or_else(expected)?;
    let literal = match (leaf.mode, value) {
        (Mode::Request, Value::String(text)) => IecLiteral::parse(time_type, text).ok(),
        (Mode::Stored, Value::Number(number)) => number
            .to_string()
            .parse::<i128>()
            .ok()
            .and_then(|count| IecLiteral::try_from((time_type, count)).ok()),
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
fn numeric(value: &Value, primitive: Primitive) -> Result<Value, String> {
    let Value::Number(number) = value else {
        return Err(numeric_expected(primitive));
    };
    let decimal = parse_decimal(&number.to_string())
        .and_then(canonical)
        .ok_or_else(|| numeric_expected(primitive))?;
    if is_integer(primitive) && decimal.scale() != 0 {
        return Err(format!("an integer of type {}", primitive.name()));
    }
    let (low, high) = numeric_range(primitive).ok_or_else(|| numeric_expected(primitive))?;
    if decimal < low || decimal > high {
        return Err(numeric_expected(primitive));
    }
    decimal
        .to_string()
        .parse::<Number>()
        .map(Value::Number)
        .map_err(|_| numeric_expected(primitive))
}

/// Write the expected form of a numeric leaf.
fn numeric_expected(primitive: Primitive) -> String {
    match numeric_range(primitive) {
        Some((low, high)) if is_integer(primitive) => {
            format!("a number of type {} from {low} to {high}", primitive.name())
        }
        _ => format!(
            "a number of type {} with at most {MAX_SIGNIFICANT_DIGITS} significant digits",
            primitive.name()
        ),
    }
}

#[cfg(test)]
mod tests;
