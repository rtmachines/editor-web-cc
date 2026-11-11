use super::*;

#[test]
fn plc_address_of_one_and_two_hundred_characters_passes() {
    let longest = "a".repeat(200);
    assert_eq!(
        PlcAddress::parse("a").as_ref().map(PlcAddress::as_str),
        Ok("a")
    );
    assert_eq!(
        PlcAddress::parse(&longest).as_ref().map(PlcAddress::as_str),
        Ok(longest.as_str())
    );
}

#[test]
fn plc_address_counts_characters_not_bytes() {
    let longest = "\u{105}".repeat(200);
    assert!(PlcAddress::parse(&longest).is_ok());
}

#[test]
fn plc_address_of_two_hundred_and_one_characters_fails() {
    assert_eq!(
        PlcAddress::parse(&"a".repeat(201)),
        Err(ConnectionTextReason::TooLong { max: 200 })
    );
}

#[test]
fn plc_address_that_is_empty_fails() {
    assert_eq!(PlcAddress::parse(""), Err(ConnectionTextReason::Empty));
}

#[test]
fn plc_address_with_a_control_character_fails() {
    for text in ["a\0b", "a\tb", "a\nb", "a\r\nb", "a\u{7f}b", "\u{1b}"] {
        assert_eq!(
            PlcAddress::parse(text),
            Err(ConnectionTextReason::ControlCharacter),
            "{text:?}"
        );
    }
}

#[test]
fn plc_address_keeps_its_text_and_prints_it() {
    let address = PlcAddress::parse(" 192.168.1.10.1.1 ");
    assert_eq!(
        address.as_ref().map(ToString::to_string),
        Ok(" 192.168.1.10.1.1 ".to_owned())
    );
}
