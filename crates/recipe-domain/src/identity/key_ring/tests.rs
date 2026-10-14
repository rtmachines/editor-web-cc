use sha2::{Digest, Sha256};

use super::*;

const KEY_ONE: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const KEY_TWO: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const BLOCK_BYTES: usize = 64;

fn ring(text: &str) -> KeyRing {
    KeyRing::parse(text).unwrap()
}

/// HMAC-SHA256 from the definition, with the 32-byte key padded to one block.
fn reference_hmac(key_byte: u8, message: &[u8]) -> [u8; 32] {
    let mut block = [0u8; BLOCK_BYTES];
    block[..32].fill(key_byte);
    let inner_key: Vec<u8> = block.iter().map(|byte| byte ^ 0x36).collect();
    let outer_key: Vec<u8> = block.iter().map(|byte| byte ^ 0x5c).collect();
    let mut inner = Sha256::new();
    inner.update(&inner_key);
    inner.update(message);
    let mut outer = Sha256::new();
    outer.update(&outer_key);
    outer.update(inner.finalize());
    outer.finalize().into()
}

#[test]
fn one_key_parses() {
    assert!(KeyRing::parse(KEY_ONE).is_ok());
}

#[test]
fn a_trailing_line_end_is_allowed() {
    assert!(KeyRing::parse(&format!("{KEY_ONE}\n")).is_ok());
    assert!(KeyRing::parse(&format!("{KEY_ONE}\r\n{KEY_TWO}\r\n")).is_ok());
}

#[test]
fn an_empty_file_fails() {
    assert_eq!(KeyRing::parse("").unwrap_err(), KeyRingReason::Empty);
}

#[test]
fn a_file_with_only_a_line_end_fails_with_an_empty_line() {
    assert_eq!(
        KeyRing::parse("\n").unwrap_err(),
        KeyRingReason::EmptyLine { line: 1 }
    );
}

#[test]
fn an_empty_line_in_the_middle_fails() {
    assert_eq!(
        KeyRing::parse(&format!("{KEY_ONE}\n\n{KEY_TWO}")).unwrap_err(),
        KeyRingReason::EmptyLine { line: 2 }
    );
}

#[test]
fn an_empty_line_at_the_end_fails() {
    assert_eq!(
        KeyRing::parse(&format!("{KEY_ONE}\n\n")).unwrap_err(),
        KeyRingReason::EmptyLine { line: 2 }
    );
}

#[test]
fn an_upper_case_line_fails() {
    let upper = "A".repeat(64);
    assert_eq!(
        KeyRing::parse(&upper).unwrap_err(),
        KeyRingReason::NotLowerHex { line: 1 }
    );
}

#[test]
fn a_line_with_a_character_outside_hex_fails() {
    let text = format!("{}g", "a".repeat(63));
    assert_eq!(
        KeyRing::parse(&text).unwrap_err(),
        KeyRingReason::NotLowerHex { line: 1 }
    );
}

#[test]
fn a_line_of_63_characters_fails() {
    assert_eq!(
        KeyRing::parse(&"a".repeat(63)).unwrap_err(),
        KeyRingReason::Length { line: 1 }
    );
}

#[test]
fn a_line_of_65_characters_fails() {
    assert_eq!(
        KeyRing::parse(&format!("{KEY_ONE}\n{}", "a".repeat(65))).unwrap_err(),
        KeyRingReason::Length { line: 2 }
    );
}

#[test]
fn a_line_with_a_space_fails() {
    assert_eq!(
        KeyRing::parse(&format!(" {KEY_ONE}")).unwrap_err(),
        KeyRingReason::Length { line: 1 }
    );
}

#[test]
fn the_reason_names_no_input_text() {
    let text = format!("{}g", "a".repeat(63));
    let shown = KeyRing::parse(&text).unwrap_err().to_string();
    assert!(!shown.contains("aaaa"));
}

#[test]
fn the_ring_signs_with_hmac_sha256() {
    let tag = ring(KEY_ONE).sign(b"jkowalski");
    assert_eq!(tag, reference_hmac(0x11, b"jkowalski"));
}

#[test]
fn the_ring_signs_with_the_first_key_only() {
    let both = ring(&format!("{KEY_TWO}\n{KEY_ONE}"));
    assert_eq!(both.sign(b"message"), reference_hmac(0x22, b"message"));
    assert_ne!(both.sign(b"message"), ring(KEY_ONE).sign(b"message"));
}

#[test]
fn the_ring_accepts_a_signature_of_each_key() {
    let both = ring(&format!("{KEY_TWO}\n{KEY_ONE}"));
    assert!(both.verify(b"message", &ring(KEY_TWO).sign(b"message")));
    assert!(both.verify(b"message", &ring(KEY_ONE).sign(b"message")));
}

#[test]
fn the_ring_rejects_a_signature_of_an_unknown_key() {
    assert!(!ring(KEY_ONE).verify(b"message", &ring(KEY_TWO).sign(b"message")));
}

#[test]
fn a_changed_message_fails() {
    let ring = ring(KEY_ONE);
    let tag = ring.sign(b"nonce-1");
    assert!(ring.verify(b"nonce-1", &tag));
    assert!(!ring.verify(b"nonce-2", &tag));
}

#[test]
fn a_changed_tag_fails() {
    let ring = ring(KEY_ONE);
    let mut tag = ring.sign(b"nonce");
    tag[31] ^= 1;
    assert!(!ring.verify(b"nonce", &tag));
}

#[test]
fn a_tag_of_the_wrong_length_fails() {
    let ring = ring(KEY_ONE);
    let tag = ring.sign(b"nonce");
    assert!(!ring.verify(b"nonce", &tag[..31]));
    assert!(!ring.verify(b"nonce", &[]));
    let mut long = tag.to_vec();
    long.push(0);
    assert!(!ring.verify(b"nonce", &long));
}

#[test]
fn debug_of_the_ring_is_redacted() {
    let shown = format!("{:?}", ring(&format!("{KEY_ONE}\n{KEY_TWO}")));
    assert_eq!(shown, "KeyRing([redacted])");
}
