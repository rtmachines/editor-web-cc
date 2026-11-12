//! Data block number of an S7 machine (ADR-0156, ADR-0154 (d)).

use crate::plant::connection::range::{RangeReason, parse_in_range};
use crate::plant::{MachineCode, PlantError, PlcPlatform};

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

/// Apply the data block rule of a live machine (ADR-0156).
///
/// An S7 platform needs a number that is not above the platform limit. An IEC platform has none.
/// The caller skips this check for an archived S7 machine that has no number.
///
/// # Errors
///
/// Returns `E-PLT-050` for an S7 machine without a number, `E-PLT-051` for an IEC machine with
/// a number, and `E-PLT-024` for a number above the platform limit.
pub fn check_data_block(
    code: MachineCode,
    platform: PlcPlatform,
    number: Option<DataBlockNumber>,
) -> Result<(), PlantError> {
    match (platform.max_data_block(), number) {
        (Some(_), None) => Err(PlantError::DataBlockMissing { code, platform }),
        (None, Some(_)) => Err(PlantError::DataBlockNotOnPlatform { code, platform }),
        (Some(max), Some(number)) if number.value() > max => Err(PlantError::DataBlockOutOfRange {
            code,
            number: number.value(),
            max,
        }),
        (Some(_), Some(_)) | (None, None) => Ok(()),
    }
}

#[cfg(test)]
mod tests;
