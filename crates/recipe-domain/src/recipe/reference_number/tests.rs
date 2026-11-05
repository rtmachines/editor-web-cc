use super::*;
use proptest::prelude::*;

#[test]
fn lower_case_input_is_folded_to_upper_case() {
    assert_eq!(
        ReferenceNumber::parse("r-100").map(|r| r.as_str().to_owned()),
        Ok("R-100".to_owned())
    );
}

#[test]
fn fifty_characters_pass() {
    let text = "A".repeat(50);
    assert!(ReferenceNumber::parse(&text).is_ok());
}

#[test]
fn fifty_one_characters_fail() {
    let text = "A".repeat(51);
    assert_eq!(
        ReferenceNumber::parse(&text),
        Err(ReferenceNumberReason::Format)
    );
}

#[test]
fn invalid_texts_fail_with_format() {
    for text in ["", "-R1", "_R1", "R 1", "Rß", "R.1"] {
        assert_eq!(
            ReferenceNumber::parse(text),
            Err(ReferenceNumberReason::Format),
            "{text}"
        );
    }
}

proptest! {
    #[test]
    fn parse_and_as_str_round_trip(text in "[A-Z0-9][A-Z0-9_-]{0,49}") {
        let reference = ReferenceNumber::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(reference.as_str(), text.as_str());
        prop_assert_eq!(reference.to_string(), text);
    }

    #[test]
    fn lower_case_input_parses_to_upper_case(text in "[A-Z0-9][A-Z0-9_-]{0,49}") {
        let reference = ReferenceNumber::parse(&text.to_ascii_lowercase()).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(reference.as_str(), text.as_str());
    }
}
