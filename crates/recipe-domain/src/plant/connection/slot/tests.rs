use super::*;

const OUT_OF_RANGE: RangeReason = RangeReason::OutOfRange { min: 0, max: 31 };

#[test]
fn slot_boundaries_pass() {
    assert_eq!(Slot::parse("0").map(Slot::value), Ok(0));
    assert_eq!(Slot::parse("31").map(Slot::value), Ok(31));
    assert_eq!(Slot::parse("2").map(Slot::value), Ok(2));
}

#[test]
fn slot_above_the_range_fails() {
    assert_eq!(Slot::parse("32"), Err(OUT_OF_RANGE));
    assert_eq!(Slot::parse("256"), Err(OUT_OF_RANGE));
    assert_eq!(Slot::parse("99999999999999999999"), Err(OUT_OF_RANGE));
}

#[test]
fn negative_slot_fails() {
    assert_eq!(Slot::parse("-1"), Err(RangeReason::NotANumber));
}

#[test]
fn slot_that_is_not_a_number_fails() {
    for text in ["x", "", " 1", "1 ", "+1", "1.0", "0x1", "\u{0663}"] {
        assert_eq!(Slot::parse(text), Err(RangeReason::NotANumber), "{text:?}");
    }
}
