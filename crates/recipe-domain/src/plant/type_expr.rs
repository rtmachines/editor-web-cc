//! Text form of a type in a request (`domain-model.md` section 2.1).

use std::fmt;

use super::error::{DerivedTypeReason, PlantError, TypeLimit};
use super::lexical::{IntegerError, is_canonical_digits, parse_integer, split_at_most};
use super::limits::{MAX_DEPTH, MAX_DIMENSIONS};
use super::{ArrayDimension, ArrayDimensions, Primitive, StringKind, TypeName, TypeNameReason};

/// A type as written in a request, such as `ARRAY[1..8] OF REAL`.
///
/// The arrays are stored as a flat list, from the outermost to the innermost
/// `ARRAY[...] OF ` prefix. The base is the type that is not an array.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeExpr {
    arrays: Vec<ArrayDimensions>,
    base: BaseExpr,
}

/// The base of a type, which is not an array.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseExpr {
    /// A primitive type of `data-model.md` section 7, except the string types.
    Primitive(Primitive),
    /// A `STRING` or a `WSTRING`. The length is `None` when the text has none.
    String {
        /// The kind of the string.
        kind: StringKind,
        /// The number of characters, when the text gives one.
        length: Option<u16>,
    },
    /// A struct revision of the machine, such as `ST_Zone@3`.
    Udt {
        /// The name of the struct.
        name: TypeName,
        /// The revision of the struct.
        revision: u32,
    },
}

impl TypeExpr {
    /// Parse the text form of a type.
    ///
    /// # Errors
    ///
    /// Returns [`PlantError::DerivedTypeInvalid`] with the input text and a reason.
    /// Returns [`PlantError::TypeTooLarge`] when the array prefixes have more levels than
    /// `json.max_depth`, because such a value never fits the JSON depth limit.
    pub fn parse(text: &str) -> Result<Self, PlantError> {
        parse_type(text)
    }

    /// Return the array prefixes, from the outermost to the innermost.
    #[must_use]
    pub fn arrays(&self) -> &[ArrayDimensions] {
        &self.arrays
    }

    /// Return the base of the type, which is not an array.
    #[must_use]
    pub fn base(&self) -> &BaseExpr {
        &self.base
    }
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for dimensions in &self.arrays {
            write!(f, "ARRAY[{dimensions}] OF ")?;
        }
        write!(f, "{}", self.base)
    }
}

impl fmt::Display for BaseExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(primitive) => f.write_str(primitive.name()),
            Self::String { kind, length } => {
                let keyword = match kind {
                    StringKind::String => "STRING",
                    StringKind::Wstring => "WSTRING",
                };
                f.write_str(keyword)?;
                match length {
                    Some(length) => write!(f, "({length})"),
                    None => Ok(()),
                }
            }
            Self::Udt { name, revision } => write!(f, "{name}@{revision}"),
        }
    }
}

/// Largest revision of a struct: the INTEGER range (`2147483647`).
const MAX_REVISION: i64 = 2_147_483_647;

/// Build the error of a text that breaks a grammar rule.
fn invalid(text: &str, reason: DerivedTypeReason) -> PlantError {
    PlantError::DerivedTypeInvalid {
        type_expr: text.to_owned(),
        reason,
    }
}

/// Parse a whole type. The loop reads the `ARRAY[...] OF ` prefixes without recursion.
///
/// Each dimension adds one JSON level. The loop stops at once when the levels of the prefixes
/// pass `MAX_DEPTH`, so a long text costs a bounded amount of work.
fn parse_type(text: &str) -> Result<TypeExpr, PlantError> {
    let mut arrays = Vec::new();
    let mut levels = 0_usize;
    let mut rest = text;
    while let Some(after) = rest.strip_prefix("ARRAY[") {
        let (dimensions_text, tail) = after
            .split_once(']')
            .ok_or_else(|| invalid(text, DerivedTypeReason::Grammar))?;
        let tail = tail
            .strip_prefix(" OF ")
            .ok_or_else(|| invalid(text, DerivedTypeReason::Grammar))?;
        let dimensions =
            parse_dimensions(dimensions_text).map_err(|reason| invalid(text, reason))?;
        levels = levels
            .checked_add(dimensions.len())
            .filter(|sum| *sum <= depth_limit())
            .ok_or_else(|| PlantError::TypeTooLarge {
                type_name: text.to_owned(),
                limit: TypeLimit::Depth,
            })?;
        arrays.push(dimensions);
        rest = tail;
    }
    let base = parse_base(rest).map_err(|reason| invalid(text, reason))?;
    Ok(TypeExpr { arrays, base })
}

/// Return `MAX_DEPTH` as a `usize`. The value is 32, so the conversion never falls back.
fn depth_limit() -> usize {
    usize::try_from(MAX_DEPTH).unwrap_or(usize::MAX)
}

/// Parse the comma-separated dimensions of one `ARRAY[...]` prefix.
fn parse_dimensions(text: &str) -> Result<ArrayDimensions, DerivedTypeReason> {
    let parts = split_at_most(text, ',', MAX_DIMENSIONS).ok_or(DerivedTypeReason::Grammar)?;
    let dimensions = parts
        .into_iter()
        .map(parse_dimension)
        .collect::<Result<Vec<ArrayDimension>, _>>()?;
    ArrayDimensions::parse(dimensions).map_err(|_| DerivedTypeReason::Grammar)
}

/// Parse one dimension such as `1..8`.
fn parse_dimension(text: &str) -> Result<ArrayDimension, DerivedTypeReason> {
    let (lower, upper) = text.split_once("..").ok_or(DerivedTypeReason::Grammar)?;
    let lower = parse_bound(lower)?;
    let upper = parse_bound(upper)?;
    ArrayDimension::parse(lower, upper).map_err(|_| DerivedTypeReason::OutOfRange)
}

/// Parse a bound `-?[0-9]{1,10}`. The value must fit the INTEGER range.
fn parse_bound(text: &str) -> Result<i32, DerivedTypeReason> {
    parse_integer(text).map_err(|error| match error {
        IntegerError::Grammar => DerivedTypeReason::Grammar,
        IntegerError::OutOfRange => DerivedTypeReason::OutOfRange,
    })
}

/// Parse the base of a type, which is not an array.
fn parse_base(text: &str) -> Result<BaseExpr, DerivedTypeReason> {
    if let Some((name, revision)) = text.split_once('@') {
        return parse_udt(name, revision);
    }
    if let Some((kind, tail)) = split_string_keyword(text) {
        return parse_string(kind, tail);
    }
    Primitive::parse(text)
        .map(BaseExpr::Primitive)
        .map_err(|_| DerivedTypeReason::Grammar)
}

/// Split `STRING` or `WSTRING` from the text that follows it.
fn split_string_keyword(text: &str) -> Option<(StringKind, &str)> {
    if let Some(tail) = text.strip_prefix("WSTRING") {
        return Some((StringKind::Wstring, tail));
    }
    text.strip_prefix("STRING")
        .map(|tail| (StringKind::String, tail))
}

/// Parse the optional `(length)` of a string type.
fn parse_string(kind: StringKind, tail: &str) -> Result<BaseExpr, DerivedTypeReason> {
    if tail.is_empty() {
        return Ok(BaseExpr::String { kind, length: None });
    }
    let inner = tail
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or(DerivedTypeReason::Grammar)?;
    if !is_canonical_digits(inner, 5) {
        return Err(DerivedTypeReason::Grammar);
    }
    let value: u32 = inner.parse().map_err(|_| DerivedTypeReason::Grammar)?;
    let length = u16::try_from(value).map_err(|_| DerivedTypeReason::OutOfRange)?;
    Ok(BaseExpr::String {
        kind,
        length: Some(length),
    })
}

/// Parse a struct reference `name@revision`.
fn parse_udt(name: &str, revision: &str) -> Result<BaseExpr, DerivedTypeReason> {
    let name = TypeName::parse(name).map_err(|reason| match reason {
        TypeNameReason::Format => DerivedTypeReason::Grammar,
        TypeNameReason::Reserved => DerivedTypeReason::UnknownStruct,
    })?;
    if !is_canonical_digits(revision, 10) {
        return Err(DerivedTypeReason::Grammar);
    }
    let value: i64 = revision.parse().map_err(|_| DerivedTypeReason::Grammar)?;
    if value > MAX_REVISION {
        return Err(DerivedTypeReason::UnknownStruct);
    }
    let revision = u32::try_from(value).map_err(|_| DerivedTypeReason::UnknownStruct)?;
    Ok(BaseExpr::Udt { name, revision })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn reason_of(text: &str) -> Option<DerivedTypeReason> {
        match TypeExpr::parse(text) {
            Err(PlantError::DerivedTypeInvalid { type_expr, reason }) => {
                assert_eq!(type_expr, text);
                Some(reason)
            }
            _ => None,
        }
    }

    fn assert_round_trip(text: &str) {
        let parsed = TypeExpr::parse(text);
        assert!(parsed.is_ok(), "{text} must parse");
        assert_eq!(parsed.ok().map(|t| t.to_string()), Some(text.to_owned()));
    }

    #[test]
    fn type_expr_examples_round_trip() {
        for text in [
            "INT",
            "STRING",
            "STRING(20)",
            "WSTRING(80)",
            "ST_Zone@3",
            "ARRAY[1..8] OF REAL",
            "ARRAY[0..1,1..2] OF ST_Zone@3",
            "ARRAY[1..4] OF STRING(16)",
            "ARRAY[1..2] OF ARRAY[1..3] OF INT",
        ] {
            assert_round_trip(text);
        }
    }

    #[test]
    fn type_expr_nested_array_has_two_prefixes() {
        let parsed = TypeExpr::parse("ARRAY[1..2] OF ARRAY[1..3] OF INT");
        assert_eq!(parsed.ok().map(|t| t.arrays().len()), Some(2));
    }

    #[test]
    fn type_expr_base_of_string_without_length_is_none() {
        let parsed = TypeExpr::parse("STRING");
        assert_eq!(
            parsed.ok().map(|t| t.base().clone()),
            Some(BaseExpr::String {
                kind: StringKind::String,
                length: None,
            })
        );
    }

    #[test]
    fn type_expr_udt_without_at_is_grammar_error() {
        assert_eq!(reason_of("ST_Zone"), Some(DerivedTypeReason::Grammar));
    }

    #[test]
    fn type_expr_missing_of_is_grammar_error() {
        assert_eq!(
            reason_of("ARRAY[1..8] INT"),
            Some(DerivedTypeReason::Grammar)
        );
    }

    #[test]
    fn type_expr_four_dimensions_is_grammar_error() {
        assert_eq!(
            reason_of("ARRAY[1..2,1..2,1..2,1..2] OF INT"),
            Some(DerivedTypeReason::Grammar)
        );
    }

    #[test]
    fn type_expr_reversed_bounds_is_out_of_range() {
        assert_eq!(
            reason_of("ARRAY[5..1] OF INT"),
            Some(DerivedTypeReason::OutOfRange)
        );
    }

    #[test]
    fn type_expr_span_of_65536_is_out_of_range() {
        assert_eq!(
            reason_of("ARRAY[0..65536] OF INT"),
            Some(DerivedTypeReason::OutOfRange)
        );
    }

    #[test]
    fn type_expr_string_length_above_u16_is_out_of_range() {
        assert_eq!(
            reason_of("STRING(99999)"),
            Some(DerivedTypeReason::OutOfRange)
        );
    }

    #[test]
    fn type_expr_revision_above_integer_range_is_unknown_struct() {
        assert_eq!(
            reason_of("ST_Zone@4294967296"),
            Some(DerivedTypeReason::UnknownStruct)
        );
        assert_eq!(
            reason_of("ST_Zone@2147483648"),
            Some(DerivedTypeReason::UnknownStruct)
        );
    }

    #[test]
    fn type_expr_revision_at_integer_max_parses() {
        assert_round_trip("ST_Zone@2147483647");
    }

    #[test]
    fn type_expr_lower_bound_above_integer_max_is_out_of_range() {
        assert_eq!(
            reason_of("ARRAY[2147483648..1] OF INT"),
            Some(DerivedTypeReason::OutOfRange)
        );
    }

    #[test]
    fn type_expr_lower_bound_below_integer_min_is_out_of_range() {
        assert_eq!(
            reason_of("ARRAY[-2147483649..1] OF INT"),
            Some(DerivedTypeReason::OutOfRange)
        );
    }

    #[test]
    fn type_expr_bound_with_eleven_digits_is_grammar_error() {
        assert_eq!(
            reason_of("ARRAY[12345678901..1] OF INT"),
            Some(DerivedTypeReason::Grammar)
        );
    }

    #[test]
    fn type_expr_malformed_texts_are_grammar_errors() {
        for text in [
            "STRING(0)",
            "STRING(020)",
            "STRING()",
            "ST_Zone@0",
            "ST_Zone@01",
            "ST_Zone",
            "1ST@1",
            "int",
            "DT",
            "array[1..2] OF INT",
            "ARRAY[1..2]OF INT",
            "ARRAY [1..2] OF INT",
            "ARRAY[1..2] OF  INT",
            "ARRAY[] OF INT",
            "ARRAY[1..2,] OF INT",
            "ARRAY[1..2] OF ",
            "",
        ] {
            assert_eq!(reason_of(text), Some(DerivedTypeReason::Grammar), "{text}");
        }
    }

    #[test]
    fn type_expr_primitive_name_with_at_is_unknown_struct() {
        assert_eq!(reason_of("INT@1"), Some(DerivedTypeReason::UnknownStruct));
    }

    #[test]
    fn type_expr_reversed_bounds_with_leading_zeros_is_out_of_range() {
        assert_eq!(
            reason_of("ARRAY[007..-0] OF INT"),
            Some(DerivedTypeReason::OutOfRange)
        );
    }

    #[test]
    fn type_expr_negative_zero_bound_displays_canonical_number() {
        let parsed = TypeExpr::parse("ARRAY[-0..007] OF INT");
        assert_eq!(
            parsed.ok().map(|t| t.to_string()),
            Some("ARRAY[0..7] OF INT".to_owned())
        );
    }

    fn depth_error(text: &str) -> bool {
        matches!(
            TypeExpr::parse(text),
            Err(PlantError::TypeTooLarge {
                limit: TypeLimit::Depth,
                ..
            })
        )
    }

    #[test]
    fn type_expr_32_one_dimensional_prefixes_parse() {
        let text = format!("{}INT", "ARRAY[1..1] OF ".repeat(32));
        let parsed = TypeExpr::parse(&text);
        assert_eq!(parsed.as_ref().ok().map(|t| t.arrays().len()), Some(32));
        assert_eq!(parsed.ok().map(|t| t.to_string()), Some(text));
    }

    #[test]
    fn type_expr_33_one_dimensional_prefixes_are_too_deep() {
        assert!(depth_error(&format!("{}INT", "ARRAY[1..1] OF ".repeat(33))));
    }

    #[test]
    fn type_expr_ten_thousand_prefixes_are_too_deep_without_recursion() {
        assert!(depth_error(&format!(
            "{}INT",
            "ARRAY[1..1] OF ".repeat(10_000)
        )));
    }

    #[test]
    fn type_expr_eleven_three_dimensional_prefixes_are_too_deep() {
        let prefix = "ARRAY[1..1,1..1,1..1] OF ";
        assert!(!depth_error(&format!("{}INT", prefix.repeat(10))));
        assert!(depth_error(&format!("{}INT", prefix.repeat(11))));
    }

    #[test]
    fn type_expr_too_deep_error_carries_the_input_text() {
        let text = format!("{}INT", "ARRAY[1..1] OF ".repeat(33));
        assert!(matches!(
            TypeExpr::parse(&text),
            Err(PlantError::TypeTooLarge { type_name, .. }) if type_name == text
        ));
    }

    #[test]
    fn type_expr_hundred_thousand_commas_are_a_grammar_error() {
        let text = format!("ARRAY[{}] OF INT", ",".repeat(100_000));
        assert_eq!(reason_of(&text), Some(DerivedTypeReason::Grammar));
    }

    fn dimension() -> impl Strategy<Value = String> {
        (-100_i32..=100, -100_i32..=100).prop_map(|(a, b)| {
            let (lower, upper) = (a.min(b), a.max(b));
            format!("{lower}..{upper}")
        })
    }

    fn array_prefix() -> impl Strategy<Value = String> {
        prop::collection::vec(dimension(), 1..=3)
            .prop_map(|dims| format!("ARRAY[{}] OF ", dims.join(",")))
    }

    fn base() -> impl Strategy<Value = String> {
        let primitives: Vec<&'static str> = Primitive::ALL
            .into_iter()
            .filter(|p| !matches!(p, Primitive::String | Primitive::Wstring))
            .map(Primitive::name)
            .collect();
        let primitive = prop::sample::select(primitives).prop_map(str::to_owned);
        let string = (
            prop_oneof![Just("STRING"), Just("WSTRING")],
            prop::option::of(1_u16..=16_382),
        )
            .prop_map(|(keyword, length)| match length {
                Some(length) => format!("{keyword}({length})"),
                None => keyword.to_owned(),
            });
        let udt = ("[A-Za-z_][A-Za-z0-9_]{0,10}", 1_u32..=2_147_483_647)
            .prop_filter("name is not a primitive", |(name, _)| {
                Primitive::parse(name).is_err()
            })
            .prop_map(|(name, revision)| format!("{name}@{revision}"));
        prop_oneof![primitive, string, udt]
    }

    proptest! {
        #[test]
        fn type_expr_generated_text_round_trips(
            prefixes in prop::collection::vec(array_prefix(), 0..=4),
            base_text in base(),
        ) {
            let text = format!("{}{base_text}", prefixes.concat());
            match TypeExpr::parse(&text) {
                Ok(parsed) => prop_assert_eq!(parsed.to_string(), text),
                Err(error) => prop_assert!(false, "{text} failed: {error}"),
            }
        }
    }
}
