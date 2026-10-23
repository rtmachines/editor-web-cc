use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use serde_json::Value;
use uuid::Uuid;

use super::{error_body, fallback_body};

const TRICKY: &str =
    "quote \" backslash \\ polish \u{17c}\u{f3}\u{142}\u{107} control \u{1} tab \t newline \n";

const REQUEST_TEXT: &str = "00000000-0000-0000-0000-00000000002a";

fn request_id() -> RequestId {
    RequestId::from(Uuid::from_u128(42))
}

fn text_of(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

fn parse(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn body_has_the_exact_bytes_of_the_compact_object() {
    let body = error_body(
        ErrorCode::EXP_001,
        "Recipe R-100 has no approved version.",
        request_id(),
    );

    assert_eq!(
        text_of(&body),
        format!(
            r#"{{"code":"E-EXP-001","message":"Recipe R-100 has no approved version.","request_id":"{REQUEST_TEXT}"}}"#
        )
    );
}

#[test]
fn fields_come_in_the_order_code_message_request_id() {
    let body = error_body(ErrorCode::EXP_002, "m", request_id());
    let text = text_of(&body);

    let code = text.find("\"code\"").unwrap();
    let message = text.find("\"message\"").unwrap();
    let request_id = text.find("\"request_id\"").unwrap();
    assert!(code < message && message < request_id, "{text}");
}

#[test]
fn special_characters_of_the_message_are_escaped_and_parse_back() {
    let body = error_body(ErrorCode::EXP_003, TRICKY, request_id());

    let value = parse(&body);
    assert_eq!(value["code"], "E-EXP-003");
    assert_eq!(value["message"], TRICKY);
    assert_eq!(value["request_id"], REQUEST_TEXT);
    assert_eq!(value.as_object().unwrap().len(), 3);
}

#[test]
fn special_characters_of_the_message_have_the_exact_escaped_bytes() {
    let body = error_body(ErrorCode::EXP_004, "a\"b\\c\u{1}\u{142}", request_id());

    assert_eq!(
        text_of(&body),
        format!(
            "{{\"code\":\"E-EXP-004\",\"message\":\"a\\\"b\\\\c\\u0001\u{142}\",\"request_id\":\"{REQUEST_TEXT}\"}}"
        )
    );
}

#[test]
fn empty_message_gives_an_empty_string() {
    let body = error_body(ErrorCode::EXP_001, "", request_id());

    assert_eq!(
        text_of(&body),
        format!(r#"{{"code":"E-EXP-001","message":"","request_id":"{REQUEST_TEXT}"}}"#)
    );
}

#[test]
fn fallback_body_has_an_empty_message_and_the_exact_bytes() {
    let body = fallback_body(ErrorCode::EXP_004, request_id());

    assert_eq!(
        text_of(&body),
        format!(r#"{{"code":"E-EXP-004","message":"","request_id":"{REQUEST_TEXT}"}}"#)
    );
}

#[test]
fn fallback_body_is_valid_json_with_the_same_code_and_request_id() {
    let body = fallback_body(ErrorCode::EXP_002, request_id());

    let value = parse(&body);
    assert_eq!(value["code"], "E-EXP-002");
    assert_eq!(value["message"], "");
    assert_eq!(value["request_id"], REQUEST_TEXT);
    assert_eq!(value.as_object().unwrap().len(), 3);
}

#[test]
fn fallback_body_has_the_same_bytes_as_the_normal_body_with_an_empty_message() {
    let normal = error_body(ErrorCode::EXP_003, "", request_id());

    assert_eq!(fallback_body(ErrorCode::EXP_003, request_id()), normal);
}
