use super::*;

#[test]
fn name_of_one_character_passes() {
    assert!(Text::parse("a").is_ok());
}

#[test]
fn name_of_200_characters_passes_even_with_more_bytes() {
    let name = "é".repeat(200);
    assert!(Text::parse(&name).is_ok());
}

#[test]
fn name_of_201_characters_fails_with_too_long() {
    let name = "a".repeat(201);
    assert_eq!(Text::parse(&name), Err(TextReason::TooLong));
}

#[test]
fn empty_name_fails_with_empty() {
    assert_eq!(Text::parse(""), Err(TextReason::Empty));
}

#[test]
fn blank_name_fails_with_empty() {
    assert_eq!(Text::parse("   "), Err(TextReason::Empty));
}

#[test]
fn tab_inside_name_fails_with_control_character() {
    assert_eq!(Text::parse("a\tb"), Err(TextReason::ControlCharacter));
}

#[test]
fn bell_inside_name_fails_with_control_character() {
    assert_eq!(Text::parse("a\u{7}"), Err(TextReason::ControlCharacter));
}

#[test]
fn as_str_returns_the_input_without_trimming() {
    let parsed = Text::parse("  a b ").expect("valid text");
    assert_eq!(parsed.as_str(), "  a b ");
    assert_eq!(parsed.to_string(), "  a b ");
}

#[test]
fn long_text_of_4000_multi_byte_characters_passes() {
    let text = "é".repeat(4000);
    assert!(LongText::parse(&text).is_ok());
}

#[test]
fn long_text_of_4001_characters_fails_with_too_long() {
    let text = "a".repeat(4001);
    assert_eq!(LongText::parse(&text), Err(LongTextReason::TooLong));
}

#[test]
fn long_text_with_lf_passes() {
    assert!(LongText::parse("a\nb").is_ok());
}

#[test]
fn long_text_with_crlf_passes_and_keeps_the_input() {
    let parsed = LongText::parse("a\r\nb").expect("valid text");
    assert_eq!(parsed.as_str(), "a\r\nb");
}

#[test]
fn long_text_of_blank_characters_fails_with_empty() {
    assert_eq!(LongText::parse("   "), Err(LongTextReason::Empty));
    assert_eq!(LongText::parse(""), Err(LongTextReason::Empty));
}

#[test]
fn long_text_with_lone_cr_fails_with_control_character() {
    assert_eq!(
        LongText::parse("a\rb"),
        Err(LongTextReason::ControlCharacter)
    );
    assert_eq!(
        LongText::parse("a\r"),
        Err(LongTextReason::ControlCharacter)
    );
}

#[test]
fn long_text_with_tab_fails_with_control_character() {
    assert_eq!(
        LongText::parse("a\tb"),
        Err(LongTextReason::ControlCharacter)
    );
}
