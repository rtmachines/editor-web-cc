//! Length of a `STRING(n)` or a `WSTRING(n)` (`domain-model.md` section 2.1).

use std::fmt;

use super::platform::PlcPlatform;

/// Kind of a string type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StringKind {
    /// `STRING`, one byte per character.
    String,
    /// `WSTRING`, two bytes per character.
    Wstring,
}

/// Number of characters of a string type. The range depends on the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringLength(u16);

/// Reason why a string length is not valid on a platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringLengthReason {
    /// The string kind does not exist on the platform, such as `WSTRING` on `S7_300`.
    NotOnPlatform,
    /// The length is outside the range of the platform.
    OutOfRange,
}

impl fmt::Display for StringLengthReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOnPlatform => f.write_str("the string type is not available on this platform"),
            Self::OutOfRange => {
                f.write_str("the string length is outside the range of this platform")
            }
        }
    }
}

impl StringLength {
    /// Parse the length of a string type on a platform.
    ///
    /// # Errors
    ///
    /// Returns [`StringLengthReason::NotOnPlatform`] for `WSTRING` on `S7_300`.
    /// Returns [`StringLengthReason::OutOfRange`] for a length outside the range.
    pub fn parse(
        kind: StringKind,
        platform: PlcPlatform,
        length: u32,
    ) -> Result<Self, StringLengthReason> {
        let max = max_length(kind, platform).ok_or(StringLengthReason::NotOnPlatform)?;
        let length = u16::try_from(length).map_err(|_| StringLengthReason::OutOfRange)?;
        if (1..=max).contains(&length) {
            Ok(Self(length))
        } else {
            Err(StringLengthReason::OutOfRange)
        }
    }

    /// Return the number of characters.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Return the default length of a string type on a platform, or `None`
    /// when the string type is not available on the platform.
    #[must_use]
    pub const fn default_length(kind: StringKind, platform: PlcPlatform) -> Option<u16> {
        match (kind, platform) {
            (StringKind::String, PlcPlatform::CodesysIec | PlcPlatform::TwinCat3)
            | (StringKind::Wstring, PlcPlatform::CodesysIec | PlcPlatform::TwinCat3) => Some(80),
            (StringKind::String, _)
            | (StringKind::Wstring, PlcPlatform::S7_1200 | PlcPlatform::S7_1500) => Some(254),
            (StringKind::Wstring, PlcPlatform::S7_300) => None,
        }
    }

    /// Return the S7 size of the string in bytes. `STRING(n)` is `n + 2`
    /// and `WSTRING(n)` is `2n + 4`. The result is `None` only when the
    /// length is not valid, because a valid length cannot overflow `u32`.
    #[must_use]
    pub fn size_bytes(self, kind: StringKind) -> Option<u32> {
        let length = u32::from(self.0);
        match kind {
            StringKind::String => length.checked_add(2),
            StringKind::Wstring => length.checked_mul(2)?.checked_add(4),
        }
    }
}

/// Return the largest length of a string type on a platform (`domain-model.md` section 2.1).
///
/// The smallest length is 1 on every platform. `None` means the type is not available.
const fn max_length(kind: StringKind, platform: PlcPlatform) -> Option<u16> {
    match (kind, platform) {
        (StringKind::String, PlcPlatform::CodesysIec | PlcPlatform::TwinCat3)
        | (StringKind::Wstring, PlcPlatform::CodesysIec | PlcPlatform::TwinCat3) => Some(255),
        (StringKind::String, PlcPlatform::S7_300 | PlcPlatform::S7_1200 | PlcPlatform::S7_1500) => {
            Some(254)
        }
        (StringKind::Wstring, PlcPlatform::S7_1200 | PlcPlatform::S7_1500) => Some(16_382),
        (StringKind::Wstring, PlcPlatform::S7_300) => None,
    }
}

impl fmt::Display for StringLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests;
