//! The one parser of a TCP port (`http.public_port`, `http.internal_port`).
//!
//! The subcommand `serve` and the subcommand `healthcheck` read the same key
//! `http.internal_port`. Both use this parser, so they accept the same texts. A port is a whole
//! number from 1 to 65535. Port 0 is not a port of a listener.

use std::fmt;

use serde::Deserializer;
use serde::de::{self, Visitor};

/// The port of the internal listener when the operator sets no value (`http.internal_port`).
pub const DEFAULT_INTERNAL_PORT: u16 = 9000;

/// Read a port from a text of ASCII digits only.
///
/// The function returns `None` for port 0, for an empty text, for a sign or a space, and for a
/// number above 65535.
#[must_use]
pub fn parse_port(text: &str) -> Option<u16> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u16>().ok().filter(|port| *port != 0)
}

/// Read a port from a number or from a text of the figment provider.
///
/// A serde `deserialize_with` function. The environment provider gives a number, and a test
/// provider can give a text. Both pass the same rule as [`parse_port`]. The error text never
/// shows the rejected value. The error of figment adds the key.
///
/// # Errors
///
/// Returns a deserialization error when the value is not a port from 1 to 65535.
pub fn deserialize_port<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(PortVisitor)
}

/// The visitor of [`deserialize_port`].
struct PortVisitor;

impl PortVisitor {
    fn from_number<E: de::Error>(number: Option<u16>) -> Result<u16, E> {
        number
            .filter(|port| *port != 0)
            .ok_or_else(|| E::custom(NOT_A_PORT))
    }
}

/// The text of a refusal. It holds no value.
const NOT_A_PORT: &str = "the value is not a port number from 1 to 65535";

impl Visitor<'_> for PortVisitor {
    type Value = u16;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a port number from 1 to 65535")
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<u16, E> {
        Self::from_number(u16::try_from(value).ok())
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<u16, E> {
        Self::from_number(u16::try_from(value).ok())
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<u16, E> {
        Self::from_number(parse_port(value))
    }
}

#[cfg(test)]
mod tests;
