//! Text form of a type in a request (`domain-model.md` section 2.1).

use std::fmt;

use super::error::{DerivedTypeReason, PlantError, TypeLimit};
use super::lexical::{IntegerError, is_canonical_digits, parse_integer, split_at_most};
use super::limits::{MAX_DEPTH, MAX_DIMENSIONS};
use super::{
    ArrayDimension, ArrayDimensions, PlcPlatform, Primitive, RuleSet, StringKind, TypeName,
    TypeNameReason,
};

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
        Self::from_stored(text, RuleSet::Current)
    }

    /// Parse the text form of a type under a rule set (ADR-0120, ADR-0143 (i1), (i2)).
    ///
    /// [`RuleSet::Current`] applies the rule of [`TypeExpr::parse`]. A struct name that is
    /// reserved in any case gives [`DerivedTypeReason::ReservedName`]. [`RuleSet::Earlier`]
    /// refuses only a primitive name in exact case. Thus the text `Int@1` names the stored
    /// struct `Int@1`. A caller that reads a type text that it does not write passes
    /// [`RuleSet::Earlier`].
    ///
    /// # Errors
    ///
    /// Returns the errors of [`TypeExpr::parse`].
    pub fn from_stored(text: &str, rules: RuleSet) -> Result<Self, PlantError> {
        parse_type(text, rules)
    }

    /// Return the array prefixes, from the outermost to the innermost.
    #[must_use]
    pub fn arrays(&self) -> &[ArrayDimensions] {
        &self.arrays
    }

    /// Refuse an array of an array on `S7_1200` and `S7_1500` (ADR-0148 (a), (b), (d), (i)).
    ///
    /// Solves this problem: TIA Portal does not allow an array element that is an array on
    /// these two platforms. A multi-dimension prefix such as `ARRAY[1..2,1..3] OF BYTE` is
    /// one prefix and passes. The rule reads only the written text. A struct reference may
    /// name a stored struct with such an array, and this method does not open it
    /// (ADR-0148 (c)). Every other platform accepts the expression.
    ///
    /// # Errors
    ///
    /// Returns [`PlantError::DerivedTypeInvalid`] with the reason
    /// [`DerivedTypeReason::NotOnPlatform`] and the text of the expression (`E-PLT-013`) when
    /// the platform is `S7_1200` or `S7_1500` and the expression has more than one array prefix.
    pub fn require_no_nested_array(&self, platform: PlcPlatform) -> Result<(), PlantError> {
        let refuses_nesting = matches!(platform, PlcPlatform::S7_1200 | PlcPlatform::S7_1500);
        if refuses_nesting && self.arrays.len() > 1 {
            return Err(invalid(&self.to_string(), DerivedTypeReason::NotOnPlatform));
        }
        Ok(())
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
fn parse_type(text: &str, rules: RuleSet) -> Result<TypeExpr, PlantError> {
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
    let base = parse_base(rest, rules).map_err(|reason| invalid(text, reason))?;
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
fn parse_base(text: &str, rules: RuleSet) -> Result<BaseExpr, DerivedTypeReason> {
    if let Some((name, revision)) = text.split_once('@') {
        return parse_udt(name, revision, rules);
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

/// Parse a struct reference `name@revision` under a rule set (ADR-0143 (i2)).
///
/// A primitive name in exact case is `UnknownStruct` under both rule sets. Under
/// [`RuleSet::Current`], any other reserved name is `ReservedName`.
fn parse_udt(name: &str, revision: &str, rules: RuleSet) -> Result<BaseExpr, DerivedTypeReason> {
    let name = TypeName::from_stored(name, rules).map_err(|reason| match reason {
        TypeNameReason::Format => DerivedTypeReason::Grammar,
        TypeNameReason::Reserved if Primitive::parse(name).is_ok() => {
            DerivedTypeReason::UnknownStruct
        }
        TypeNameReason::Reserved => DerivedTypeReason::ReservedName,
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
mod tests;
