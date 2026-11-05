use super::*;
use proptest::prelude::*;

#[test]
fn upper_case_letters_fold_to_lower_case() {
    assert_eq!(
        Login::parse("Admin.User").map(|login| login.as_str().to_owned()),
        Ok("admin.user".to_owned())
    );
}

#[test]
fn three_characters_pass() {
    assert!(Login::parse("abc").is_ok());
}

#[test]
fn sixty_four_characters_pass() {
    let text = "a".repeat(64);
    assert!(Login::parse(&text).is_ok());
}

#[test]
fn two_characters_fail_with_format() {
    assert_eq!(Login::parse("ab"), Err(LoginReason::Format));
}

#[test]
fn sixty_five_characters_fail_with_format() {
    let text = "a".repeat(65);
    assert_eq!(Login::parse(&text), Err(LoginReason::Format));
}

#[test]
fn text_with_a_space_fails_with_format() {
    assert_eq!(Login::parse("a b c"), Err(LoginReason::Format));
}

#[test]
fn non_ascii_letter_fails_with_format() {
    assert_eq!(Login::parse("ł"), Err(LoginReason::Format));
    assert_eq!(Login::parse("łab"), Err(LoginReason::Format));
}

proptest! {
    #[test]
    fn parse_and_display_round_trip(text in "[a-z0-9._-]{3,64}") {
        let login = Login::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(login.as_str(), text.as_str());
        prop_assert_eq!(login.to_string(), text);
    }

    #[test]
    fn upper_case_input_parses_to_lower_case(text in "[a-z0-9._-]{3,64}") {
        let login = Login::parse(&text.to_ascii_uppercase()).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(login.as_str(), text.as_str());
    }
}
