//! Slot number of an S7 connection (ADR-0154, design decision D2).

use super::range::{RangeReason, parse_in_range};

/// Highest slot number.
const MAX_SLOT: u16 = 31;

/// Slot number of an S7 PLC, from 0 to 31.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Slot(u8);

impl Slot {
    /// Parse the text of a slot field such as `0` or `31`.
    ///
    /// # Errors
    ///
    /// Returns the [`RangeReason`] that names the broken rule.
    pub fn parse(text: &str) -> Result<Self, RangeReason> {
        let number = parse_in_range(text, 0, MAX_SLOT)?;
        let value = u8::try_from(number).map_err(|_| RangeReason::OutOfRange {
            min: 0,
            max: MAX_SLOT,
        })?;
        Ok(Self(value))
    }

    /// Return the slot number.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests;
