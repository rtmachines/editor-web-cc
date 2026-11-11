//! AMS Net ID of a TwinCAT ADS connection (ADR-0154, design decision D2).

use std::fmt;

/// Number of numbers in an AMS Net ID.
const AMS_NET_ID_NUMBERS: usize = 6;

/// Reason why a text is not an AMS Net ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmsNetIdReason {
    /// The text does not have six numbers with a dot between two numbers.
    Count,
    /// A number is empty, has a character that is not a digit, has a leading zero,
    /// or is above 255.
    Number,
}

impl fmt::Display for AmsNetIdReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Count => {
                f.write_str("an AMS Net ID has six numbers with a dot between two numbers")
            }
            Self::Number => {
                f.write_str("a number of an AMS Net ID is from 0 to 255 without a leading zero")
            }
        }
    }
}

/// AMS Net ID: six numbers from 0 to 255 with a dot between two numbers.
///
/// A number has no leading zero.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AmsNetId(String);

impl AmsNetId {
    /// Parse an AMS Net ID such as `192.168.1.10.1.1`.
    ///
    /// # Errors
    ///
    /// Returns the [`AmsNetIdReason`] that names the first broken rule.
    pub fn parse(text: &str) -> Result<Self, AmsNetIdReason> {
        if text.split('.').count() != AMS_NET_ID_NUMBERS {
            return Err(AmsNetIdReason::Count);
        }
        if !text.split('.').all(is_number) {
            return Err(AmsNetIdReason::Number);
        }
        Ok(Self(text.to_owned()))
    }

    /// Return the AMS Net ID as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AmsNetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Tell whether a text is a number from 0 to 255 of ASCII digits without a leading zero.
fn is_number(text: &str) -> bool {
    let has_leading_zero = text.len() > 1 && text.starts_with('0');
    text.bytes().all(|b| b.is_ascii_digit()) && !has_leading_zero && text.parse::<u8>().is_ok()
}

#[cfg(test)]
mod tests;
