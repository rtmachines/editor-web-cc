//! Tests of the key file check (ADR-0031, ADR-0035 (d), design D4 step 2).

use recipe_domain::identity::KeyRingReason;

use super::{KeyFileFault, parse_key_file, read_key_file};
use crate::config::test_support::Sandbox;

const KEY_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const KEY_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
const KEY: &str = "security.csrf_signing_key_file";

#[test]
fn key_file_one_key_with_or_without_a_line_feed_is_a_ring() {
    for text in [KEY_A.to_owned(), format!("{KEY_A}\n")] {
        let ring = parse_key_file(&text).unwrap();

        let tag = ring.sign(b"message");
        assert!(ring.verify(b"message", &tag));
    }
}

#[test]
fn key_file_older_keys_below_the_first_line_still_verify() {
    let both = parse_key_file(&format!("{KEY_A}\n{KEY_B}\n")).unwrap();
    let old_only = parse_key_file(KEY_B).unwrap();

    let old_tag = old_only.sign(b"message");

    assert!(both.verify(b"message", &old_tag));
    assert_eq!(
        both.sign(b"message"),
        parse_key_file(KEY_A).unwrap().sign(b"message")
    );
}

#[test]
fn key_file_carriage_return_anywhere_is_a_fault() {
    for text in [
        format!("{KEY_A}\r\n"),
        format!("{KEY_A}\r"),
        format!("{KEY_A}\n{KEY_B}\r\n"),
        format!("{}\r{}\n", &KEY_A[..10], &KEY_A[10..]),
    ] {
        let fault = parse_key_file(&text).unwrap_err();

        assert_eq!(fault, KeyFileFault::CarriageReturn, "{text:?}");
    }
}

#[test]
fn key_file_ring_faults_keep_their_reason() {
    let cases = [
        (String::new(), KeyRingReason::Empty),
        (
            format!("{KEY_A}\n\n{KEY_B}"),
            KeyRingReason::EmptyLine { line: 2 },
        ),
        (KEY_A[..63].to_owned(), KeyRingReason::Length { line: 1 }),
        (
            format!("{KEY_A}\n{}", KEY_B.to_uppercase()),
            KeyRingReason::NotLowerHex { line: 2 },
        ),
    ];
    for (text, reason) in cases {
        let fault = parse_key_file(&text).unwrap_err();

        assert_eq!(fault, KeyFileFault::Ring(reason), "{text:?}");
    }
}

#[test]
fn key_file_blank_line_after_the_last_key_is_an_empty_line_fault() {
    for (text, line) in [
        (format!("{KEY_A}\n\n"), 2),
        (format!("{KEY_A}\n{KEY_B}\n\n"), 3),
    ] {
        let fault = parse_key_file(&text).unwrap_err();

        assert_eq!(
            fault,
            KeyFileFault::Ring(KeyRingReason::EmptyLine { line }),
            "{text:?}"
        );
    }
}

#[test]
fn key_file_length_counts_characters_and_not_bytes() {
    // Each multi-byte character has 2 bytes. Only the count of characters decides the length.
    for letter in ['ł', 'é'] {
        let at_limit = letter.to_string().repeat(64);
        let one_under = letter.to_string().repeat(63);
        let one_over = letter.to_string().repeat(65);
        let hex_and_letter = format!("{}{letter}", &KEY_A[..62]);

        assert_eq!(
            parse_key_file(&at_limit).unwrap_err(),
            KeyFileFault::Ring(KeyRingReason::NotLowerHex { line: 1 }),
            "{at_limit:?}"
        );
        assert_eq!(
            parse_key_file(&one_under).unwrap_err(),
            KeyFileFault::Ring(KeyRingReason::Length { line: 1 }),
            "{one_under:?}"
        );
        assert_eq!(
            parse_key_file(&one_over).unwrap_err(),
            KeyFileFault::Ring(KeyRingReason::Length { line: 1 }),
            "{one_over:?}"
        );
        // 62 digits and one letter have 63 characters and 64 bytes.
        assert_eq!(hex_and_letter.len(), 64);
        assert_eq!(
            parse_key_file(&hex_and_letter).unwrap_err(),
            KeyFileFault::Ring(KeyRingReason::Length { line: 1 }),
            "{hex_and_letter:?}"
        );
    }
}

#[test]
fn key_file_fault_text_never_holds_the_content() {
    let secret_text = format!("{}\r\n", KEY_A.replace('0', "z"));
    let texts = [
        secret_text,
        KEY_A.replace('0', "Z"),
        format!("{KEY_A}\n{}x", KEY_B),
    ];
    for text in texts {
        let fault = parse_key_file(&text).unwrap_err();

        let shown = format!("{fault} {fault:?}");
        assert!(!shown.contains("123456789"), "{shown}");
        assert!(!shown.contains("zzzz"), "{shown}");
        assert!(!shown.contains("ZZZZ"), "{shown}");
        assert!(!shown.contains("fedcba"), "{shown}");
    }
}

#[test]
fn key_file_read_names_the_key_and_never_the_path_or_the_content() {
    let dir = Sandbox::new();
    let good = dir.write("good", &format!("{KEY_A}\n"));
    let carriage = dir.write("carriage", &format!("{KEY_A}\r\n"));
    let short = dir.write("short", "abcdef\n");
    let missing = dir.path("absent_file");
    let folder = dir.path("folder");
    std::fs::create_dir_all(&folder).unwrap();

    assert!(read_key_file(KEY, &good).is_ok());
    for (path, words) in [
        (carriage, "carriage return"),
        (short, "64 characters"),
        (missing, "cannot be read"),
        (folder, "cannot be read"),
    ] {
        let error = read_key_file(KEY, &path).unwrap_err();

        let text = format!("{error:#}");
        assert!(text.contains(KEY), "{text}");
        assert!(text.contains(words), "{text}");
        assert!(!text.contains("recipe-server-config"), "{text}");
        assert!(!text.contains("abcdef"), "{text}");
        assert!(!text.contains("123456789"), "{text}");
    }
}
