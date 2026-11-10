use super::*;
use proptest::prelude::*;

#[test]
fn tag_of_one_character_passes() {
    assert!(ParameterTag::parse("A").is_ok());
}

#[test]
fn tag_of_64_characters_passes() {
    let text = "A".repeat(64);
    assert!(ParameterTag::parse(&text).is_ok());
}

#[test]
fn tag_of_65_characters_fails_with_format() {
    let text = "A".repeat(65);
    assert_eq!(ParameterTag::parse(&text), Err(ParameterTagReason::Format));
}

#[test]
fn lower_case_tag_fails_with_format() {
    assert_eq!(
        ParameterTag::parse("speed"),
        Err(ParameterTagReason::Format)
    );
}

#[test]
fn tag_starting_with_a_digit_fails_with_format() {
    assert_eq!(ParameterTag::parse("1A"), Err(ParameterTagReason::Format));
}

proptest! {
    #[test]
    fn parse_and_as_str_round_trip(text in "[A-Z][A-Z0-9_]{0,63}") {
        let tag = ParameterTag::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(tag.as_str(), text.as_str());
        prop_assert_eq!(tag.to_string(), text);
    }
}

// reliability.md section 7.11 (`text.tag_max`) and 4.3 (row `ParameterTag`): the text names the 1 to 64 length range.
#[test]
fn format_text_names_the_length_limit_as_a_lower_case_fragment() {
    assert!(ParameterTag::parse(&"A".repeat(64)).is_ok());
    assert_eq!(
        ParameterTag::parse(&"A".repeat(65)),
        Err(ParameterTagReason::Format)
    );
    let text = ParameterTagReason::Format.to_string();
    assert!(
        text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "{text}"
    );
    assert!(!text.ends_with('.'), "{text}");
    assert!(text.contains("1 to 64"), "{text}");
}
