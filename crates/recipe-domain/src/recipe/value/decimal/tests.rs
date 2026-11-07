use proptest::prelude::*;
use rust_decimal::Decimal;

use super::{canonical, is_json_number, parse_decimal, significant_digits};
use crate::recipe::value::test_support::{REAL, primitive_tree};
use crate::recipe::value::{Expected, ParameterValue, ValueFault};

fn digits(text: &str) -> u32 {
    significant_digits(parse_decimal(text).unwrap())
}

#[test]
fn significant_digits_ignores_trailing_zeros_of_a_whole_number() {
    assert_eq!(digits("1000000000"), 1);
}

#[test]
fn significant_digits_ignores_leading_zeros_of_a_fraction() {
    assert_eq!(digits("0.00012345"), 5);
}

#[test]
fn significant_digits_counts_all_digits_of_an_integer() {
    assert_eq!(digits("16777217"), 8);
}

#[test]
fn significant_digits_of_zero_is_zero() {
    assert_eq!(significant_digits(Decimal::ZERO), 0);
    assert_eq!(digits("0.000"), 0);
}

#[test]
fn significant_digits_ignores_the_sign() {
    assert_eq!(digits("-16777217"), 8);
    assert_eq!(digits("-0.00012345"), 5);
}

#[test]
fn significant_digits_ignores_trailing_zeros_of_a_fraction() {
    assert_eq!(digits("1.500"), 2);
    assert_eq!(digits("0.1200"), 2);
    assert_eq!(digits("100.00"), 1);
}

#[test]
fn significant_digits_counts_zeros_between_non_zero_digits() {
    assert_eq!(digits("1.0005"), 5);
}

#[test]
fn significant_digits_of_the_largest_decimal_is_29_and_not_canonical() {
    assert_eq!(significant_digits(Decimal::MAX), 29);
    assert_eq!(canonical(Decimal::MAX), None);
}

#[test]
fn significant_digits_of_the_largest_mantissa_is_29() {
    assert_eq!(significant_digits(Decimal::MAX), 29);
}

#[test]
fn significant_digits_of_one_is_1() {
    assert_eq!(significant_digits(Decimal::ONE), 1);
}

#[test]
fn canonical_uses_the_same_count_as_significant_digits() {
    let with_28 = parse_decimal("1234567890123456789012345678").unwrap();
    assert_eq!(significant_digits(with_28), 28);
    assert!(canonical(with_28).is_some());
    let trailing = parse_decimal("10000000000000000000000000000").unwrap();
    assert_eq!(significant_digits(trailing), 1);
    assert!(canonical(trailing).is_some());
}

fn dec(mantissa: i64, scale: u32) -> Decimal {
    Decimal::new(mantissa, scale)
}

#[test]
fn parse_decimal_keeps_trailing_zero_mantissa_with_a_negative_exponent() {
    assert_eq!(parse_decimal("150e-29"), Some(dec(15, 28)));
}

#[test]
fn parse_decimal_reads_ten_with_exponent_minus_29_as_the_smallest_scale() {
    assert_eq!(parse_decimal("10e-29"), Some(dec(1, 28)));
}

#[test]
fn parse_decimal_reads_100_with_exponent_minus_30_as_the_smallest_scale() {
    assert_eq!(parse_decimal("100e-30"), Some(dec(1, 28)));
}

#[test]
fn parse_decimal_accepts_29_fraction_zeros_after_a_one() {
    assert_eq!(
        parse_decimal("1.00000000000000000000000000000"),
        Some(Decimal::ONE)
    );
}

#[test]
fn parse_decimal_gives_zero_for_a_zero_mantissa_with_a_huge_exponent() {
    assert_eq!(parse_decimal("0e99999999999"), Some(Decimal::ZERO));
    assert_eq!(parse_decimal("-0e99999999999"), Some(Decimal::ZERO));
}

#[test]
fn parse_decimal_gives_zero_for_a_zero_mantissa_with_an_exponent_above_i64() {
    assert_eq!(parse_decimal("0e99999999999999999999"), Some(Decimal::ZERO));
    assert_eq!(
        parse_decimal("-0e-99999999999999999999"),
        Some(Decimal::ZERO)
    );
}

#[test]
fn parse_decimal_refuses_a_value_below_the_smallest_scale() {
    assert_eq!(parse_decimal("1e-29"), None);
}

#[test]
fn parse_decimal_refuses_a_value_above_the_range_of_decimal() {
    assert_eq!(parse_decimal("1e29"), None);
}

#[test]
fn parse_decimal_gives_zero_for_every_spelling_of_zero() {
    for text in ["0", "-0", "0.000", "0e-30", "0E+30"] {
        assert_eq!(parse_decimal(text), Some(Decimal::ZERO), "{text}");
    }
}

#[test]
fn parse_decimal_accepts_the_largest_decimal_in_plain_and_scientific_form() {
    assert_eq!(
        parse_decimal("79228162514264337593543950335"),
        Some(Decimal::MAX)
    );
    assert_eq!(
        parse_decimal("7.9228162514264337593543950335e28"),
        Some(Decimal::MAX)
    );
}

#[test]
fn parse_decimal_refuses_a_value_one_above_the_largest_decimal() {
    assert_eq!(parse_decimal("79228162514264337593543950336"), None);
}

#[test]
fn parse_decimal_refuses_a_29_digit_value_above_the_largest_decimal() {
    assert_eq!(parse_decimal("8e28"), None);
    assert_eq!(parse_decimal("99999999999999999999999999999"), None);
}

#[test]
fn a_number_with_29_significant_digits_parses_but_is_not_canonical() {
    let parsed = parse_decimal("123456789012345678901234567890e-1").unwrap();
    assert_eq!(parsed.to_string(), "12345678901234567890123456789");
    assert_eq!(significant_digits(parsed), 29);
    assert_eq!(canonical(parsed), None);
}

#[test]
fn a_real_leaf_refuses_a_number_with_29_significant_digits() {
    let tree = primitive_tree(REAL);
    let fault = ParameterValue::parse("123456789012345678901234567890e-1", &tree).unwrap_err();
    assert!(
        matches!(
            &fault,
            ValueFault::Mismatch {
                expected: Expected::NumberDigits { digits: 28, .. },
                ..
            }
        ),
        "{fault:?}"
    );
}

/// Parser at commit `47731e8`. It is the oracle of the differential property test.
fn head_parse_decimal(text: &str) -> Option<Decimal> {
    if !is_json_number(text) {
        return None;
    }
    match text.split_once(['e', 'E']) {
        None => Decimal::from_str_exact(text).ok(),
        Some((mantissa, exponent)) => {
            let base = Decimal::from_str_exact(mantissa).ok()?;
            head_apply_exponent(base, exponent)
        }
    }
}

/// Exponent step at commit `47731e8`.
fn head_apply_exponent(base: Decimal, exponent: &str) -> Option<Decimal> {
    let (is_negative, digits) = match exponent.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, exponent.strip_prefix('+').unwrap_or(exponent)),
    };
    let power: u32 = digits.parse().ok()?;
    if base.is_zero() {
        return Some(Decimal::ZERO);
    }
    if is_negative {
        let scale = base.normalize().scale().checked_add(power)?;
        let mut shifted = base.normalize();
        (scale <= 28).then_some(())?;
        shifted.set_scale(scale).ok()?;
        return Some(shifted);
    }
    let mut value = base;
    for _ in 0..power {
        value = value.checked_mul(Decimal::TEN)?;
    }
    Some(value)
}

proptest! {
    #[test]
    fn parse_decimal_shifts_the_exponent_by_the_appended_zeros(
        mantissa in any::<i64>().prop_filter("not zero", |m| *m != 0),
        scale in 0_usize..=28,
        zeros in 0_usize..=10,
    ) {
        let shift = scale.checked_add(zeros).unwrap();
        let text = format!("{mantissa}{}e-{shift}", "0".repeat(zeros));
        let expected = Decimal::new(mantissa, u32::try_from(scale).unwrap());
        prop_assert_eq!(parse_decimal(&text), Some(expected), "{}", text);
    }

    #[test]
    fn parse_decimal_agrees_with_the_parser_at_head_where_that_parser_accepts(
        text in r"-?(0|[1-9][0-9]{0,12})(\.[0-9]{1,14}0{0,16})?([eE][+-]?[0-9]{1,2})?",
    ) {
        if let Some(old) = head_parse_decimal(&text) {
            let new = parse_decimal(&text);
            prop_assert!(new.is_some(), "{} was accepted at HEAD", text);
            prop_assert_eq!(
                new.and_then(canonical).map(|value| value.to_string()),
                canonical(old).map(|value| value.to_string()),
                "{}",
                text
            );
        }
    }
}
