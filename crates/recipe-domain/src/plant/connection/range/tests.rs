use super::*;

#[test]
fn bounds_are_inclusive() {
    assert_eq!(parse_in_range("1", 1, 65535), Ok(1));
    assert_eq!(parse_in_range("65535", 1, 65535), Ok(65535));
}

#[test]
fn below_the_minimum_fails() {
    assert_eq!(
        parse_in_range("0", 1, 65535),
        Err(RangeReason::OutOfRange { min: 1, max: 65535 })
    );
}

#[test]
fn above_the_maximum_fails() {
    assert_eq!(
        parse_in_range("65536", 1, 65535),
        Err(RangeReason::OutOfRange { min: 1, max: 65535 })
    );
}

#[test]
fn display_names_the_range() {
    assert_eq!(
        RangeReason::OutOfRange { min: 0, max: 7 }.to_string(),
        "a value is a number from 0 to 7"
    );
    assert!(!RangeReason::NotANumber.to_string().is_empty());
}
