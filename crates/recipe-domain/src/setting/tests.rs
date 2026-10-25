use super::*;
use uuid::Uuid;

/// Fixed vector made outside the crate: `hashlib.sha256` of `WEBHOOK_URL` as UTF-8.
const WEBHOOK_URL: &str = "https://hooks.example.test/services/T000/B000/XXXX";
const WEBHOOK_FINGERPRINT: &str =
    "sha256:4366b08b669f046690849de691e57e9c3b912e418050200186fb9b882bfad1ca";

/// Fixed vector of a text with multi-byte characters (`zażółć`).
const POLISH_TEXT: &str = "za\u{17c}\u{f3}\u{142}\u{107}";
const POLISH_FINGERPRINT: &str =
    "sha256:7eefe5e3046c86cec19ce1215e0a04724124d948468005a1ebc4adf2643bea13";

/// Fixed vector of the empty text.
const EMPTY_FINGERPRINT: &str =
    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn setting_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(301);
    assert_eq!(SettingId::from(raw).as_uuid(), &raw);
}

#[test]
fn setting_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(302);
    assert_eq!(SettingId::from(raw).to_string(), raw.to_string());
}

#[test]
fn keys_of_the_grammar_pass() {
    let keys = [
        "approval.separation_of_duties",
        "alerts.webhook_url",
        "audit.last_check_seq",
        "heartbeat.last_at",
        "a.b",
        "a1.b2.c3",
        "a_.b_",
        "a.b.c.d.e",
    ];
    for key in keys {
        let parsed = SettingKey::parse(key);
        assert!(parsed.is_ok(), "key: {key:?}");
        assert_eq!(parsed.unwrap().as_str(), key, "key: {key:?}");
    }
}

#[test]
fn keys_that_the_sql_check_rejects_fail_with_format() {
    let keys = [
        "",
        "a",
        "abc",
        ".",
        "a.",
        ".b",
        "a..b",
        "a.b.",
        ".a.b",
        "A.b",
        "a.B",
        "aB.c",
        "a.bC",
        "a\u{105}.b",
        "a.b\u{105}",
        "1a.b",
        "a.1b",
        "_a.b",
        "a._b",
        "a-b.c",
        "a.b-c",
        "a b.c",
        "a.b c",
        " a.b",
        "a.b ",
        "a.b\n",
        "\na.b",
        "a.b\0",
        "\u{105}.b",
        "a.\u{105}",
        "a\u{ff0e}b",
    ];
    for key in keys {
        assert_eq!(
            SettingKey::parse(key),
            Err(SettingKeyReason::Format),
            "key: {key:?}"
        );
    }
}

#[test]
fn key_display_is_the_text() {
    let key = SettingKey::parse("session.secure").unwrap();
    assert_eq!(key.to_string(), "session.secure");
}

#[test]
fn value_limit_is_the_value_of_the_reliability_table() {
    assert_eq!(SETTING_VALUE_MAX, 1000);
}

#[test]
fn value_of_1000_characters_passes() {
    let text = "a".repeat(1000);
    assert!(SettingValue::parse(&text).is_ok());
}

#[test]
fn value_of_1000_multi_byte_characters_passes() {
    let text = "\u{e9}".repeat(1000);
    assert!(text.len() > 1000);
    assert!(SettingValue::parse(&text).is_ok());
}

#[test]
fn value_of_1001_characters_fails_with_too_long() {
    let text = "a".repeat(1001);
    assert_eq!(SettingValue::parse(&text), Err(SettingValueReason::TooLong));
}

#[test]
fn value_of_1001_multi_byte_characters_fails_with_too_long() {
    let text = "\u{e9}".repeat(1001);
    assert_eq!(SettingValue::parse(&text), Err(SettingValueReason::TooLong));
}

#[test]
fn value_with_a_nul_character_fails_with_nul_character() {
    assert_eq!(
        SettingValue::parse("a\0"),
        Err(SettingValueReason::NulCharacter)
    );
    assert_eq!(
        SettingValue::parse("\0"),
        Err(SettingValueReason::NulCharacter)
    );
}

#[test]
fn empty_value_passes() {
    let value = SettingValue::parse("").unwrap();
    assert_eq!(value.as_str(), "");
}

#[test]
fn value_keeps_the_input_without_trimming() {
    let value = SettingValue::parse("  a b ").unwrap();
    assert_eq!(value.as_str(), "  a b ");
    assert_eq!(value.to_string(), "  a b ");
}

#[test]
fn fingerprint_of_the_webhook_url_equals_the_fixed_vector() {
    assert_eq!(
        SettingValue::fingerprint(WEBHOOK_URL).as_str(),
        WEBHOOK_FINGERPRINT
    );
}

#[test]
fn fingerprint_hashes_the_utf8_bytes() {
    assert_eq!(
        SettingValue::fingerprint(POLISH_TEXT).as_str(),
        POLISH_FINGERPRINT
    );
}

#[test]
fn fingerprint_of_the_empty_text_equals_the_fixed_vector() {
    assert_eq!(SettingValue::fingerprint("").as_str(), EMPTY_FINGERPRINT);
}

#[test]
fn fingerprint_has_the_prefix_and_64_lower_case_hex_characters() {
    let value = SettingValue::fingerprint(WEBHOOK_URL);
    let hex = value.as_str().strip_prefix("sha256:").unwrap();
    assert_eq!(hex.len(), 64);
    assert!(
        hex.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
}

#[test]
fn fingerprint_passes_the_value_parser() {
    let value = SettingValue::fingerprint(WEBHOOK_URL);
    assert_eq!(SettingValue::parse(value.as_str()), Ok(value));
}

#[test]
fn fingerprint_of_another_text_differs() {
    assert_ne!(
        SettingValue::fingerprint(WEBHOOK_URL),
        SettingValue::fingerprint("https://hooks.example.test/other")
    );
}

#[test]
fn reasons_have_a_fixed_message_without_the_input() {
    assert_eq!(
        SettingKeyReason::Format.to_string(),
        "a setting key has two or more dot-separated segments that start with a to z and hold a to z, 0 to 9, or underscore"
    );
    assert_eq!(
        SettingValueReason::TooLong.to_string(),
        "a setting value has more than 1000 characters"
    );
    assert_eq!(
        SettingValueReason::NulCharacter.to_string(),
        "a setting value has a NUL character"
    );
}
