//! Behavior guards for the edge inputs of `indent_of` and of the address plan check.
//!
//! These tests pass before and after the removal of saturating arithmetic. They show
//! that the change keeps the behavior of an empty line, a blank line, and a prefix of 0 or 32.

use std::collections::BTreeMap;

use crate::lines::indent_of;
use crate::topology::check_address_plan;

#[test]
fn indent_of_counts_the_leading_white_space_of_an_edge_line() {
    let cases = [
        ("", 0),
        ("x", 0),
        ("  x", 2),
        ("  ", 2),
        ("\t x", 2),
        ("\u{a0}x", 2),
        ("  x  ", 2),
    ];
    for (line, expected) in cases {
        assert_eq!(indent_of(line), expected, "line: {line:?}");
    }
}

fn plan_faults(subnet: &str, range: &str, proxy: &str) -> Vec<String> {
    let env = BTreeMap::from([
        ("COMPOSE_SUBNET", subnet),
        ("COMPOSE_IP_RANGE", range),
        ("PROXY_ADDRESS", proxy),
    ]);
    check_address_plan(&env)
        .faults
        .iter()
        .map(|fault| fault.key.clone())
        .collect()
}

#[test]
fn a_prefix_of_zero_covers_every_address() {
    assert!(plan_faults("0.0.0.0/0", "10.0.0.0/8", "192.168.0.1").is_empty());
}

#[test]
fn a_prefix_of_32_covers_one_address_only() {
    let keys = plan_faults("10.0.0.1/32", "10.0.0.1/32", "10.0.0.2");
    assert_eq!(
        keys,
        ["PROXY_ADDRESS"],
        "proxy outside the subnet of one address"
    );
    let keys = plan_faults("10.0.0.1/32", "10.0.0.1/32", "10.0.0.1");
    assert_eq!(keys, ["PROXY_ADDRESS"], "proxy inside the range");
}
