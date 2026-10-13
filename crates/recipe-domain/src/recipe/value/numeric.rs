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
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::ToPrimitive;

    use super::{is_integer, numeric_range};
    use crate::plant::{LeafClass, Primitive};

    /// The 12 integer types with the low and high end of their IEC range.
    const INTEGER_TYPES: [(Primitive, &str, &str); 12] = [
        (Primitive::Sint, "-128", "127"),
        (Primitive::Usint, "0", "255"),
        (Primitive::Int, "-32768", "32767"),
        (Primitive::Uint, "0", "65535"),
        (Primitive::Dint, "-2147483648", "2147483647"),
        (Primitive::Udint, "0", "4294967295"),
        (
            Primitive::Lint,
            "-9223372036854775808",
            "9223372036854775807",
        ),
        (Primitive::Ulint, "0", "18446744073709551615"),
        (Primitive::Byte, "0", "255"),
        (Primitive::Word, "0", "65535"),
        (Primitive::Dword, "0", "4294967295"),
        (Primitive::Lword, "0", "18446744073709551615"),
    ];

    fn dec(text: &str) -> Decimal {
        text.parse().unwrap()
    }

    fn contains(range: (Decimal, Decimal), value: Decimal) -> bool {
        value >= range.0 && value <= range.1
    }

    #[test]
    fn integer_ranges_include_both_ends() {
        for (primitive, low, high) in INTEGER_TYPES {
            let range = numeric_range(primitive).unwrap();
            assert!(contains(range, dec(low)), "low end of {primitive:?}");
            assert!(contains(range, dec(high)), "high end of {primitive:?}");
        }
    }

    #[test]
    fn integer_ranges_exclude_one_step_outside_each_end() {
        for (primitive, low, high) in INTEGER_TYPES {
            let range = numeric_range(primitive).unwrap();
            let below = dec(low).checked_sub(Decimal::ONE).unwrap();
            let above = dec(high).checked_add(Decimal::ONE).unwrap();
            assert!(!contains(range, below), "below low end of {primitive:?}");
            assert!(!contains(range, above), "above high end of {primitive:?}");
        }
    }

    #[test]
    fn integer_ranges_are_exact() {
        for (primitive, low, high) in INTEGER_TYPES {
            assert_eq!(numeric_range(primitive), Some((dec(low), dec(high))));
        }
    }

    #[test]
    fn float_ranges_are_the_decimal_range() {
        assert_eq!(
            numeric_range(Primitive::Real),
            Some((Decimal::MIN, Decimal::MAX))
        );
        assert_eq!(
            numeric_range(Primitive::Lreal),
            Some((Decimal::MIN, Decimal::MAX))
        );
    }

    #[test]
    fn non_numeric_primitives_have_no_range() {
        let numeric: Vec<Primitive> = Primitive::ALL
            .into_iter()
            .filter(|primitive| primitive.leaf_class() == LeafClass::Numeric)
            .collect();
        assert_eq!(numeric.len(), 14);
        for primitive in Primitive::ALL {
            if primitive.leaf_class() != LeafClass::Numeric {
                assert_eq!(numeric_range(primitive), None, "{primitive:?}");
            }
        }
    }

    #[test]
    fn is_integer_is_true_for_the_twelve_integer_types() {
        for (primitive, _, _) in INTEGER_TYPES {
            assert!(is_integer(primitive), "{primitive:?}");
        }
    }

    #[test]
    fn is_integer_is_false_for_floats_and_non_numeric_types() {
        assert!(!is_integer(Primitive::Real));
        assert!(!is_integer(Primitive::Lreal));
        assert!(!is_integer(Primitive::Time));
    }

    #[test]
    fn decimal_extremes_fit_the_ieee_float_ranges() {
        for value in [Decimal::MAX, Decimal::MIN] {
            let as_f32 = value.to_f32().unwrap();
            let as_f64 = value.to_f64().unwrap();
            assert!(as_f32.is_finite());
            assert!(as_f64.is_finite());
            assert!(as_f32.abs() < f32::MAX);
            assert!(as_f64.abs() < f64::MAX);
        }
    }
}
