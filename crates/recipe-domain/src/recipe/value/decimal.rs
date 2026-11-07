//! Exact parsing and canonical form of a JSON number (`reliability.md` section 4.4).

use rust_decimal::Decimal;

/// Largest number of significant digits of a numeric leaf (`reliability.md` section 4.4).
pub(super) const MAX_SIGNIFICANT_DIGITS: u32 = 28;

/// Largest scale of a `Decimal`.
const MAX_SCALE: u32 = 28;

/// Largest count of digits that a `Decimal` can hold.
const MAX_DIGITS: usize = 29;

/// Parse the text of a JSON number into an exact decimal.
///
/// The text must match the JSON number grammar. The function gives `None` in three cases:
/// - The value is outside the range of `Decimal`.
/// - The text has more than 29 digits after the zeros at both ends.
/// - The scale of the value is outside -28 to 28.
///
/// A zero mantissa gives zero for every exponent. The function never rounds.
pub(in crate::recipe) fn parse_decimal(text: &str) -> Option<Decimal> {
    if !is_json_number(text) {
        return None;
    }
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (text, None),
    };
    let (is_negative, unsigned) = match mantissa.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, mantissa),
    };
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let all_digits = [whole, fraction].concat();
    let trimmed = all_digits.trim_start_matches('0');
    if trimmed.is_empty() {
        return Some(Decimal::ZERO);
    }
    let significant = trimmed.trim_end_matches('0');
    let removed_zeros = trimmed.len().checked_sub(significant.len())?;
    let scale = i64::try_from(fraction.len())
        .ok()?
        .checked_sub(i64::try_from(removed_zeros).ok()?)?
        .checked_sub(read_exponent(exponent)?)?;
    build_decimal(is_negative, significant, scale)
}

/// Read the optional exponent text into `i64`. An overflow gives `None`.
fn read_exponent(exponent: Option<&str>) -> Option<i64> {
    let Some(text) = exponent else {
        return Some(0);
    };
    let (is_negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let magnitude: i64 = digits.parse().ok()?;
    if is_negative {
        magnitude.checked_neg()
    } else {
        Some(magnitude)
    }
}

/// Build the decimal `digits` times ten to the power of minus `scale`.
///
/// `digits` holds ASCII digits only. A scale outside -28 to 28 or a value outside the range of
/// `Decimal` gives `None`.
fn build_decimal(is_negative: bool, digits: &str, scale: i64) -> Option<Decimal> {
    if digits.len() > MAX_DIGITS {
        return None;
    }
    let mut value: i128 = digits.parse().ok()?;
    if is_negative {
        value = value.checked_neg()?;
    }
    let max_scale = i64::from(MAX_SCALE);
    if scale > max_scale || scale < max_scale.checked_neg()? {
        return None;
    }
    let scale = match u32::try_from(scale) {
        Ok(scale) => scale,
        Err(_) => {
            for _ in 0..scale.unsigned_abs() {
                value = value.checked_mul(10)?;
            }
            0
        }
    };
    Decimal::try_from_i128_with_scale(value, scale).ok()
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
mod tests;
