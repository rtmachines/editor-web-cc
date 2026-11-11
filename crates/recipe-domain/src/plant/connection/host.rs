//! Host of a PLC connection (ADR-0154, design decision D2).

use std::fmt;

/// Longest length of a host in characters (RFC 1123).
const MAX_HOST_LENGTH: usize = 253;

/// Longest length of a label of a host name in characters (RFC 1123).
const MAX_LABEL_LENGTH: usize = 63;

/// Number of numbers in an IPv4 address.
const IPV4_NUMBERS: usize = 4;

/// Reason why a text is not a PLC host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostReason {
    /// The text is empty.
    Empty,
    /// The text is longer than 253 characters.
    TooLong,
    /// The text has only digits and dots, but it is not an IPv4 address in dotted decimal form.
    Ipv4,
    /// A label of the host name is empty, longer than 63 characters, or has a bad character.
    Label,
    /// The host name ends with a dot.
    TrailingDot,
}

impl fmt::Display for HostReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a host is not empty"),
            Self::TooLong => write!(f, "a host has at most {MAX_HOST_LENGTH} characters"),
            Self::Ipv4 => f.write_str(
                "a host of digits and dots is four numbers from 0 to 255 without a leading zero",
            ),
            Self::Label => f.write_str(
                "a host name has labels of 1 to 63 letters, digits, or hyphens, and a label does not start or end with a hyphen",
            ),
            Self::TrailingDot => f.write_str("a host name does not end with a dot"),
        }
    }
}

/// Host of a PLC: an IPv4 address in dotted decimal form, or an RFC 1123 host name
/// of at most 253 characters (ADR-0154, design decision D2).
///
/// A text of digits and dots only is an IPv4 address. A leading zero is not valid.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlcHost(String);

impl PlcHost {
    /// Parse a PLC host such as `192.168.1.20` or `plc-line1.plant.local`.
    ///
    /// # Errors
    ///
    /// Returns the [`HostReason`] that names the first broken rule.
    pub fn parse(text: &str) -> Result<Self, HostReason> {
        if text.is_empty() {
            return Err(HostReason::Empty);
        }
        if text.len() > MAX_HOST_LENGTH {
            return Err(HostReason::TooLong);
        }
        if text.ends_with('.') {
            return Err(HostReason::TrailingDot);
        }
        let is_number_text = text.bytes().all(|b| b.is_ascii_digit() || b == b'.');
        let is_valid = if is_number_text {
            is_ipv4(text)
        } else {
            text.split('.').all(is_label)
        };
        match (is_valid, is_number_text) {
            (true, _) => Ok(Self(text.to_owned())),
            (false, true) => Err(HostReason::Ipv4),
            (false, false) => Err(HostReason::Label),
        }
    }

    /// Return the host as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlcHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Tell whether a text is four numbers from 0 to 255 in dotted decimal form.
///
/// A number has 1 to 3 digits and no leading zero.
fn is_ipv4(text: &str) -> bool {
    text.split('.').count() == IPV4_NUMBERS && text.split('.').all(is_ipv4_number)
}

fn is_ipv4_number(text: &str) -> bool {
    let has_leading_zero = text.len() > 1 && text.starts_with('0');
    (1..=3).contains(&text.len()) && !has_leading_zero && text.parse::<u8>().is_ok()
}

/// Tell whether a text is a label of an RFC 1123 host name.
fn is_label(text: &str) -> bool {
    (1..=MAX_LABEL_LENGTH).contains(&text.len())
        && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !text.starts_with('-')
        && !text.ends_with('-')
}

#[cfg(test)]
mod tests;
