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
mod tests {
    use super::*;

    #[test]
    fn codesys_iec_round_trips_through_parse_and_display() {
        assert_round_trip(PlcPlatform::CodesysIec, "CodesysIec");
    }

    #[test]
    fn twincat3_round_trips_through_parse_and_display() {
        assert_round_trip(PlcPlatform::TwinCat3, "TwinCat3");
    }

    #[test]
    fn s7_1200_round_trips_through_parse_and_display() {
        assert_round_trip(PlcPlatform::S7_1200, "S7_1200");
    }

    #[test]
    fn s7_1500_round_trips_through_parse_and_display() {
        assert_round_trip(PlcPlatform::S7_1500, "S7_1500");
    }

    #[test]
    fn s7_300_round_trips_through_parse_and_display() {
        assert_round_trip(PlcPlatform::S7_300, "S7_300");
    }

    #[test]
    fn hyphenated_s7_name_is_rejected() {
        assert_eq!(
            PlcPlatform::parse("S7-1500"),
            Err(PlcPlatformReason::Unknown)
        );
    }

    #[test]
    fn lower_case_name_is_rejected() {
        assert_eq!(
            PlcPlatform::parse("twincat3"),
            Err(PlcPlatformReason::Unknown)
        );
    }

    #[test]
    fn empty_text_is_rejected() {
        assert_eq!(PlcPlatform::parse(""), Err(PlcPlatformReason::Unknown));
    }

    fn assert_round_trip(platform: PlcPlatform, text: &str) {
        assert_eq!(PlcPlatform::parse(text), Ok(platform));
        assert_eq!(platform.to_string(), text);
        assert_eq!(platform.as_str(), text);
    }
}
