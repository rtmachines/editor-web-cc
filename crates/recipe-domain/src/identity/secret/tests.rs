//! Tests of task 2.2: the secret value objects of design D4.

use super::*;
use crate::audit::SessionRef;
use crate::identity::PasswordReason;
use sha2::{Digest, Sha256};

const PHC: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21l$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGFzaGhhc2g";

/// Return the lower-case hex text of the SHA-256 of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn a_password_of_one_character_passes() {
    assert!(Password::parse("x").is_ok());
}

#[test]
fn an_empty_password_fails_with_empty() {
    assert_eq!(Password::parse("").unwrap_err(), PasswordReason::Empty);
}

#[test]
fn a_password_counts_characters_and_not_bytes() {
    let longest = "\u{142}".repeat(128);
    assert!(Password::parse(&longest).is_ok());
    let too_long = "\u{142}".repeat(129);
    assert_eq!(
        Password::parse(&too_long).unwrap_err(),
        PasswordReason::TooLong
    );
}

#[test]
fn a_password_keeps_its_text_for_the_hasher() {
    assert_eq!(Password::parse(" a b ").unwrap().as_str(), " a b ");
}

#[test]
fn a_new_password_needs_twelve_characters() {
    assert_eq!(
        NewPassword::parse(&"a".repeat(11)).unwrap_err(),
        PasswordReason::TooShort
    );
    // The text "aaaaaaaaaaaa" is in the breached-password list, so the test uses another text.
    assert!(NewPassword::parse("q7Lm2x9Zp4Rt").is_ok());
    assert_eq!(
        NewPassword::parse("").unwrap_err(),
        PasswordReason::TooShort
    );
}

#[test]
fn a_new_password_has_at_most_128_characters() {
    assert!(NewPassword::parse(&"a".repeat(128)).is_ok());
    assert_eq!(
        NewPassword::parse(&"a".repeat(129)).unwrap_err(),
        PasswordReason::TooLong
    );
}

#[test]
fn a_new_password_counts_characters_and_not_bytes() {
    // 12 characters of 2 bytes each are 24 bytes.
    assert!(NewPassword::parse(&"\u{142}".repeat(12)).is_ok());
    assert_eq!(
        NewPassword::parse(&"\u{142}".repeat(11)).unwrap_err(),
        PasswordReason::TooShort
    );
}

#[test]
fn a_new_password_converts_to_a_password_with_the_same_text() {
    let new = NewPassword::parse("correct horse battery").unwrap();
    let password = Password::from(&new);
    assert_eq!(password.as_str(), "correct horse battery");
}

#[test]
fn a_phc_string_of_argon2id_passes() {
    let hash = PasswordHash::parse(PHC).unwrap();
    assert_eq!(hash.as_str(), PHC);
}

#[test]
fn a_phc_string_of_another_algorithm_fails() {
    for text in [
        "$argon2i$v=19$m=16,t=2,p=1$c29tZXNhbHQ$aGFzaA",
        "$argon2d$v=19$m=16,t=2,p=1$c29tZXNhbHQ$aGFzaA",
        "$2b$12$abcdefghijklmnopqrstuuABCDEFGHIJKLMNOPQRSTUVWXYZ01234",
        "$argon2id",
        "argon2id$v=19",
        "",
    ] {
        assert_eq!(
            PasswordHash::parse(text).unwrap_err(),
            PasswordHashReason::Format,
            "{text}"
        );
    }
}

#[test]
fn a_phc_string_with_white_space_or_control_characters_fails() {
    for text in [
        "$argon2id$v=19 $m=16",
        "$argon2id$v=19\n",
        "$argon2id$\t",
        "$argon2id$\u{142}",
    ] {
        assert!(PasswordHash::parse(text).is_err(), "{text:?}");
    }
}

#[test]
fn a_phc_string_longer_than_the_limit_fails() {
    let text = format!("$argon2id${}", "a".repeat(512));
    assert!(PasswordHash::parse(&text).is_err());
}

#[test]
fn a_session_token_round_trips_through_hex() {
    let token = SessionToken::from_bytes([0xab; 16]);
    assert_eq!(token.to_hex(), "ab".repeat(16));
    let parsed = SessionToken::parse(&token.to_hex()).unwrap();
    assert_eq!(parsed.as_bytes(), &[0xab; 16]);
}

#[test]
fn a_session_token_needs_32_lower_case_hex_characters() {
    for text in [
        "ab".repeat(15),
        "ab".repeat(17),
        "AB".repeat(16),
        "zz".repeat(16),
        String::new(),
        "\u{142}".repeat(32),
    ] {
        assert_eq!(
            SessionToken::parse(&text).unwrap_err(),
            TokenReason::Format,
            "{text}"
        );
    }
}

#[test]
fn a_session_id_is_the_sha256_of_the_token_bytes_in_64_hex_characters() {
    let bytes: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let id = SessionId::of(&SessionToken::from_bytes(bytes));
    assert_eq!(id.as_str(), sha256_hex(&bytes));
    assert_eq!(id.as_str().len(), 64);
}

#[test]
fn the_session_id_of_a_zero_token_equals_the_known_sha256() {
    let id = SessionId::of(&SessionToken::from_bytes([0; 16]));
    assert_eq!(
        id.as_str(),
        "374708fff7719dd5979ec875d56cd2286f6d3cf7ec317a3b25632aab28ec37bb"
    );
}

#[test]
fn a_session_id_differs_from_the_token_hex() {
    let token = SessionToken::from_bytes([7; 16]);
    assert_ne!(SessionId::of(&token).as_str(), token.to_hex());
}

#[test]
fn a_session_id_parses_64_lower_case_hex_characters() {
    let text = "0123456789abcdef".repeat(4);
    assert_eq!(SessionId::parse(&text).unwrap().as_str(), text);
}

#[test]
fn a_session_id_of_another_shape_fails() {
    for text in [
        "0123456789abcdef".repeat(3),
        "0123456789abcdef".repeat(5),
        "0123456789ABCDEF".repeat(4),
        "0123456789abcdeg".repeat(4),
        String::new(),
    ] {
        assert_eq!(SessionId::parse(&text).unwrap_err(), TokenReason::Format);
    }
}

#[test]
fn a_csrf_token_has_32_bytes_in_64_hex_characters() {
    let token = CsrfToken::from_bytes([0x5a; 32]);
    assert_eq!(token.to_hex(), "5a".repeat(32));
    assert_eq!(token.as_bytes(), &[0x5a; 32]);
    let parsed = CsrfToken::parse(&token.to_hex()).unwrap();
    assert_eq!(parsed.as_bytes(), token.as_bytes());
}

#[test]
fn a_csrf_token_of_another_shape_fails() {
    for text in [
        "5a".repeat(31),
        "5a".repeat(33),
        "5A".repeat(32),
        "5g".repeat(32),
        String::new(),
    ] {
        assert_eq!(CsrfToken::parse(&text).unwrap_err(), TokenReason::Format);
    }
}

#[test]
fn a_session_ref_is_the_first_16_characters_of_the_session_id() {
    let id = SessionId::of(&SessionToken::from_bytes([0; 16]));
    let reference = SessionRef::of(&id);
    assert_eq!(reference.as_str(), "374708fff7719dd5");
    assert_eq!(SessionRef::parse(reference.as_str()).unwrap(), reference);
}

#[test]
fn debug_of_every_secret_prints_redacted() {
    let password = Password::parse("hunter2-hunter2").unwrap();
    let new_password = NewPassword::parse("hunter2-hunter2").unwrap();
    let hash = PasswordHash::parse(PHC).unwrap();
    let token = SessionToken::from_bytes([0xcd; 16]);
    let id = SessionId::of(&token);
    let csrf = CsrfToken::from_bytes([0xef; 32]);
    let texts = [
        format!("{password:?}"),
        format!("{new_password:?}"),
        format!("{hash:?}"),
        format!("{token:?}"),
        format!("{id:?}"),
        format!("{csrf:?}"),
    ];
    for text in texts {
        assert_eq!(text, "[redacted]");
    }
}

#[test]
fn debug_inside_a_struct_and_pretty_debug_hide_the_secret() {
    #[derive(Debug)]
    struct Holder {
        token: SessionToken,
        csrf: CsrfToken,
    }
    let holder = Holder {
        token: SessionToken::from_bytes([0xcd; 16]),
        csrf: CsrfToken::from_bytes([0xef; 32]),
    };
    for text in [format!("{holder:?}"), format!("{holder:#?}")] {
        assert!(!text.contains("cdcd"), "{text}");
        assert!(!text.contains("efef"), "{text}");
        assert!(text.contains("[redacted]"), "{text}");
    }
    assert_eq!(holder.token.to_hex(), "cd".repeat(16));
    assert_eq!(holder.csrf.to_hex(), "ef".repeat(32));
}

#[test]
fn a_failed_parse_names_no_input_text() {
    let secret = "S3cr3t-input-text";
    let texts = [
        SessionToken::parse(secret).unwrap_err().to_string(),
        SessionId::parse(secret).unwrap_err().to_string(),
        CsrfToken::parse(secret).unwrap_err().to_string(),
        PasswordHash::parse(secret).unwrap_err().to_string(),
        format!("{:?}", PasswordHash::parse(secret).unwrap_err()),
        format!("{:?}", SessionToken::parse(secret).unwrap_err()),
    ];
    for text in texts {
        assert!(!text.contains(secret), "{text}");
        assert!(!text.is_empty());
    }
}
