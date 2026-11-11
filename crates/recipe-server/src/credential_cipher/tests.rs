//! Tests of the credential cipher (ADR-0154 (g), design D5).

use recipe_app::ports::{CipherFault, CredentialCipher, SealedSecret};
use recipe_domain::plant::{MachineId, PlcPassword};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{AesGcmCipher, CipherKey, KEY_LENGTH, NONCE_LENGTH, TAG_LENGTH, VERSION};

const PASSWORD_TEXT: &str = "s3cret-Pa55 wörd";

fn machine(seed: u128) -> MachineId {
    MachineId::from(Uuid::from_u128(seed))
}

fn password() -> PlcPassword {
    PlcPassword::parse(PASSWORD_TEXT).unwrap()
}

fn key(fill: u8) -> CipherKey {
    CipherKey::from_bytes(Zeroizing::new([fill; KEY_LENGTH]))
}

fn cipher() -> AesGcmCipher {
    AesGcmCipher::new(&key(7))
}

#[test]
fn round_trip_returns_the_password() {
    let cipher = cipher();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    let opened = cipher.open(machine(1), &sealed).unwrap();
    assert_eq!(opened.expose(), PASSWORD_TEXT);
}

#[test]
fn sealed_form_has_version_nonce_ciphertext_and_tag() {
    let sealed = cipher().seal(machine(1), &password()).unwrap();
    let bytes = sealed.as_bytes();
    assert_eq!(bytes.first(), Some(&VERSION));
    assert_eq!(VERSION, 0x01);
    assert_eq!(
        bytes.len(),
        1 + NONCE_LENGTH + PASSWORD_TEXT.len() + TAG_LENGTH
    );
}

#[test]
fn sealed_form_does_not_hold_the_plain_text() {
    let sealed = cipher().seal(machine(1), &password()).unwrap();
    let needle = PASSWORD_TEXT.as_bytes();
    assert!(
        !sealed
            .as_bytes()
            .windows(needle.len())
            .any(|window| window == needle)
    );
}

#[test]
fn another_machine_fails_with_authentication() {
    let cipher = cipher();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    let result = cipher.open(machine(2), &sealed);
    assert_eq!(result.unwrap_err(), CipherFault::Authentication);
}

#[test]
fn another_key_fails_with_authentication() {
    let sealed = cipher().seal(machine(1), &password()).unwrap();
    let other = AesGcmCipher::new(&key(8));
    let result = other.open(machine(1), &sealed);
    assert_eq!(result.unwrap_err(), CipherFault::Authentication);
}

#[test]
fn two_seals_of_one_password_differ() {
    let cipher = cipher();
    let first = cipher.seal(machine(1), &password()).unwrap();
    let second = cipher.seal(machine(1), &password()).unwrap();
    assert_ne!(first, second);
}

#[test]
fn a_wrong_version_byte_fails_with_format() {
    let cipher = cipher();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    let mut bytes = sealed.as_bytes().to_vec();
    *bytes.first_mut().unwrap() = 0x02;
    let result = cipher.open(machine(1), &SealedSecret::from_bytes(bytes));
    assert_eq!(result.unwrap_err(), CipherFault::Format);
}

#[test]
fn a_truncated_input_fails_with_format() {
    let cipher = cipher();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    let shortest = 1 + NONCE_LENGTH + TAG_LENGTH;
    for length in [0, 1, NONCE_LENGTH, shortest - 1] {
        let bytes = sealed.as_bytes().get(..length).unwrap().to_vec();
        let result = cipher.open(machine(1), &SealedSecret::from_bytes(bytes));
        assert_eq!(result.unwrap_err(), CipherFault::Format, "length {length}");
    }
}

#[test]
fn a_changed_ciphertext_fails_with_authentication() {
    let cipher = cipher();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    let mut bytes = sealed.as_bytes().to_vec();
    let last = bytes.last_mut().unwrap();
    *last ^= 0x01;
    let result = cipher.open(machine(1), &SealedSecret::from_bytes(bytes));
    assert_eq!(result.unwrap_err(), CipherFault::Authentication);
}

#[test]
fn a_changed_nonce_fails_with_authentication() {
    let cipher = cipher();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    let mut bytes = sealed.as_bytes().to_vec();
    *bytes.get_mut(1).unwrap() ^= 0x01;
    let result = cipher.open(machine(1), &SealedSecret::from_bytes(bytes));
    assert_eq!(result.unwrap_err(), CipherFault::Authentication);
}

#[test]
fn debug_text_of_the_key_and_the_cipher_hides_the_key() {
    let key = key(0xAB);
    let cipher = AesGcmCipher::new(&key);
    assert_eq!(format!("{key:?}"), "CipherKey(<redacted>)");
    assert_eq!(format!("{cipher:?}"), "AesGcmCipher(<redacted>)");
    assert!(!format!("{key:?}{cipher:?}").to_lowercase().contains("ab"));
}

#[test]
fn the_cipher_is_usable_as_the_port_object() {
    let cipher: Box<dyn CredentialCipher> = Box::new(cipher());
    let sealed = cipher.seal(machine(3), &password()).unwrap();
    assert!(cipher.open(machine(3), &sealed).is_ok());
}
