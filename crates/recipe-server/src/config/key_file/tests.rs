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
