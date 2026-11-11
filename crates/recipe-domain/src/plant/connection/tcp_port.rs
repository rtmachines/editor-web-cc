//! TCP port of an IEC connection (ADR-0154, design decision D2).

use super::range::{RangeReason, parse_in_range};

/// Lowest TCP port.
const MIN_PORT: u16 = 1;
/// Highest TCP port.
const MAX_PORT: u16 = 65535;

/// TCP port of a PLC, from 1 to 65535.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TcpPort(u16);

impl TcpPort {
    /// Parse the text of a port field such as `102` or `65535`.
    ///
    /// # Errors
    ///
    /// Returns the [`RangeReason`] that names the broken rule.
    pub fn parse(text: &str) -> Result<Self, RangeReason> {
        parse_in_range(text, MIN_PORT, MAX_PORT).map(Self)
    }

    /// Return the port number.
    #[must_use]
    pub fn value(self) -> u16 {
        self.0
    }
}

#[cfg(test)]
mod tests;
