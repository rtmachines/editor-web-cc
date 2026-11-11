//! Address of a PLC on an IEC connection (ADR-0154, design decision D2).

use std::fmt;

use super::text::{ConnectionTextReason, check_text};

/// Highest number of characters of an address.
const MAX_ADDRESS_LENGTH: usize = 200;

/// Address of a PLC: 1 to 200 characters without a control character.
///
/// With the mode `Ads`, the address is also an `AmsNetId`. That rule belongs to the platform
/// shape and not to this type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlcAddress(String);

impl PlcAddress {
    /// Parse an address such as `192.168.1.10.1.1` or `PLC1`.
    ///
    /// # Errors
    ///
    /// Returns the [`ConnectionTextReason`] that names the first broken rule.
    pub fn parse(text: &str) -> Result<Self, ConnectionTextReason> {
        check_text(text, MAX_ADDRESS_LENGTH)?;
        Ok(Self(text.to_owned()))
    }

    /// Return the address as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlcAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
