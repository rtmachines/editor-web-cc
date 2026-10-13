//! Lexical helpers that the text parsers of the Plant Model share.
//!
//! The helpers solve one problem: three parsers read the same digit and list
//! forms, and a copy in each parser can drift.

/// Why `parse_integer` rejects a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntegerError {
    /// The text is not `-?[0-9]{1,10}`.
    Grammar,
    /// The number does not fit the INTEGER range.
    OutOfRange,
}

/// Return `true` for 1 to `max_length` ASCII digits.
pub(crate) fn is_digits(text: &str, max_length: usize) -> bool {
    !text.is_empty() && text.len() <= max_length && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Return `true` for 1 to `max_length` ASCII digits without a leading zero.
pub(crate) fn is_canonical_digits(text: &str, max_length: usize) -> bool {
    is_digits(text, max_length) && !text.starts_with('0')
}

/// Parse an INTEGER written as `-?[0-9]{1,10}`.
///
/// # Errors
///
/// Returns [`IntegerError::Grammar`] for a text of another form and
/// [`IntegerError::OutOfRange`] for a number outside the INTEGER range.
pub(crate) fn parse_integer(text: &str) -> Result<i32, IntegerError> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if !is_digits(digits, 10) {
        return Err(IntegerError::Grammar);
    }
    let value: i64 = text.parse().map_err(|_| IntegerError::Grammar)?;
    i32::try_from(value).map_err(|_| IntegerError::OutOfRange)
}

/// Split `text` at `separator` into at most `max_parts` parts.
///
/// The function reads at most `max_parts + 1` parts, so a long text costs no
/// more than a short one. It returns `None` when the text has more parts.
pub(crate) fn split_at_most(text: &str, separator: char, max_parts: usize) -> Option<Vec<&str>> {
    let limit = max_parts.checked_add(1)?;
    let parts: Vec<&str> = text.splitn(limit, separator).collect();
    (parts.len() <= max_parts).then_some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_need_one_to_the_maximum_length() {
        assert!(is_digits("0", 5));
        assert!(is_digits("00007", 5));
        assert!(!is_digits("", 5));
        assert!(!is_digits("123456", 5));
        assert!(!is_digits("12a", 5));
        assert!(!is_digits("-1", 5));
    }

    #[test]
    fn canonical_digits_reject_a_leading_zero() {
        assert!(is_canonical_digits("10", 5));
        assert!(!is_canonical_digits("0", 5));
        assert!(!is_canonical_digits("01", 5));
    }

    #[test]
    fn integer_accepts_the_integer_range() {
        assert_eq!(parse_integer("0"), Ok(0));
        assert_eq!(parse_integer("-0"), Ok(0));
        assert_eq!(parse_integer("007"), Ok(7));
        assert_eq!(parse_integer("2147483647"), Ok(i32::MAX));
        assert_eq!(parse_integer("-2147483648"), Ok(i32::MIN));
    }

    #[test]
    fn integer_above_the_range_is_out_of_range() {
        assert_eq!(parse_integer("2147483648"), Err(IntegerError::OutOfRange));
        assert_eq!(parse_integer("-2147483649"), Err(IntegerError::OutOfRange));
    }

    #[test]
    fn integer_of_another_form_is_a_grammar_error() {
        for text in ["", "-", "+1", "1.0", "12345678901", "--1", "1a", " 1"] {
            assert_eq!(parse_integer(text), Err(IntegerError::Grammar), "{text}");
        }
    }

    #[test]
    fn split_keeps_a_list_within_the_limit() {
        assert_eq!(split_at_most("a", ',', 3), Some(vec!["a"]));
        assert_eq!(split_at_most("a,b,c", ',', 3), Some(vec!["a", "b", "c"]));
        assert_eq!(split_at_most("", ',', 3), Some(vec![""]));
    }

    #[test]
    fn split_rejects_a_list_above_the_limit() {
        assert_eq!(split_at_most("a,b,c,d", ',', 3), None);
        assert_eq!(split_at_most(&",".repeat(100_000), ',', 3), None);
    }
}
