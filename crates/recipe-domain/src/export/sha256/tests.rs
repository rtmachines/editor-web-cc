use super::*;
use proptest::prelude::*;

const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn lower_case_hex_parses_and_displays_the_same_text() {
    let parsed = Sha256::parse(EMPTY_HASH).map(|hash| hash.to_string());
    assert_eq!(parsed, Ok(EMPTY_HASH.to_owned()));
}

#[test]
fn sha256_of_the_empty_input_parses_and_round_trips() {
    let hash = Sha256::parse(EMPTY_HASH).unwrap();
    assert_eq!(hash.to_string(), EMPTY_HASH);
    assert_eq!(hash.as_bytes().first(), Some(&0xe3));
}

#[test]
fn sixty_three_characters_give_length_reason() {
    let text = &EMPTY_HASH[..63];
    assert_eq!(Sha256::parse(text), Err(Sha256Reason::Length));
}

#[test]
fn sixty_five_characters_give_length_reason() {
    let text = format!("{EMPTY_HASH}0");
    assert_eq!(Sha256::parse(&text), Err(Sha256Reason::Length));
}

#[test]
fn upper_case_letter_gives_not_lower_hex_reason() {
    let text = format!("E{}", &EMPTY_HASH[1..]);
    assert_eq!(Sha256::parse(&text), Err(Sha256Reason::NotLowerHex));
}

#[test]
fn non_hex_letter_gives_not_lower_hex_reason() {
    let text = format!("g{}", &EMPTY_HASH[1..]);
    assert_eq!(Sha256::parse(&text), Err(Sha256Reason::NotLowerHex));
}

#[test]
fn bytes_zero_to_thirty_one_display_as_hex_pairs() {
    let bytes: [u8; 32] = core::array::from_fn(|index| u8::try_from(index).unwrap());
    let expected = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    assert_eq!(Sha256::from(bytes).to_string(), expected);
}

#[test]
fn from_bytes_keeps_the_bytes() {
    let bytes = [7_u8; 32];
    assert_eq!(Sha256::from(bytes).as_bytes(), &bytes);
}

proptest! {
    #[test]
    fn any_bytes_round_trip_through_text(bytes in any::<[u8; 32]>()) {
        let hash = Sha256::from(bytes);
        let text = hash.to_string();
        prop_assert_eq!(Sha256::parse(&text), Ok(hash));
    }
}
