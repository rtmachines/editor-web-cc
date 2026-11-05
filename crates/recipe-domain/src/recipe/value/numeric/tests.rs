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
