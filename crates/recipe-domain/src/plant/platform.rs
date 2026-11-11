//! PLC platform of a machine (`domain-model.md` section 2.1).

use std::fmt;

/// PLC platform of a machine. Each machine has exactly one platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlcPlatform {
    /// CODESYS controller with IEC 61131-3 programming.
    CodesysIec,
    /// Beckhoff TwinCAT 3 controller.
    TwinCat3,
    /// Siemens S7-1200 controller.
    S7_1200,
    /// Siemens S7-1500 controller.
    S7_1500,
    /// Siemens S7-300 controller.
    S7_300,
}

/// Reason why a text is not a PLC platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlcPlatformReason {
    /// The text is not one of the five platform names.
    Unknown,
}

impl fmt::Display for PlcPlatformReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => f.write_str("the text is not a PLC platform"),
        }
    }
}

impl PlcPlatform {
    /// Every PLC platform, in the order of `domain-model.md` section 2.1.
    pub const ALL: [Self; 5] = [
        Self::CodesysIec,
        Self::TwinCat3,
        Self::S7_1200,
        Self::S7_1500,
        Self::S7_300,
    ];

    /// Parse the SQL value of a platform, such as `S7_1500`.
    ///
    /// # Errors
    ///
    /// Returns [`PlcPlatformReason::Unknown`] for any other text.
    pub fn parse(text: &str) -> Result<Self, PlcPlatformReason> {
        Self::ALL
            .into_iter()
            .find(|platform| platform.as_str() == text)
            .ok_or(PlcPlatformReason::Unknown)
    }

    /// Return true when the platform is a Siemens S7 controller.
    #[must_use]
    pub const fn is_s7(self) -> bool {
        match self {
            Self::S7_300 | Self::S7_1200 | Self::S7_1500 => true,
            Self::CodesysIec | Self::TwinCat3 => false,
        }
    }

    /// Return the highest data block number of this platform, or `None` when the platform has
    /// no S7 data blocks.
    ///
    /// The number is 16000 on `S7_300` (ADR-0116 (c)) and 59999 on `S7_1200` and `S7_1500`
    /// (ADR-0140 (a)). The number is a fact of the platform family, not a setting.
    #[must_use]
    pub const fn max_data_block(self) -> Option<u16> {
        match self {
            Self::S7_300 => Some(16_000),
            Self::S7_1200 | Self::S7_1500 => Some(59_999),
            Self::CodesysIec | Self::TwinCat3 => None,
        }
    }

    /// Return the SQL value of this platform.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CodesysIec => "CodesysIec",
            Self::TwinCat3 => "TwinCat3",
            Self::S7_1200 => "S7_1200",
            Self::S7_1500 => "S7_1500",
            Self::S7_300 => "S7_300",
        }
    }
}

impl fmt::Display for PlcPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests;
