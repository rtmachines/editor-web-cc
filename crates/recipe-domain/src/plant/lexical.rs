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
mod tests;
