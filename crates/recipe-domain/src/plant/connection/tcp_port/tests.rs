use super::*;

const OUT_OF_RANGE: RangeReason = RangeReason::OutOfRange { min: 1, max: 65535 };

#[test]
fn tcp_port_boundaries_pass() {
    assert_eq!(TcpPort::parse("1").map(TcpPort::value), Ok(1));
    assert_eq!(TcpPort::parse("65535").map(TcpPort::value), Ok(65535));
    assert_eq!(TcpPort::parse("102").map(TcpPort::value), Ok(102));
}

#[test]
fn tcp_port_zero_fails() {
    assert_eq!(TcpPort::parse("0"), Err(OUT_OF_RANGE));
}

#[test]
fn tcp_port_above_the_range_fails() {
    assert_eq!(TcpPort::parse("65536"), Err(OUT_OF_RANGE));
    assert_eq!(TcpPort::parse("99999999999999999999"), Err(OUT_OF_RANGE));
}

#[test]
fn tcp_port_that_is_not_a_number_fails() {
    for text in ["-1", "x", "", " 1", "1 ", "+1", "1.0", "0x1", "\u{0663}"] {
        assert_eq!(
            TcpPort::parse(text),
            Err(RangeReason::NotANumber),
            "{text:?}"
        );
    }
}
