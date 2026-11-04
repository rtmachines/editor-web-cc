//! Exact parsing and canonical form of a JSON number (`reliability.md` section 4.4).

use rust_decimal::Decimal;

/// Largest number of significant digits of a numeric leaf (`reliability.md` section 4.4).
pub(super) const MAX_SIGNIFICANT_DIGITS: u32 = 28;

/// Largest scale of a `Decimal`.
const MAX_SCALE: u32 = 28;

/// Parse the text of a JSON number into an exact decimal.
///
/// The text must match the JSON number grammar. A text with more than 28 significant digits
/// or a value outside the range of `Decimal` gives `None`. The function never rounds.
pub(in crate::recipe) fn parse_decimal(text: &str) -> Option<Decimal> {
    if !is_json_number(text) {
        return None;
    }
    match text.split_once(['e', 'E']) {
        None => Decimal::from_str_exact(text).ok(),
        Some((mantissa, exponent)) => {
            let base = Decimal::from_str_exact(mantissa).ok()?;
            apply_exponent(base, exponent)
        }
    }
}

/// Return `true` when `text` matches `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`.
pub(super) fn is_json_number(text: &str) -> bool {
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (text, None),
    };
    let unsigned = mantissa.strip_prefix('-').unwrap_or(mantissa);
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (unsigned, None),
    };
    let is_whole = match whole.strip_prefix('0') {
        Some(rest) => rest.is_empty(),
        None => starts_with_nonzero_digit(whole),
    };
    is_whole
        && fraction.is_none_or(is_digits)
        && exponent.is_none_or(|digits| is_digits(strip_sign(digits)))
}

/// Remove one leading `+` or `-`.
fn strip_sign(text: &str) -> &str {
    text.strip_prefix(['+', '-']).unwrap_or(text)
}

/// Return `true` when `text` is one or more ASCII digits.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Return `true` when `text` is digits and the first digit is not zero.
fn starts_with_nonzero_digit(text: &str) -> bool {
    is_digits(text) && !text.starts_with('0')
}

/// Multiply `base` by ten to the power of the exponent text. Any overflow gives `None`.
fn apply_exponent(base: Decimal, exponent: &str) -> Option<Decimal> {
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
        (scale <= MAX_SCALE).then_some(())?;
        shifted.set_scale(scale).ok()?;
        return Some(shifted);
    }
    let mut value = base;
    for _ in 0..power {
        value = value.checked_mul(Decimal::TEN)?;
    }
    Some(value)
}

/// Return the number of significant digits of a decimal.
///
/// The count runs from the first to the last digit that is not zero. Leading zeros, the sign,
/// and the position of the decimal point do not count. Trailing zeros do not count, whether
/// they stand in the whole part or in the fraction. Thus `1000000000` and `1.500` have 1 and 2
/// digits. Zeros between two non-zero digits count. Zero has no significant digit.
///
/// The count uses `checked_add` and falls back to `u32::MAX`, which is above every limit.
/// An overflow cannot occur, but it would fail closed and keep the signature `u32`.
pub(in crate::recipe) fn significant_digits(decimal: Decimal) -> u32 {
    let mut mantissa = decimal.mantissa().unsigned_abs();
    while mantissa != 0 && mantissa.checked_rem(10) == Some(0) {
        mantissa = mantissa.checked_div(10).unwrap_or(0);
    }
    let Some(log) = mantissa.checked_ilog10() else {
        return 0;
    };
    let Some(count) = log.checked_add(1) else {
        return u32::MAX;
    };
    count
}

/// Return the canonical form of a decimal, or `None` above 28 significant digits.
///
/// Trailing zeros of the mantissa are not significant (see [`significant_digits`]).
pub(in crate::recipe) fn canonical(decimal: Decimal) -> Option<Decimal> {
    if decimal.is_zero() {
        return Some(Decimal::ZERO);
    }
    (significant_digits(decimal) <= MAX_SIGNIFICANT_DIGITS).then(|| decimal.normalize())
}
#[cfg(test)]
mod tests {
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
}
