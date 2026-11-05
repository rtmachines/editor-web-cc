use super::*;

#[test]
fn ordinal_zero_passes() {
    assert_eq!(Ordinal::parse(0).map(|ordinal| ordinal.value()), Ok(0));
}

#[test]
fn ordinal_100_000_passes() {
    assert!(Ordinal::parse(100_000).is_ok());
}

#[test]
fn ordinal_100_001_fails_with_too_large() {
    assert_eq!(Ordinal::parse(100_001), Err(OrdinalReason::TooLarge));
}
