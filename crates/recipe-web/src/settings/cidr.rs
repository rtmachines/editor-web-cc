//! A CIDR network and a list of networks, for `http.trusted_proxies` (`reliability.md` 7.11).

use std::fmt;
use std::net::IpAddr;

/// Width of an IPv4 address in bits.
const IPV4_BITS: u8 = 32;
/// Width of an IPv6 address in bits.
const IPV6_BITS: u8 = 128;
/// Prefix bits of the IPv4-mapped IPv6 block `::ffff:0:0/96`.
const MAPPED_PREFIX: u8 = 96;

/// The reason that a text is not a CIDR network.
///
/// The type is a reason of `Cidr::parse`, not an error type of its own (`reliability.md` 1.1).
/// The caller wraps it into its coded error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CidrReason {
    /// The text has no `/` or more than one `/`.
    MissingPrefix,
    /// The part before the `/` is not an IP address.
    BadAddress,
    /// The part after the `/` is not a number that fits the address family.
    BadPrefix,
}

impl fmt::Display for CidrReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingPrefix => "a network needs one prefix length after a slash",
            Self::BadAddress => "the network address is not an IP address",
            Self::BadPrefix => "the prefix length does not fit the address family",
        })
    }
}

/// One network such as `172.28.0.0/16`.
///
/// The type solves one problem: the trusted-proxy check must match an address against a
/// network without an external crate. Host bits of the written network are ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix: u8,
}

/// Return the address as a number and the width of its family in bits.
fn bits_of(address: IpAddr) -> (u128, u8) {
    match address {
        IpAddr::V4(v4) => (u128::from(v4.to_bits()), IPV4_BITS),
        IpAddr::V6(v6) => (v6.to_bits(), IPV6_BITS),
    }
}

impl Cidr {
    /// Parse the text `address/prefix`.
    ///
    /// # Errors
    ///
    /// Returns a [`CidrReason`] when the text has no single slash, a bad address, or a prefix
    /// that is not a decimal number up to the width of the family.
    pub fn parse(text: &str) -> Result<Self, CidrReason> {
        let (address, prefix) = text.split_once('/').ok_or(CidrReason::MissingPrefix)?;
        if prefix.contains('/') {
            return Err(CidrReason::MissingPrefix);
        }
        let network: IpAddr = address.parse().map_err(|_bad| CidrReason::BadAddress)?;
        if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
            return Err(CidrReason::BadPrefix);
        }
        let prefix: u8 = prefix.parse().map_err(|_bad| CidrReason::BadPrefix)?;
        let (_, width) = bits_of(network);
        if prefix > width {
            return Err(CidrReason::BadPrefix);
        }
        Ok(Self::canonical(network, prefix))
    }

    /// Map an IPv4-mapped IPv6 network with a prefix of 96 or more to its IPv4 network.
    ///
    /// A shorter prefix also covers addresses that are not mapped, so it has no IPv4 form.
    fn canonical(network: IpAddr, prefix: u8) -> Self {
        if let IpAddr::V6(v6) = network
            && let Some(v4) = v6.to_ipv4_mapped()
            && let Some(v4_prefix) = prefix.checked_sub(MAPPED_PREFIX)
        {
            return Self {
                network: IpAddr::V4(v4),
                prefix: v4_prefix,
            };
        }
        Self { network, prefix }
    }

    /// Return true when `address` lies inside the network.
    ///
    /// An address of the other family never matches. The caller maps an IPv4-mapped IPv6
    /// address to IPv4 first (design D11, step 4). `parse` already stores a mapped entry in its
    /// canonical IPv4 form, so both sides use one form.
    #[must_use]
    pub fn contains(&self, address: IpAddr) -> bool {
        let (wanted, width) = bits_of(self.network);
        let (given, given_width) = bits_of(address);
        if width != given_width {
            return false;
        }
        // The shift is at most the width of the family, so only a /0 network shifts everything out.
        let shift = u32::from(width.saturating_sub(self.prefix));
        let top = |value: u128| value.checked_shr(shift).unwrap_or(0);
        top(wanted) == top(given)
    }
}

/// A list of networks.
///
/// An address matches the list when it matches one network. The empty list matches nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CidrList(Vec<Cidr>);

impl CidrList {
    /// Parse every text of `texts`.
    ///
    /// # Errors
    ///
    /// Returns the [`CidrReason`] of the first text that is not a network.
    pub fn parse_all<I, S>(texts: I) -> Result<Self, CidrReason>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        texts
            .into_iter()
            .map(|text| Cidr::parse(text.as_ref()))
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }

    /// Return true when `address` lies inside one network of the list.
    #[must_use]
    pub fn contains(&self, address: IpAddr) -> bool {
        self.0.iter().any(|cidr| cidr.contains(address))
    }
}

#[cfg(test)]
mod tests;
