use super::*;

const OUT_OF_RANGE: RangeReason = RangeReason::OutOfRange { min: 0, max: 7 };

#[test]
fn rack_boundaries_pass() {
    assert_eq!(Rack::parse("0").map(Rack::value), Ok(0));
    assert_eq!(Rack::parse("7").map(Rack::value), Ok(7));
    assert_eq!(Rack::parse("3").map(Rack::value), Ok(3));
}

#[test]
fn rack_above_the_range_fails() {
    assert_eq!(Rack::parse("8"), Err(OUT_OF_RANGE));
    assert_eq!(Rack::parse("255"), Err(OUT_OF_RANGE));
    assert_eq!(Rack::parse("99999999999999999999"), Err(OUT_OF_RANGE));
}

#[test]
fn negative_rack_fails() {
    assert_eq!(Rack::parse("-1"), Err(RangeReason::NotANumber));
}

#[test]
fn rack_that_is_not_a_number_fails() {
    for text in ["x", "", " 1", "1 ", "+1", "1.0", "0x1", "\u{0663}"] {
        assert_eq!(Rack::parse(text), Err(RangeReason::NotANumber), "{text:?}");
    }
}
