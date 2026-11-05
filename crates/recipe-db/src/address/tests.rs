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
