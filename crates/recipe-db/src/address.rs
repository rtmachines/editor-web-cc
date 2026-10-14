//! The text form of a client address for the `inet` columns (ADR-0035 item p).

use std::net::IpAddr;

/// Return the canonical text of a client address, ready for the cast `::inet`.
///
/// The function solves one problem: `::ffff:10.0.0.5` and `10.0.0.5` are one client. PostgreSQL
/// keeps them as two different `inet` values, so the throttle count would split. Every adapter
/// that binds a client address calls this function.
pub(crate) fn canonical_text(address: IpAddr) -> String {
    address.to_canonical().to_string()
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::canonical_text;

    #[test]
    fn a_mapped_ipv4_address_gives_the_ipv4_text() {
        let mapped = IpAddr::V6(Ipv4Addr::new(10, 0, 0, 5).to_ipv6_mapped());
        assert_eq!(canonical_text(mapped), "10.0.0.5");
    }

    #[test]
    fn a_plain_ipv6_address_keeps_its_text() {
        let address: IpAddr = "2001:db8::1".parse().unwrap();
        assert_eq!(canonical_text(address), "2001:db8::1");
    }
}
