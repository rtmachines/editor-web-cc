//! Tests of the key file of the PLC credential key (ADR-0154 (g), design D5).

use recipe_app::ports::{CipherFault, CredentialCipher};
use recipe_domain::plant::{MachineId, PlcPassword};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{PlcKeyFault, parse_plc_key_file, read_plc_key_file};
use crate::config::test_support::Sandbox;
use crate::credential_cipher::{AesGcmCipher, CipherKey, KEY_LENGTH};

const KEY_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const KEY_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
const KEY: &str = "security.plc_credential_key_file";

fn machine() -> MachineId {
    MachineId::from(Uuid::from_u128(1))
}

fn password() -> PlcPassword {
    PlcPassword::parse("s3cret-Pa55").unwrap()
}

/// The 32 bytes that `KEY_A` stands for.
fn bytes_of_key_a() -> [u8; KEY_LENGTH] {
    let pattern = [0x01_u8, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
    let mut bytes = [0_u8; KEY_LENGTH];
    for (slot, value) in bytes.iter_mut().zip(pattern.iter().cycle()) {
        *slot = *value;
    }
    bytes
}

#[test]
fn plc_key_file_one_key_with_or_without_a_final_line_feed_is_a_key() {
    for text in [KEY_A.to_owned(), format!("{KEY_A}\n")] {
        let key = parse_plc_key_file(&text).unwrap();

        let cipher = AesGcmCipher::new(&key.into_cipher_key());
        let reference = AesGcmCipher::new(&CipherKey::from_bytes(Zeroizing::new(bytes_of_key_a())));
        let sealed = reference.seal(machine(), &password()).unwrap();
        let opened = cipher.open(machine(), &sealed).unwrap();
        assert_eq!(opened.expose(), "s3cret-Pa55", "{text:?}");
    }
}

#[test]
fn plc_key_file_another_key_does_not_open_the_sealed_password() {
    let first = AesGcmCipher::new(&parse_plc_key_file(KEY_A).unwrap().into_cipher_key());
    let second = AesGcmCipher::new(&parse_plc_key_file(KEY_B).unwrap().into_cipher_key());

    let sealed = first.seal(machine(), &password()).unwrap();

    assert_eq!(
        second.open(machine(), &sealed).unwrap_err(),
        CipherFault::Authentication
    );
}

#[test]
fn plc_key_file_carriage_return_anywhere_is_a_fault() {
    for text in [
        format!("{KEY_A}\r\n"),
        format!("{KEY_A}\r"),
        format!("{}\r{}\n", &KEY_A[..10], &KEY_A[10..]),
    ] {
        let fault = parse_plc_key_file(&text).unwrap_err();

        assert_eq!(fault, PlcKeyFault::CarriageReturn, "{text:?}");
    }
}

#[test]
fn plc_key_file_an_empty_file_is_a_fault() {
    for text in ["", "\n"] {
        let fault = parse_plc_key_file(text).unwrap_err();

        assert_eq!(fault, PlcKeyFault::Empty, "{text:?}");
    }
}

#[test]
fn plc_key_file_a_second_line_is_a_fault() {
    for text in [
        format!("{KEY_A}\n{KEY_B}"),
        format!("{KEY_A}\n{KEY_B}\n"),
        format!("{KEY_A}\n\n"),
        format!("{KEY_A}\nx"),
        format!("\n{KEY_A}"),
    ] {
        let fault = parse_plc_key_file(&text).unwrap_err();

        assert_eq!(fault, PlcKeyFault::SecondLine, "{text:?}");
    }
}

#[test]
fn plc_key_file_62_or_66_characters_are_a_length_fault() {
    for text in [
        KEY_A[..62].to_owned(),
        format!("{}\n", &KEY_A[..62]),
        format!("{KEY_A}ab"),
        format!("{KEY_A}ab\n"),
        KEY_A[..63].to_owned(),
        format!("{KEY_A}a"),
    ] {
        let fault = parse_plc_key_file(&text).unwrap_err();

        assert_eq!(fault, PlcKeyFault::Length, "{text:?}");
    }
}

#[test]
fn plc_key_file_a_character_outside_lower_case_hex_is_a_fault() {
    let upper_case = KEY_A.to_uppercase();
    let blank_start = format!(" {}", &KEY_A[..63]);
    let blank_end = format!("{} ", &KEY_A[..63]);
    let non_hex = format!("g{}", &KEY_A[..63]);
    let plus_sign = format!("+f{}", &KEY_A[..62]);
    let multi_byte = "é".repeat(64);
    for text in [
        upper_case.as_str(),
        blank_start.as_str(),
        blank_end.as_str(),
        non_hex.as_str(),
        plus_sign.as_str(),
        multi_byte.as_str(),
    ] {
        let fault = parse_plc_key_file(text).unwrap_err();

        assert_eq!(fault, PlcKeyFault::NotLowerHex, "{text:?}");
    }
}

#[test]
fn plc_key_file_length_counts_characters_and_not_bytes() {
    // 63 two-byte characters have 126 bytes. Only the count of characters decides the length.
    for text in ["é".repeat(63), "é".repeat(65)] {
        let fault = parse_plc_key_file(&text).unwrap_err();

        assert_eq!(fault, PlcKeyFault::Length, "{text:?}");
    }
}

#[test]
fn plc_credential_key_debug_is_redacted_and_the_fault_text_holds_no_content() {
    let key = parse_plc_key_file(KEY_A).unwrap();

    assert_eq!(format!("{key:?}"), "PlcCredentialKey(<redacted>)");
    assert_eq!(format!("{key:#?}"), "PlcCredentialKey(<redacted>)");
    for text in [
        format!("{}\r\n", KEY_A.replace('0', "z")),
        KEY_A.replace('0', "Z"),
        format!("{KEY_A}\n{KEY_B}x"),
    ] {
        let fault = parse_plc_key_file(&text).unwrap_err();

        let shown = format!("{fault} {fault:?}");
        assert!(!shown.contains("123456789"), "{shown}");
        assert!(!shown.contains("zzzz"), "{shown}");
        assert!(!shown.contains("ZZZZ"), "{shown}");
        assert!(!shown.contains("fedcba"), "{shown}");
    }
}

#[test]
fn plc_key_file_read_returns_the_key_of_a_good_file() {
    let dir = Sandbox::new();
    let file = dir.write("plc_key", &format!("{KEY_A}\n"));

    let key = read_plc_key_file(KEY, &file).unwrap();

    let reference = AesGcmCipher::new(&CipherKey::from_bytes(Zeroizing::new(bytes_of_key_a())));
    let sealed = reference.seal(machine(), &password()).unwrap();
    let opened = AesGcmCipher::new(&key.into_cipher_key())
        .open(machine(), &sealed)
        .unwrap();
    assert_eq!(opened.expose(), "s3cret-Pa55");
}

#[test]
fn plc_key_file_read_faults_name_the_key_and_not_the_content_or_the_path() {
    let dir = Sandbox::new();
    let folder = dir.path("folder");
    std::fs::create_dir_all(&folder).unwrap();
    let cases = [
        (dir.path("absent_file"), None, "cannot be read"),
        (folder, None, "cannot be read"),
        (
            dir.write("bad_length", &KEY_A[..62]),
            Some(KEY_A),
            "64 characters",
        ),
        (
            dir.write("bad_length_long", &format!("{KEY_A}ab\n")),
            Some(KEY_A),
            "64 characters",
        ),
        (
            dir.write("bad_case", &format!("{}\n", KEY_A.to_uppercase())),
            Some(KEY_A),
            "outside 0-9 and a-f",
        ),
        (
            dir.write("second_line", &format!("{KEY_A}\n{KEY_B}\n")),
            Some(KEY_B),
            "more than one line",
        ),
        (
            dir.write("windows_line_end", &format!("{KEY_A}\r\n")),
            Some(KEY_A),
            "carriage return",
        ),
        (dir.write("empty", ""), None, "no key"),
    ];
    for (path, content, words) in cases {
        let message = format!("{:#}", read_plc_key_file(KEY, &path).unwrap_err());

        assert!(message.contains(KEY), "{message}");
        assert!(message.contains(words), "{message}");
        assert!(!message.contains("recipe-server-config"), "{message}");
        assert!(!message.contains("0123456789"), "{message}");
        assert!(!message.contains("fedcba"), "{message}");
        if let Some(content) = content {
            assert!(!message.contains(content), "{message}");
        }
    }
}
