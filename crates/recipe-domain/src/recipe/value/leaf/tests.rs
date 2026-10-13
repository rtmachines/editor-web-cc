use crate::plant::TypeTree;
use crate::recipe::value::test_support::{
    BOOL, CHAR, DATE, LTIME, REAL, TIME, WCHAR, primitive_tree, string_tree, wstring_tree,
};
use crate::recipe::value::{ParameterValue, ValueFault};

/// Parse `json` and return the stored JSON text.
fn stored_text(json: &str, tree: &TypeTree) -> String {
    ParameterValue::parse(json, tree)
        .unwrap()
        .as_json()
        .to_string()
}

/// Parse `json` and return the fault.
fn fault(json: &str, tree: &TypeTree) -> ValueFault {
    ParameterValue::parse(json, tree).unwrap_err()
}

#[test]
fn string_with_a_character_outside_latin_1_gives_a_fault() {
    let fault = fault(r#""Łódź""#, &string_tree(10));
    assert_eq!(fault.leaf().to_string(), "");
    assert_eq!(
        fault.expected(),
        "a string of at most 10 characters from Latin-1"
    );
}

#[test]
fn string_with_latin_1_characters_passes() {
    assert_eq!(stored_text(r#""Zéàü""#, &string_tree(10)), r#""Zéàü""#);
}

#[test]
fn string_longer_than_its_type_gives_a_fault() {
    assert!(ParameterValue::parse(r#""ABCDE""#, &string_tree(4)).is_err());
}

#[test]
fn string_as_long_as_its_type_passes() {
    assert_eq!(stored_text(r#""ABCD""#, &string_tree(4)), r#""ABCD""#);
}

#[test]
fn string_length_counts_characters_and_not_bytes() {
    assert_eq!(stored_text(r#""éééé""#, &string_tree(4)), r#""éééé""#);
}

#[test]
fn number_for_a_string_gives_a_fault() {
    assert!(ParameterValue::parse("5", &string_tree(4)).is_err());
}

#[test]
fn wstring_counts_utf16_units() {
    let fault = fault(r#""a😀""#, &wstring_tree(2));
    assert_eq!(fault.expected(), "a string of at most 2 UTF-16 units");
}

#[test]
fn wstring_with_one_surrogate_pair_fits_two_units() {
    assert_eq!(stored_text(r#""😀""#, &wstring_tree(2)), r#""😀""#);
}

#[test]
fn wstring_accepts_characters_outside_latin_1() {
    assert_eq!(stored_text(r#""Łódź""#, &wstring_tree(4)), r#""Łódź""#);
}

#[test]
fn number_and_text_for_a_bool_give_a_fault() {
    let tree = primitive_tree(BOOL);
    assert_eq!(fault("1", &tree).expected(), "true or false");
    assert!(ParameterValue::parse(r#""true""#, &tree).is_err());
    assert!(ParameterValue::parse("null", &tree).is_err());
}

#[test]
fn bool_values_pass() {
    let tree = primitive_tree(BOOL);
    assert_eq!(stored_text("true", &tree), "true");
    assert_eq!(stored_text("false", &tree), "false");
}

#[test]
fn char_takes_exactly_one_scalar_up_to_latin_1() {
    let tree = primitive_tree(CHAR);
    assert!(ParameterValue::parse(r#""ab""#, &tree).is_err());
    assert!(ParameterValue::parse(r#""""#, &tree).is_err());
    assert!(ParameterValue::parse(r#""Ł""#, &tree).is_err());
    assert_eq!(stored_text(r#""é""#, &tree), r#""é""#);
}

#[test]
fn wchar_takes_one_scalar_up_to_the_basic_plane() {
    let tree = primitive_tree(WCHAR);
    assert!(ParameterValue::parse(r#""😀""#, &tree).is_err());
    assert_eq!(stored_text(r#""Ł""#, &tree), r#""Ł""#);
}

#[test]
fn number_for_a_char_gives_a_fault() {
    assert!(ParameterValue::parse("65", &primitive_tree(CHAR)).is_err());
}

#[test]
fn time_literal_is_stored_as_a_count() {
    assert_eq!(stored_text(r#""T#90m""#, &primitive_tree(TIME)), "5400000");
}

#[test]
fn time_literal_with_parts_in_the_wrong_order_gives_a_fault() {
    let fault = fault(r#""T#2h1d""#, &primitive_tree(TIME));
    assert_eq!(fault.expected(), "an IEC literal of type TIME");
}

#[test]
fn time_literal_with_the_prefix_of_another_type_gives_a_fault() {
    assert!(ParameterValue::parse(r#""LT#5s""#, &primitive_tree(TIME)).is_err());
}

#[test]
fn negative_time_on_twincat3_gives_a_fault() {
    assert!(ParameterValue::parse(r#""T#-1s""#, &primitive_tree(TIME)).is_err());
}

#[test]
fn ltime_literal_is_stored_as_a_count_of_nanoseconds() {
    assert_eq!(
        stored_text(r#""LT#5s""#, &primitive_tree(LTIME)),
        "5000000000"
    );
}

#[test]
fn date_literal_is_stored_as_a_count() {
    assert_eq!(
        stored_text(r#""D#2026-10-12""#, &primitive_tree(DATE)),
        "20738"
    );
}

#[test]
fn number_for_a_time_leaf_gives_a_fault_in_a_request() {
    assert!(ParameterValue::parse("5400000", &primitive_tree(TIME)).is_err());
}

#[test]
fn real_takes_one_significant_digit_with_a_large_exponent() {
    assert_eq!(
        stored_text("1e28", &primitive_tree(REAL)),
        "10000000000000000000000000000"
    );
}

#[test]
fn real_with_29_significant_digits_gives_a_fault() {
    assert!(
        ParameterValue::parse("1.2345678901234567890123456789", &primitive_tree(REAL)).is_err()
    );
}

#[test]
fn real_with_29_significant_digits_in_scientific_form_gives_a_fault() {
    let tree = primitive_tree(REAL);
    assert!(ParameterValue::parse("1.00000000000000000000000000001e0", &tree).is_err());
}

#[test]
fn parse_decimal_follows_the_json_number_grammar() {
    use crate::recipe::value::parse_decimal;
    for text in [
        "1_000", "+5", ".5", "5.", "01", "", "-", "1e", "1e+", "0x10", " 5",
    ] {
        assert!(parse_decimal(text).is_none(), "{text:?}");
    }
    for text in ["0", "-0", "10", "-1.5", "1e2", "1E+2", "2.5e-3"] {
        assert!(parse_decimal(text).is_some(), "{text:?}");
    }
}
