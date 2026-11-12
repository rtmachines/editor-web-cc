//! Reason type shared by the bounded number value objects of a PLC connection.

use std::fmt;

/// Reason why a text is not a number in the range of a connection field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeReason {
    /// The text is not a whole number of ASCII digits.
    NotANumber,
    /// The number is below the lowest or above the highest permitted value.
    OutOfRange {
        /// Lowest permitted value.
        min: u16,
        /// Highest permitted value.
        max: u16,
    },
}

impl fmt::Display for RangeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotANumber => f.write_str("a value is a whole number of digits"),
            Self::OutOfRange { min, max } => {
                write!(f, "a value is a number from {min} to {max}")
            }
        }
    }
}

/// Parse a whole number of ASCII digits that lies between `min` and `max`.
pub(crate) fn parse_in_range(text: &str, min: u16, max: u16) -> Result<u16, RangeReason> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(RangeReason::NotANumber);
    }
    let out_of_range = RangeReason::OutOfRange { min, max };
    let number: u16 = text.parse().map_err(|_| out_of_range)?;
    if (min..=max).contains(&number) {
        Ok(number)
    } else {
        Err(out_of_range)
    }
}

#[cfg(test)]
mod tests;
