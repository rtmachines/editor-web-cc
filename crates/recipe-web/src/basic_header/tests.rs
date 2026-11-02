//! Tests of the parser contract of ADR-0109 (ma).

use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, HeaderValue};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use recipe_app::identity::BasicCredentials;
use recipe_domain::error_codes::ErrorCode;

use super::{BASIC_HEADER_MAX, parse_basic};
use crate::error::WebError;
use crate::test_support::capture_logs;

/// A password that the tests never want in a log line or a `Debug` text. It holds a colon.
const SECRET: &str = "s3cret-p4ss:word";

fn headers_of(values: &[&str]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for value in values {
        headers.append(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
    }
    headers
}

fn basic(pair: &str) -> String {
    format!("Basic {}", STANDARD.encode(pair))
}

fn parse(value: &str) -> Result<BasicCredentials, WebError> {
    parse_basic(&headers_of(&[value]))
}

fn assert_missing(result: Result<BasicCredentials, WebError>, case: &str) {
    let error = result.expect_err(case);
    assert!(
        matches!(error, WebError::MissingBasicCredentials),
        "{case}: {error:?}"
    );
    assert_eq!(error.code(), ErrorCode::WEB_012, "{case}");
}

#[test]
fn basic_parser_reads_login_and_password() {
    let credentials = parse(&basic("Anna:hunter2")).unwrap();
    assert_eq!(credentials.login.as_str(), "anna");
    assert_eq!(credentials.password.as_str(), "hunter2");
}

#[test]
fn basic_parser_splits_at_the_first_colon_so_the_password_holds_colons() {
    let credentials = parse(&basic(&format!("anna:{SECRET}"))).unwrap();
    assert_eq!(credentials.login.as_str(), "anna");
    assert_eq!(credentials.password.as_str(), SECRET);
}

#[test]
fn basic_parser_accepts_a_password_of_128_characters_and_refuses_129() {
    let at_limit = "p".repeat(128);
    let credentials = parse(&basic(&format!("anna:{at_limit}"))).unwrap();
    assert_eq!(credentials.password.as_str(), at_limit);
    let over_limit = "p".repeat(129);
    assert_missing(
        parse(&basic(&format!("anna:{over_limit}"))),
        "129 characters",
    );
}

#[test]
fn basic_parser_counts_password_characters_and_not_bytes() {
    let at_limit = "ż".repeat(128);
    assert!(parse(&basic(&format!("anna:{at_limit}"))).is_ok());
}

#[test]
fn basic_parser_ignores_the_case_of_the_scheme() {
    for scheme in ["basic", "BASIC", "bAsIc", "Basic"] {
        let value = format!("{scheme} {}", STANDARD.encode("anna:hunter2"));
        assert!(parse(&value).is_ok(), "{scheme}");
    }
}

#[test]
fn basic_parser_refuses_another_scheme() {
    let token = STANDARD.encode("anna:hunter2");
    for scheme in ["Bearer", "Digest", "Basicx", "RecipeExport"] {
        assert_missing(parse(&format!("{scheme} {token}")), scheme);
    }
}

#[test]
fn basic_parser_refuses_a_missing_header() {
    assert_missing(parse_basic(&HeaderMap::new()), "no header");
}

#[test]
fn basic_parser_refuses_a_duplicate_header_even_when_both_are_valid() {
    let valid = basic("anna:hunter2");
    assert_missing(parse_basic(&headers_of(&[&valid, &valid])), "duplicate");
}

#[test]
fn basic_parser_refuses_a_header_over_the_limit() {
    assert_eq!(BASIC_HEADER_MAX, 1024);
    let prefix = "Basic ";
    let filler = "A".repeat(BASIC_HEADER_MAX - prefix.len() + 1);
    let value = format!("{prefix}{filler}");
    assert_eq!(value.len(), BASIC_HEADER_MAX + 1);
    assert_missing(parse(&value), "1 025 bytes");
}

#[test]
fn basic_parser_counts_the_whole_header_and_not_only_the_token() {
    let token = STANDARD.encode(format!("anna:{}", "p".repeat(100)));
    let tail = " ".repeat(BASIC_HEADER_MAX);
    assert_missing(parse(&format!("Basic {token}{tail}")), "long tail");
}

#[test]
fn basic_parser_refuses_base64_without_padding_or_outside_the_standard_alphabet() {
    let pair = "anna:hunter2!";
    let unpadded = STANDARD_NO_PAD.encode(pair);
    assert_ne!(unpadded, STANDARD.encode(pair), "the case needs padding");
    assert_missing(parse(&format!("Basic {unpadded}")), "no padding");
    // The URL-safe alphabet uses `-` and `_`. The standard alphabet has neither.
    assert_missing(parse("Basic YW5uYTo-Pz4_"), "url-safe alphabet");
    assert_missing(parse("Basic !!!!"), "not Base64");
    assert_missing(parse("Basic"), "no token");
    assert_missing(parse("Basic "), "empty token");
}

#[test]
fn basic_parser_refuses_bytes_that_are_not_utf8() {
    let value = format!("Basic {}", STANDARD.encode([b'a', b':', 0xff, 0xfe]));
    assert_missing(parse(&value), "not UTF-8");
}

#[test]
fn basic_parser_refuses_text_without_a_colon() {
    assert_missing(parse(&basic("annahunter2")), "no colon");
}

#[test]
fn basic_parser_refuses_a_bad_login_length_and_an_empty_password() {
    assert_missing(parse(&basic(":hunter2")), "empty login");
    assert_missing(parse(&basic("anna:")), "empty password");
    let too_long = format!("{}:hunter2", "a".repeat(65));
    assert_missing(parse(&basic(&too_long)), "login of 65 characters");
    let at_limit = format!("{}:hunter2", "a".repeat(64));
    assert!(parse(&basic(&at_limit)).is_ok());
}

#[test]
fn basic_parser_refuses_a_control_character_in_the_login_or_the_password() {
    for (case, pair) in [
        ("nul in login", "an\u{0}na:hunter2"),
        ("tab in login", "an\tna:hunter2"),
        ("newline in password", "anna:hunt\ner2"),
        ("del in password", "anna:hunt\u{7f}er2"),
        ("c1 in password", "anna:hunt\u{85}er2"),
        ("nul in password", "anna:hunter2\u{0}"),
    ] {
        assert_missing(parse(&basic(pair)), case);
    }
}

#[test]
fn basic_credentials_debug_never_prints_the_password() {
    let credentials = parse(&basic(&format!("anna:{SECRET}"))).unwrap();
    let text = format!("{credentials:?}");
    assert!(!text.contains("s3cret"), "{text}");
    assert!(!text.contains(SECRET), "{text}");
}

#[test]
fn basic_error_carries_no_part_of_the_header() {
    let value = basic(&format!("anna:{}", "p".repeat(129)));
    let error = parse(&value).unwrap_err();
    let shown = format!("{error} {error:?}");
    assert!(!shown.contains(&value), "{shown}");
    assert!(!shown.contains("anna"), "{shown}");
}

#[test]
fn basic_parser_writes_no_log_line_with_the_header_value() {
    let (buffer, _guard) = capture_logs();
    let token = STANDARD.encode(format!("anna:{SECRET}"));
    let bad_cases = [
        format!("Bearer {token}"),
        format!("Basic {token}x"),
        basic(&format!("anna:{}", "p".repeat(129))),
        basic(&format!("an\u{1}na:{SECRET}")),
        format!("Basic {token}{}", "A".repeat(BASIC_HEADER_MAX)),
    ];
    for value in &bad_cases {
        assert_missing(parse(value), value);
    }
    let duplicate = headers_of(&[&basic("anna:ok"), &format!("Basic {token}")]);
    assert_missing(parse_basic(&duplicate), "duplicate");
    assert!(parse(&format!("Basic {token}")).is_ok());
    let log = buffer.text();
    for needle in [token.as_str(), SECRET, "s3cret", "Basic "] {
        assert!(!log.contains(needle), "{needle} in {log}");
    }
}
