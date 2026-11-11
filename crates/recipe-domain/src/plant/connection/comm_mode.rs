//! Mode of a PLC connection (ADR-0154, design decision D2).

use std::fmt;

/// Reason why a text is not a communication mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeReason {
    /// The text is none of the four mode names, or it has another case.
    Unknown,
}

impl fmt::Display for ModeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => {
                f.write_str("a mode is Ads, OpcUa, CodesysV3, or ModbusTcp with this exact case")
            }
        }
    }
}

/// Communication mode of a PLC with an IEC connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommMode {
    /// Beckhoff ADS.
    Ads,
    /// OPC UA.
    OpcUa,
    /// CODESYS V3.
    CodesysV3,
    /// Modbus TCP.
    ModbusTcp,
}

impl CommMode {
    /// Parse a mode name. The name is exact and case-sensitive.
    ///
    /// # Errors
    ///
    /// Returns [`ModeReason::Unknown`] when the text is none of the four names.
    pub fn parse(text: &str) -> Result<Self, ModeReason> {
        match text {
            "Ads" => Ok(Self::Ads),
            "OpcUa" => Ok(Self::OpcUa),
            "CodesysV3" => Ok(Self::CodesysV3),
            "ModbusTcp" => Ok(Self::ModbusTcp),
            _ => Err(ModeReason::Unknown),
        }
    }

    /// Return the name that [`CommMode::parse`] accepts.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ads => "Ads",
            Self::OpcUa => "OpcUa",
            Self::CodesysV3 => "CodesysV3",
            Self::ModbusTcp => "ModbusTcp",
        }
    }
}

impl fmt::Display for CommMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests;
