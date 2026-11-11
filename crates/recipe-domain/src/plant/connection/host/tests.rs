use super::*;

/// A host name of three labels of 63 characters and a last label of `tail_length` characters.
fn name_of(tail_length: usize) -> String {
    let label = "a".repeat(63);
    let tail = "a".repeat(tail_length);
    format!("{label}.{label}.{label}.{tail}")
}

#[test]
fn ipv4_address_passes() {
    assert_eq!(
        PlcHost::parse("192.168.1.20").map(|h| h.as_str().to_owned()),
        Ok("192.168.1.20".to_owned())
    );
}

#[test]
fn ipv4_edge_numbers_pass() {
    for text in ["0.0.0.0", "255.255.255.255", "10.0.0.5"] {
        assert!(PlcHost::parse(text).is_ok(), "{text}");
    }
}

#[test]
fn host_name_passes() {
    for text in [
        "plc-line1.plant.local",
        "plc",
        "PLC-1",
        "a1.b2",
        "1plc",
        "plc.1",
    ] {
        assert!(PlcHost::parse(text).is_ok(), "{text}");
    }
}

#[test]
fn display_prints_the_text() {
    let host = PlcHost::parse("plc-line1.plant.local");
    assert_eq!(
        host.map(|h| h.to_string()),
        Ok("plc-line1.plant.local".to_owned())
    );
}

#[test]
fn empty_text_fails() {
    assert_eq!(PlcHost::parse(""), Err(HostReason::Empty));
}

#[test]
fn number_above_255_fails_as_ipv4() {
    assert_eq!(PlcHost::parse("192.168.1.300"), Err(HostReason::Ipv4));
}

#[test]
fn leading_zero_fails_as_ipv4() {
    assert_eq!(PlcHost::parse("010.0.0.1"), Err(HostReason::Ipv4));
}

#[test]
fn fewer_than_four_numbers_fail_as_ipv4() {
    assert_eq!(PlcHost::parse("1.2.3"), Err(HostReason::Ipv4));
    assert_eq!(PlcHost::parse("123"), Err(HostReason::Ipv4));
}

#[test]
fn more_than_four_numbers_fail_as_ipv4() {
    assert_eq!(PlcHost::parse("1.2.3.4.5"), Err(HostReason::Ipv4));
}

#[test]
fn underscore_and_port_fail_as_label() {
    assert_eq!(PlcHost::parse("plc_01:102"), Err(HostReason::Label));
}

#[test]
fn ipv6_text_fails_as_label() {
    assert_eq!(PlcHost::parse("fe80::1"), Err(HostReason::Label));
}

#[test]
fn trailing_dot_fails() {
    assert_eq!(PlcHost::parse("plc.local."), Err(HostReason::TrailingDot));
}

#[test]
fn empty_label_fails() {
    assert_eq!(PlcHost::parse("plc..local"), Err(HostReason::Label));
    assert_eq!(PlcHost::parse(".plc"), Err(HostReason::Label));
}

#[test]
fn label_of_63_characters_passes() {
    assert!(PlcHost::parse(&"a".repeat(63)).is_ok());
}

#[test]
fn label_of_64_characters_fails() {
    assert_eq!(PlcHost::parse(&"a".repeat(64)), Err(HostReason::Label));
}

#[test]
fn hyphen_at_the_start_of_a_label_fails() {
    assert_eq!(PlcHost::parse("-plc.local"), Err(HostReason::Label));
    assert_eq!(PlcHost::parse("plc.-local"), Err(HostReason::Label));
}

#[test]
fn hyphen_at_the_end_of_a_label_fails() {
    assert_eq!(PlcHost::parse("plc-.local"), Err(HostReason::Label));
    assert_eq!(PlcHost::parse("plc.local-"), Err(HostReason::Label));
}

#[test]
fn non_ascii_letter_fails_as_label() {
    assert_eq!(PlcHost::parse("plć.local"), Err(HostReason::Label));
}

#[test]
fn name_of_253_characters_passes() {
    let name = name_of(61);
    assert_eq!(name.len(), 253);
    assert!(PlcHost::parse(&name).is_ok());
}

#[test]
fn name_of_254_characters_fails() {
    let name = name_of(62);
    assert_eq!(name.len(), 254);
    assert_eq!(PlcHost::parse(&name), Err(HostReason::TooLong));
}

#[test]
fn reason_text_names_the_rule() {
    assert!(HostReason::TooLong.to_string().contains("253"));
}
