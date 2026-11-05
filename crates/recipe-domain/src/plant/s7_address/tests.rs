use super::*;
use proptest::prelude::*;

fn reason_of(text: &str) -> Option<S7AddressReason> {
    S7Address::parse(text).err()
}

fn assert_round_trip(text: &str) {
    let parsed = S7Address::parse(text);
    assert!(parsed.is_ok(), "{text} must parse");
    assert_eq!(
        parsed.map(|address| address.to_string()),
        Ok(text.to_owned())
    );
}

#[test]
fn s7_address_examples_round_trip() {
    for text in ["DB1.DBX0.7", "DB65535.DBD65535", "DB1.DBB0", "DB12.DBW4"] {
        assert_round_trip(text);
    }
}

#[test]
fn s7_address_leading_zero_offset_displays_canonical_offset() {
    assert_eq!(
        S7Address::parse("DB1.DBW007").map(|a| a.to_string()),
        Ok("DB1.DBW7".to_owned())
    );
}

#[test]
fn s7_address_accessors_return_the_parts() {
    let bit = S7Address::parse("DB12.DBX4.3");
    assert_eq!(
        bit.map(|a| (a.db(), a.width(), a.offset(), a.bit())),
        Ok((12, S7Width::X, 4, Some(3)))
    );
    let word = S7Address::parse("DB12.DBW4");
    assert_eq!(
        word.map(|a| (a.db(), a.width(), a.offset(), a.bit())),
        Ok((12, S7Width::W, 4, None))
    );
}

#[test]
fn s7_address_width_letters_display_themselves() {
    assert_eq!(S7Width::X.letter(), 'X');
    assert_eq!(S7Width::B.letter(), 'B');
    assert_eq!(S7Width::W.letter(), 'W');
    assert_eq!(S7Width::D.letter(), 'D');
    assert_eq!(S7Width::D.to_string(), "D");
}

#[test]
fn s7_address_db_above_u16_is_db_out_of_range() {
    assert_eq!(
        reason_of("DB65536.DBW0"),
        Some(S7AddressReason::DbOutOfRange)
    );
    assert_eq!(
        reason_of("DB99999.DBW0"),
        Some(S7AddressReason::DbOutOfRange)
    );
}

#[test]
fn s7_address_db_zero_or_leading_zero_is_grammar_error() {
    assert_eq!(reason_of("DB0.DBW0"), Some(S7AddressReason::Grammar));
    assert_eq!(reason_of("DB01.DBW0"), Some(S7AddressReason::Grammar));
}

#[test]
fn s7_address_offset_above_u16_is_offset_out_of_range() {
    assert_eq!(
        reason_of("DB1.DBW65536"),
        Some(S7AddressReason::OffsetOutOfRange)
    );
    assert_eq!(
        reason_of("DB1.DBX65536.0"),
        Some(S7AddressReason::OffsetOutOfRange)
    );
}

#[test]
fn s7_address_malformed_texts_are_grammar_errors() {
    for text in [
        "DB1.DBX0",
        "DB1.DBW0.1",
        "DB1.DBX0.8",
        "DB1.DBX0.9",
        "DB1.DBQ0",
        "db1.dbw0",
        "DB1.DBW",
        "DB1.DBW123456",
        "DB1.DBX0.7 ",
        "",
    ] {
        assert_eq!(reason_of(text), Some(S7AddressReason::Grammar), "{text}");
    }
}

fn width() -> impl Strategy<Value = S7Width> {
    prop_oneof![
        Just(S7Width::X),
        Just(S7Width::B),
        Just(S7Width::W),
        Just(S7Width::D),
    ]
}

proptest! {
    #[test]
    fn s7_address_generated_text_round_trips(
        db in 1_u16..=u16::MAX,
        width in width(),
        offset in 0_u16..=u16::MAX,
        bit in 0_u8..=7,
    ) {
        let (text, expected_bit) = match width {
            S7Width::X => (format!("DB{db}.DBX{offset}.{bit}"), Some(bit)),
            other => (format!("DB{db}.DB{other}{offset}"), None),
        };
        match S7Address::parse(&text) {
            Ok(parsed) => {
                prop_assert_eq!(parsed.to_string(), text);
                prop_assert_eq!(parsed.db(), db);
                prop_assert_eq!(parsed.width(), width);
                prop_assert_eq!(parsed.offset(), offset);
                prop_assert_eq!(parsed.bit(), expected_bit);
            }
            Err(reason) => prop_assert!(false, "{text} failed: {reason}"),
        }
    }
}
