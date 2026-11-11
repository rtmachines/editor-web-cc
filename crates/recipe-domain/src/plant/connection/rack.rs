//! Rack number of an S7 connection (ADR-0154, design decision D2).

use super::range::{RangeReason, parse_in_range};

/// Highest rack number.
const MAX_RACK: u16 = 7;

/// Rack number of an S7 PLC, from 0 to 7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rack(u8);

impl Rack {
    /// Parse the text of a rack field such as `0` or `7`.
    ///
    /// # Errors
    ///
    /// Returns the [`RangeReason`] that names the broken rule.
    pub fn parse(text: &str) -> Result<Self, RangeReason> {
        let number = parse_in_range(text, 0, MAX_RACK)?;
        let value = u8::try_from(number).map_err(|_| RangeReason::OutOfRange {
            min: 0,
            max: MAX_RACK,
        })?;
        Ok(Self(value))
    }

    /// Return the rack number.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests;
