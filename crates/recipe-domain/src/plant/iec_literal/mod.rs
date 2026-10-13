//! IEC literals of time values (`domain-model.md` section 2.1, design D9).
//!
//! The parser uses no external crate. It uses checked integer arithmetic.

use std::fmt;

use super::{LeafClass, PlcPlatform, Primitive};

mod calendar;
mod canonical;
mod duration;
mod instant;
mod range;

#[cfg(test)]
mod prop;

/// A `Duration` or `Instant` type on one PLC platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimeType {
    primitive: Primitive,
    platform: PlcPlatform,
}

impl TimeType {
    /// Return the time type of a primitive on a platform.
    ///
    /// Returns `None` when the primitive is neither a `Duration` nor an `Instant`.
    #[must_use]
    pub fn of(primitive: Primitive, platform: PlcPlatform) -> Option<TimeType> {
        match primitive.leaf_class() {
            LeafClass::Duration | LeafClass::Instant => Some(TimeType {
                primitive,
                platform,
            }),
            _ => None,
        }
    }

    /// Return the primitive of this type.
    #[must_use]
    pub fn primitive(self) -> Primitive {
        self.primitive
    }

    /// Return the PLC platform of this type.
    #[must_use]
    pub fn platform(self) -> PlcPlatform {
        self.platform
    }

    /// Return the lowest and the highest count of this type on its platform (A43, ADR-0026).
    ///
    /// Returns `None` when the type does not exist on the platform.
    #[must_use]
    pub fn range(self) -> Option<(i128, i128)> {
        range::range(self).map(|allowed| (*allowed.start(), *allowed.end()))
    }
}

/// An IEC literal of a time type. The count is in the base unit of the type (A43).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IecLiteral {
    ty: TimeType,
    count: i128,
}

/// Reason why a text is not a valid IEC literal of a time type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IecLiteralReason {
    /// The text does not match the literal grammar.
    Grammar,
    /// The prefix names another type.
    Prefix,
    /// A unit is not accepted by the type, repeated, or out of order.
    Unit,
    /// A sign is written for a type that has no sign.
    Sign,
    /// A later part is not less than one unit of the part before it.
    Part,
    /// The fraction has more digits than the type accepts.
    Fraction,
    /// The date does not exist, or the time of day is out of range.
    Calendar,
    /// The S5TIME count is between two steps.
    Step,
    /// The count is outside the range of the type.
    OutOfRange,
    /// The type does not exist on the platform.
    NotOnPlatform,
}

impl IecLiteral {
    /// Parse the text of an IEC literal of the given time type.
    ///
    /// # Errors
    ///
    /// Returns the [`IecLiteralReason`] of the first rule that the text breaks.
    pub fn parse(ty: TimeType, text: &str) -> Result<IecLiteral, IecLiteralReason> {
        let Some((prefix, body)) = text.split_once('#') else {
            return Err(IecLiteralReason::Grammar);
        };
        match literal_prefix(prefix) {
            Some(named) if named == ty.primitive() => {
                let literal = match ty.primitive().leaf_class() {
                    LeafClass::Duration => duration::parse(ty, body),
                    _ => instant::parse(ty, body),
                }?;
                range::check(ty, literal.count)?;
                Ok(literal)
            }
            Some(_) => Err(IecLiteralReason::Prefix),
            None => Err(IecLiteralReason::Grammar),
        }
    }

    /// Return the count of the base unit of the type (A43).
    #[must_use]
    pub fn count(&self) -> i128 {
        self.count
    }

    /// Return the time type of this literal.
    #[must_use]
    pub fn time_type(&self) -> TimeType {
        self.ty
    }
}

/// Return the type that a literal prefix names, such as `LT` for `LTIME`.
fn literal_prefix(prefix: &str) -> Option<Primitive> {
    match prefix {
        "T" => Some(Primitive::Time),
        "LT" => Some(Primitive::Ltime),
        "S5T" => Some(Primitive::S5Time),
        "D" => Some(Primitive::Date),
        "LD" => Some(Primitive::Ldate),
        "TOD" => Some(Primitive::TimeOfDay),
        "LTOD" => Some(Primitive::Ltod),
        "DT" => Some(Primitive::DateAndTime),
        "LDT" => Some(Primitive::Ldt),
        "DTL" => Some(Primitive::Dtl),
        _ => None,
    }
}

impl fmt::Display for IecLiteralReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Grammar => "the text does not match the literal grammar",
            Self::Prefix => "the prefix names another type",
            Self::Unit => "a unit is not accepted, repeated, or out of order",
            Self::Sign => "the type takes no sign",
            Self::Part => "a later part is not less than the part before it",
            Self::Fraction => "the fraction has too many digits",
            Self::Calendar => "the date or the time of day does not exist",
            Self::Step => "the value is between two steps",
            Self::OutOfRange => "the value is outside the range of the type",
            Self::NotOnPlatform => "the type does not exist on the platform",
        };
        f.write_str(text)
    }
}
