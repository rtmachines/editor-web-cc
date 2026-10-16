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

/// Return the canonical form of a decimal, or `None` above 28 significant digits.
///
/// Trailing zeros of the mantissa are not significant.
pub(in crate::recipe) fn canonical(decimal: Decimal) -> Option<Decimal> {
    if decimal.is_zero() {
        return Some(Decimal::ZERO);
    }
    let normalized = decimal.normalize();
    let mut mantissa = normalized.mantissa().unsigned_abs();
    while mantissa.checked_rem(10) == Some(0) {
        mantissa = mantissa.checked_div(10)?;
    }
    let digits = mantissa.checked_ilog10()?.checked_add(1)?;
    (digits <= MAX_SIGNIFICANT_DIGITS).then_some(normalized)
}
