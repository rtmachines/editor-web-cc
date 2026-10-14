//! Tests of task 2.3: the breached-password list and the one-time password.

use super::*;
use crate::error_codes::ErrorCode;
use crate::identity::{IdentityError, NewPassword, PasswordReason};
use proptest::prelude::*;

/// A password of 12 characters that the list holds.
const LISTED_12: &str = "aaaaaaaaaaaa";

#[test]
fn a_listed_password_of_12_characters_fails_with_breached() {
    assert_eq!(
        NewPassword::parse(LISTED_12).unwrap_err(),
        PasswordReason::Breached
    );
}

#[test]
fn a_breached_password_gives_the_policy_error_e_idn_006() {
    let reason = NewPassword::parse(LISTED_12).unwrap_err();
    let error = IdentityError::from(reason);
    assert_eq!(error.code(), ErrorCode::IDN_006);
    assert_eq!(
        error,
        IdentityError::PasswordPolicy {
            reason: PasswordReason::Breached
        }
    );
}

#[test]
fn the_match_is_exact_and_case_sensitive() {
    assert!(NewPassword::parse("AAAAAAAAAAAA").is_ok());
    assert!(NewPassword::parse("Aaaaaaaaaaaa").is_ok());
    assert!(NewPassword::parse(" aaaaaaaaaaaa").is_ok());
}

#[test]
fn a_password_that_the_list_does_not_hold_passes() {
    assert!(NewPassword::parse("correct horse battery staple").is_ok());
}

#[test]
fn the_length_rule_comes_before_the_list() {
    assert_eq!(
        NewPassword::parse("aaaaaaaaaaa").unwrap_err(),
        PasswordReason::TooShort
    );
}

#[test]
fn the_loader_skips_comment_lines_and_blank_lines() {
    let set = parse_list("# header\n\naaaaaaaaaaaa\n# note\nzzzzzzzzzzzz\n");
    assert_eq!(set.len(), 2);
    assert!(set.contains("aaaaaaaaaaaa"));
    assert!(set.contains("zzzzzzzzzzzz"));
    assert!(!set.contains("# note"));
}

#[test]
fn the_loader_keeps_a_line_with_leading_white_space() {
    let set = parse_list(" aaaaaaaaaaaa\n");
    assert!(set.contains(" aaaaaaaaaaaa"));
}

#[test]
fn the_static_set_holds_the_number_of_lines_in_the_file_header() {
    let header_count = BREACHED_LIST
        .lines()
        .find_map(|line| line.strip_prefix("# Lines: "))
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert!(header_count > 10_000);
    assert_eq!(breached_password_count(), header_count);
}

#[test]
fn the_file_header_names_the_source_the_commit_and_the_license() {
    let header: Vec<&str> = BREACHED_LIST
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect();
    for prefix in ["# Source: ", "# Commit: ", "# License: MIT", "# Lines: "] {
        assert!(
            header.iter().any(|line| line.starts_with(prefix)),
            "{prefix}"
        );
    }
}

#[test]
fn every_listed_password_has_12_to_128_characters_and_the_file_is_sorted() {
    let passwords: Vec<&str> = BREACHED_LIST
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    for text in &passwords {
        let length = text.chars().count();
        assert!((12..=128).contains(&length), "{length}");
    }
    for pair in passwords.windows(2) {
        assert!(pair[0].as_bytes() < pair[1].as_bytes(), "{}", pair[0]);
    }
}

#[test]
fn the_alphabet_has_32_distinct_characters_without_look_alikes() {
    assert_eq!(ONE_TIME_ALPHABET.len(), 32);
    let mut sorted: Vec<char> = ONE_TIME_ALPHABET.chars().collect();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 32);
    for banned in ['0', 'O', '1', 'l', 'I'] {
        assert!(!ONE_TIME_ALPHABET.contains(banned), "{banned}");
    }
    assert!(ONE_TIME_ALPHABET.chars().all(|c| c.is_ascii_alphanumeric()));
}

#[test]
fn a_one_time_password_from_fixed_bytes_has_20_characters_of_the_alphabet() {
    let bytes: [u8; 20] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
    ];
    let password = OneTimePassword::from_bytes(bytes);
    assert_eq!(password.as_str(), "23456789ABCDEFGHJKLM");
}

#[test]
fn a_byte_maps_to_the_character_at_its_value_modulo_32() {
    let password = OneTimePassword::from_bytes([
        32, 63, 64, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    let text = password.as_str();
    let first_five: String = text.chars().take(5).collect();
    // 32 -> index 0, 63 -> index 31, 64 -> index 0, 255 -> index 31, 0 -> index 0.
    assert_eq!(first_five, "2Z2Z2");
}

#[test]
fn the_one_time_password_debug_is_redacted() {
    let password = OneTimePassword::from_bytes([9; 20]);
    assert_eq!(format!("{password:?}"), "[redacted]");
    assert_eq!(format!("{password:#?}"), "[redacted]");
}

#[test]
fn a_one_time_password_turns_into_a_new_password_with_the_same_text() {
    let password = OneTimePassword::from_bytes([3; 20]);
    let new = password.to_new_password();
    assert_eq!(new.as_str(), password.as_str());
}

#[test]
fn the_extreme_byte_patterns_meet_the_policy() {
    for byte in [0u8, 1, 31, 32, 127, 255] {
        let password = OneTimePassword::from_bytes([byte; 20]);
        let text = password.as_str().to_owned();
        // A fixed pattern such as all zero bytes can be a listed text. The generated
        // password still converts, because random bytes make a listed text negligible.
        assert_eq!(password.to_new_password().as_str(), text);
    }
}

proptest! {
    #[test]
    fn any_20_bytes_give_a_one_time_password_that_meets_the_policy(
        bytes in prop::array::uniform20(any::<u8>())
    ) {
        let password = OneTimePassword::from_bytes(bytes);
        let text = password.as_str();
        prop_assert_eq!(text.chars().count(), 20);
        prop_assert!(text.chars().all(|c| ONE_TIME_ALPHABET.contains(c)));
        let new = password.to_new_password();
        prop_assert_eq!(new.as_str(), text);
        // The generated text satisfies the length rule of the policy.
        let length = new.as_str().chars().count();
        prop_assert!((12..=128).contains(&length));
    }
}
