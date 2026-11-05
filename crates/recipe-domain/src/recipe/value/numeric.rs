//! Numeric ranges of the leaf types (`data-model.md` section 7, design D8).
//!
//! The ranges hold on every platform. `REAL` and `LREAL` use the range of `Decimal`,
//! which lies inside both IEEE 754 ranges (ADR-0028).

use rust_decimal::Decimal;

use crate::plant::{LeafClass, Primitive};

/// Return the low and high end of a numeric type, both inclusive.
///
/// The ends hold on every platform. `REAL` and `LREAL` return the range of `Decimal`.
/// The function returns `None` for every primitive that is not numeric.
#[must_use]
pub(crate) fn numeric_range(primitive: Primitive) -> Option<(Decimal, Decimal)> {
    let range = match primitive {
        Primitive::Sint => signed_range(i64::from(i8::MIN), i64::from(i8::MAX)),
        Primitive::Usint | Primitive::Byte => unsigned_range(u64::from(u8::MAX)),
        Primitive::Int => signed_range(i64::from(i16::MIN), i64::from(i16::MAX)),
        Primitive::Uint | Primitive::Word => unsigned_range(u64::from(u16::MAX)),
        Primitive::Dint => signed_range(i64::from(i32::MIN), i64::from(i32::MAX)),
        Primitive::Udint | Primitive::Dword => unsigned_range(u64::from(u32::MAX)),
        Primitive::Lint => signed_range(i64::MIN, i64::MAX),
        Primitive::Ulint | Primitive::Lword => unsigned_range(u64::MAX),
        Primitive::Real | Primitive::Lreal => (Decimal::MIN, Decimal::MAX),
        _ => return None,
    };
    Some(range)
}

/// Return `true` when a type holds only whole numbers.
///
/// The function returns `true` for the 12 integer types. It returns `false` for `REAL`,
/// `LREAL`, and every primitive that is not numeric.
#[must_use]
pub(crate) fn is_integer(primitive: Primitive) -> bool {
    primitive.leaf_class() == LeafClass::Numeric
        && !matches!(primitive, Primitive::Real | Primitive::Lreal)
}

/// Build the range of a signed type from its two ends.
fn signed_range(low: i64, high: i64) -> (Decimal, Decimal) {
    (Decimal::from(low), Decimal::from(high))
}

/// Build the range of an unsigned type. Its low end is zero.
fn unsigned_range(high: u64) -> (Decimal, Decimal) {
    (Decimal::ZERO, Decimal::from(high))
}

#[cfg(test)]
mod tests;
