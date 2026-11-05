use super::crlf_to_lf;
use std::borrow::Cow;

#[test]
fn crlf_becomes_lf() {
    assert_eq!(crlf_to_lf("Line 1\r\nLine 2\r\n"), "Line 1\nLine 2\n");
}

#[test]
fn lone_lf_stays() {
    assert_eq!(crlf_to_lf("Line 1\nLine 2"), "Line 1\nLine 2");
}

#[test]
fn lone_cr_stays() {
    assert_eq!(crlf_to_lf("Line 1\rLine 2"), "Line 1\rLine 2");
}

#[test]
fn cr_before_a_crlf_stays_unchanged() {
    let text = "a\r\r\nb";
    assert!(matches!(crlf_to_lf(text), Cow::Borrowed(same) if same == text));
}

#[test]
fn lone_cr_after_a_crlf_stays_unchanged() {
    let text = "a\r\nb\rc";
    assert!(matches!(crlf_to_lf(text), Cow::Borrowed(same) if same == text));
}

#[test]
fn text_without_line_end_is_unchanged() {
    let text = "no line end";
    assert!(matches!(crlf_to_lf(text), Cow::Borrowed(same) if same == text));
    assert_eq!(crlf_to_lf(""), "");
}
