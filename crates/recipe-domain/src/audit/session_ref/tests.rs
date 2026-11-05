use super::*;

#[test]
fn sixteen_lower_case_hex_characters_pass() {
    let parsed = SessionRef::parse("0123456789abcdef").unwrap();
    assert_eq!(parsed.as_str(), "0123456789abcdef");
    assert_eq!(parsed.to_string(), "0123456789abcdef");
}

#[test]
fn fifteen_characters_fail() {
    assert_eq!(
        SessionRef::parse("0123456789abcde"),
        Err(SessionRefReason::Format)
    );
}

#[test]
fn seventeen_characters_fail() {
    assert_eq!(
        SessionRef::parse("0123456789abcdef0"),
        Err(SessionRefReason::Format)
    );
}

#[test]
fn upper_case_hex_fails() {
    assert_eq!(
        SessionRef::parse("0123456789ABCDEF"),
        Err(SessionRefReason::Format)
    );
}

#[test]
fn a_non_hex_letter_fails() {
    assert_eq!(
        SessionRef::parse("0123456789abcdeg"),
        Err(SessionRefReason::Format)
    );
}

#[test]
fn sixteen_multi_byte_characters_fail() {
    assert_eq!(
        SessionRef::parse(
            "\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}"
        ),
        Err(SessionRefReason::Format)
    );
}

#[test]
fn the_empty_text_fails() {
    assert_eq!(SessionRef::parse(""), Err(SessionRefReason::Format));
}

#[test]
fn the_reason_displays_a_sentence() {
    assert!(!SessionRefReason::Format.to_string().is_empty());
}
