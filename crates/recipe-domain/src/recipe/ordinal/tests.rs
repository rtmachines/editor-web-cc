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

// reliability.md section 7.11 (`parameter.ordinal_max`): the text names the largest ordinal.
#[test]
fn too_large_text_names_the_largest_ordinal_as_a_lower_case_fragment() {
    assert!(Ordinal::parse(100_000).is_ok());
    assert_eq!(Ordinal::parse(100_001), Err(OrdinalReason::TooLarge));
    let text = OrdinalReason::TooLarge.to_string();
    assert!(
        text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "{text}"
    );
    assert!(!text.ends_with('.'), "{text}");
    assert!(text.contains("100000"), "{text}");
}
