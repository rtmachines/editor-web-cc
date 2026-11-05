use rust_decimal::Decimal;

use super::{canonical, parse_decimal, significant_digits};

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
