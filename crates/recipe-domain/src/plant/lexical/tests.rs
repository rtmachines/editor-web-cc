use super::*;

#[test]
fn digits_need_one_to_the_maximum_length() {
    assert!(is_digits("0", 5));
    assert!(is_digits("00007", 5));
    assert!(!is_digits("", 5));
    assert!(!is_digits("123456", 5));
    assert!(!is_digits("12a", 5));
    assert!(!is_digits("-1", 5));
}

#[test]
fn canonical_digits_reject_a_leading_zero() {
    assert!(is_canonical_digits("10", 5));
    assert!(!is_canonical_digits("0", 5));
    assert!(!is_canonical_digits("01", 5));
}

#[test]
fn integer_accepts_the_integer_range() {
    assert_eq!(parse_integer("0"), Ok(0));
    assert_eq!(parse_integer("-0"), Ok(0));
    assert_eq!(parse_integer("007"), Ok(7));
    assert_eq!(parse_integer("2147483647"), Ok(i32::MAX));
    assert_eq!(parse_integer("-2147483648"), Ok(i32::MIN));
}

#[test]
fn integer_above_the_range_is_out_of_range() {
    assert_eq!(parse_integer("2147483648"), Err(IntegerError::OutOfRange));
    assert_eq!(parse_integer("-2147483649"), Err(IntegerError::OutOfRange));
}

#[test]
fn integer_of_another_form_is_a_grammar_error() {
    for text in ["", "-", "+1", "1.0", "12345678901", "--1", "1a", " 1"] {
        assert_eq!(parse_integer(text), Err(IntegerError::Grammar), "{text}");
    }
}

#[test]
fn split_keeps_a_list_within_the_limit() {
    assert_eq!(split_at_most("a", ',', 3), Some(vec!["a"]));
    assert_eq!(split_at_most("a,b,c", ',', 3), Some(vec!["a", "b", "c"]));
    assert_eq!(split_at_most("", ',', 3), Some(vec![""]));
}

#[test]
fn split_rejects_a_list_above_the_limit() {
    assert_eq!(split_at_most("a,b,c,d", ',', 3), None);
    assert_eq!(split_at_most(&",".repeat(100_000), ',', 3), None);
}
