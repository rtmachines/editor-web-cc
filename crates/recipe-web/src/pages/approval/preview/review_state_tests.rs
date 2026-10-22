//! Tests of `review_state`, the reader of the query value `rv` of the approval preview.

use super::review_state;
use crate::error::{FieldReason, WebError};

fn is_bad_rv(error: &WebError) -> bool {
    matches!(
        error,
        WebError::BadRequest {
            field: "rv",
            reason: FieldReason::Query
        }
    )
}

#[test]
fn review_state_without_a_value_is_none() {
    assert_eq!(review_state(None).unwrap(), None);
}

#[test]
fn review_state_accepts_a_positive_whole_number_up_to_i64_max() {
    let cases = [("1", 1), ("42", 42), ("9223372036854775807", i64::MAX)];
    for (text, expected) in cases {
        assert_eq!(
            review_state(Some(text)).unwrap(),
            Some(expected),
            "{text:?}"
        );
    }
}

#[test]
fn review_state_refuses_anything_else_with_the_field_rv_and_the_reason_query() {
    for text in [
        "",
        "0",
        "01",
        "-3",
        "+3",
        "abc",
        " 1",
        "1.5",
        "9223372036854775808",
    ] {
        let error = review_state(Some(text)).unwrap_err();
        assert!(is_bad_rv(&error), "{text:?}");
    }
}
