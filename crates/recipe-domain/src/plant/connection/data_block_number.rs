//! Data block number of an S7 connection (ADR-0154, design decision D2).

use super::range::{RangeReason, parse_in_range};

/// Lowest data block number.
const MIN_DATA_BLOCK: u16 = 1;
/// Highest data block number that this type accepts. The platform limit is a separate rule.
const MAX_DATA_BLOCK: u16 = 65535;

/// Number of the data block that holds the recipe on an S7 PLC, from 1 to 65535.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DataBlockNumber(u16);

impl DataBlockNumber {
    /// Parse the text of a data block field such as `1` or `65535`.
    ///
    /// # Errors
    ///
    /// Returns the [`RangeReason`] that names the broken rule.
    pub fn parse(text: &str) -> Result<Self, RangeReason> {
        parse_in_range(text, MIN_DATA_BLOCK, MAX_DATA_BLOCK).map(Self)
    }

    /// Return the data block number.
    #[must_use]
    pub fn value(self) -> u16 {
        self.0
    }
}

#[cfg(test)]
mod tests;
