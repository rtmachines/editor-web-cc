use std::net::IpAddr;

use super::*;

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[test]
fn ipv4_network_contains_its_hosts_only() {
    let cidr = Cidr::parse("172.28.0.0/16").unwrap();
    assert!(cidr.contains(ip("172.28.0.2")));
    assert!(cidr.contains(ip("172.28.255.255")));
    assert!(!cidr.contains(ip("172.29.0.1")));
    assert!(!cidr.contains(ip("8.8.8.8")));
}

#[test]
fn prefix_zero_matches_every_address_of_the_family() {
    let cidr = Cidr::parse("0.0.0.0/0").unwrap();
    assert!(cidr.contains(ip("203.0.113.9")));
    assert!(!cidr.contains(ip("::1")));
}

#[test]
fn prefix_32_matches_one_address() {
    let cidr = Cidr::parse("10.1.2.3/32").unwrap();
    assert!(cidr.contains(ip("10.1.2.3")));
    assert!(!cidr.contains(ip("10.1.2.4")));
}

#[test]
fn host_bits_of_the_network_are_ignored() {
    let cidr = Cidr::parse("172.28.7.9/16").unwrap();
    assert!(cidr.contains(ip("172.28.0.1")));
}

#[test]
fn ipv6_network_contains_its_hosts_only() {
    let cidr = Cidr::parse("fd00::/8").unwrap();
    assert!(cidr.contains(ip("fd12:3456::1")));
    assert!(!cidr.contains(ip("fe80::1")));
    assert!(!cidr.contains(ip("10.0.0.1")));
}

#[test]
fn bad_text_is_rejected() {
    for text in [
        "",
        "172.28.0.0",
        "172.28.0.0/33",
        "::/129",
        "x/8",
        "1.2.3.4/-1",
        "1.2.3.4/",
        "1.2.3.4/8/8",
        "1.2.3.4/ 8",
    ] {
        assert!(Cidr::parse(text).is_err(), "{text:?} must not parse");
    }
}

#[test]
fn list_matches_when_any_entry_matches() {
    let list = CidrList::parse_all(["10.0.0.0/8", "192.168.0.0/16"]).unwrap();
    assert!(list.contains(ip("192.168.4.4")));
    assert!(list.contains(ip("10.9.9.9")));
    assert!(!list.contains(ip("172.16.0.1")));
}

#[test]
fn empty_list_matches_nothing() {
    let list = CidrList::default();
    assert!(!list.contains(ip("127.0.0.1")));
}
