//! Tests of the shared port parser.

use super::{DEFAULT_INTERNAL_PORT, parse_port};

#[test]
fn a_port_is_one_to_65535_in_ascii_digits() {
    assert_eq!(parse_port("1"), Some(1));
    assert_eq!(parse_port("9000"), Some(9000));
    assert_eq!(parse_port("65535"), Some(65_535));
    for bad in [
        "",
        "0",
        "00",
        "65536",
        "70000",
        " 9000",
        "9000 ",
        "+9000",
        "-1",
        "9 0",
        "abc",
        "９０００",
    ] {
        assert_eq!(parse_port(bad), None, "accepted {bad:?}");
    }
}

#[test]
fn the_default_internal_port_is_9000() {
    assert_eq!(DEFAULT_INTERNAL_PORT, 9000);
}
