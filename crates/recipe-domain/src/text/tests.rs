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

/// Assert the shape of a set of reason texts: not empty, distinct, a lower-case
/// fragment, and no final period (ADR-0025, the text is a clause in a longer message).
fn assert_reason_texts_are_clauses(texts: &[String]) {
    let distinct: std::collections::BTreeSet<&String> = texts.iter().collect();
    assert_eq!(distinct.len(), texts.len(), "texts differ: {texts:?}");
    for text in texts {
        assert!(!text.is_empty(), "empty reason text");
        assert!(!text.ends_with('.'), "final period: {text:?}");
        assert!(
            text.chars().next().is_some_and(char::is_lowercase),
            "not a lower-case fragment: {text:?}"
        );
    }
}

#[test]
fn reliability_7_11_text_reason_too_long_names_the_name_limit() {
    // `text.name_max` = 200.
    let text = TextReason::TooLong.to_string();
    assert!(text.contains("200"), "{text:?}");
}

#[test]
fn reliability_7_11_long_text_reason_too_long_names_the_long_text_limit() {
    // `text.description_max` = `text.change_note_max` = 4 000.
    let text = LongTextReason::TooLong.to_string();
    assert!(text.contains("4000"), "{text:?}");
}

#[test]
fn reliability_4_3_long_text_control_reason_names_the_line_end_exception() {
    let text = LongTextReason::ControlCharacter.to_string();
    assert!(text.contains("line end"), "{text:?}");
    assert_ne!(text, TextReason::ControlCharacter.to_string());
}

#[test]
fn adr_0025_text_reasons_are_distinct_lower_case_clauses() {
    let texts = [
        TextReason::Empty,
        TextReason::TooLong,
        TextReason::ControlCharacter,
    ]
    .map(|reason| reason.to_string());
    assert_reason_texts_are_clauses(&texts);
}

#[test]
fn adr_0025_long_text_reasons_are_distinct_lower_case_clauses() {
    let texts = [
        LongTextReason::Empty,
        LongTextReason::TooLong,
        LongTextReason::ControlCharacter,
    ]
    .map(|reason| reason.to_string());
    assert_reason_texts_are_clauses(&texts);
}

mod display_round_trip {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// `reliability.md` section 4.3: the stored text keeps the input, so `Display` returns it.
        #[test]
        fn reliability_4_3_long_text_display_returns_the_parsed_input(
            input in "[a-zé]([a-zA-Z0-9é .,]|\n|\r\n){0,200}[a-z]"
        ) {
            let parsed = LongText::parse(&input);
            prop_assert!(parsed.is_ok(), "{:?}", parsed);
            if let Ok(long_text) = parsed {
                prop_assert_eq!(long_text.to_string(), input);
            }
        }
    }
}
