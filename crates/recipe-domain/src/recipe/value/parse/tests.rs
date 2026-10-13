use crate::plant::TypeTree;
use crate::plant::fixtures::id;
use crate::recipe::value::test_support::{
    INT, REAL, USINT, ZONE_JSON, int_array_tree, primitive_tree, zone_tree,
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
fn zone_value_of_the_data_model_example_passes() {
    let value = ParameterValue::parse(ZONE_JSON, &zone_tree()).unwrap();
    let expected: serde_json::Value = serde_json::from_str(ZONE_JSON).unwrap();
    assert_eq!(value.as_json(), &expected);
}

#[test]
fn root_type_is_the_root_id_of_the_tree() {
    let value = ParameterValue::parse(ZONE_JSON, &zone_tree()).unwrap();
    assert_eq!(
        value.root_type(),
        id(crate::recipe::value::test_support::ST_ZONE)
    );
}

#[test]
fn missing_member_gives_a_fault_at_the_struct_path() {
    let json = r#"{"temp_sp": 1, "active": true, "ramps": [1, 2, 3]}"#;
    let fault = fault(json, &zone_tree());
    assert_eq!(fault.leaf().to_string(), "");
    assert_eq!(
        fault.expected(),
        "an object with the members temp_sp, active, ramps, label"
    );
}

#[test]
fn extra_key_gives_a_fault_at_the_struct_path() {
    let json = r#"{"temp_sp": 1, "active": true, "ramps": [1, 2, 3], "label": "", "x": 1}"#;
    assert_eq!(fault(json, &zone_tree()).leaf().to_string(), "");
}

#[test]
fn array_with_two_elements_instead_of_three_gives_a_fault_at_the_array() {
    let json = r#"{"temp_sp": 1, "active": true, "ramps": [1, 2], "label": ""}"#;
    let fault = fault(json, &zone_tree());
    assert_eq!(fault.leaf().to_string(), ".ramps");
    assert_eq!(fault.expected(), "an array of 3 elements");
}

#[test]
fn string_for_an_int_member_gives_a_fault_at_the_member() {
    let json = r#"{"temp_sp": "hot", "active": true, "ramps": [1, 2, 3], "label": ""}"#;
    let fault = fault(json, &zone_tree());
    assert_eq!(fault.leaf().to_string(), ".temp_sp");
}

#[test]
fn string_for_an_int_names_the_type_range() {
    let fault = fault(r#""7""#, &primitive_tree(INT));
    assert_eq!(fault.leaf().to_string(), "");
    assert_eq!(
        fault.expected(),
        "a number of type INT from -32768 to 32767"
    );
}

#[test]
fn element_of_an_array_has_the_plc_index_in_its_path() {
    let json = r#"{"temp_sp": 1, "active": true, "ramps": [1, 2, "x"], "label": ""}"#;
    assert_eq!(fault(json, &zone_tree()).leaf().to_string(), ".ramps[3]");
}

#[test]
fn exponent_form_is_stored_as_plain_decimal_text() {
    assert_eq!(stored_text("1.5e2", &primitive_tree(REAL)), "150");
    assert_eq!(stored_text("1.5E2", &primitive_tree(REAL)), "150");
}

#[test]
fn negative_zero_is_stored_as_zero() {
    assert_eq!(stored_text("-0", &primitive_tree(INT)), "0");
    assert_eq!(stored_text("-0.0", &primitive_tree(REAL)), "0");
}

#[test]
fn trailing_zeros_are_removed() {
    assert_eq!(stored_text("100.50", &primitive_tree(REAL)), "100.5");
}

#[test]
fn number_with_29_significant_digits_gives_a_fault() {
    let digits = "1.2345678901234567890123456789";
    assert!(ParameterValue::parse(digits, &primitive_tree(REAL)).is_err());
}

#[test]
fn number_with_28_significant_digits_passes() {
    let digits = "1.234567890123456789012345678";
    assert_eq!(stored_text(digits, &primitive_tree(REAL)), digits);
}

#[test]
fn value_above_the_type_range_gives_a_fault() {
    assert!(ParameterValue::parse("40000", &primitive_tree(INT)).is_err());
    assert!(ParameterValue::parse("32767", &primitive_tree(INT)).is_ok());
}

#[test]
fn negative_value_for_an_unsigned_type_gives_a_fault() {
    assert!(ParameterValue::parse("-1", &primitive_tree(USINT)).is_err());
}

#[test]
fn fraction_for_an_integer_type_gives_a_fault() {
    let fault = fault("1.5", &primitive_tree(INT));
    assert_eq!(fault.expected(), "an integer of type INT");
}

#[test]
fn whole_number_with_a_zero_fraction_is_an_integer() {
    assert_eq!(stored_text("1.0", &primitive_tree(INT)), "1");
}

#[test]
fn two_dimensional_array_takes_an_array_of_arrays() {
    let tree = int_array_tree(&[(0, 1), (1, 2)]);
    assert_eq!(stored_text("[[1,2],[3,4]]", &tree), "[[1,2],[3,4]]");
}

#[test]
fn flat_list_for_a_two_dimensional_array_gives_a_fault() {
    let tree = int_array_tree(&[(0, 1), (1, 2)]);
    assert_eq!(fault("[1,2,3,4]", &tree).leaf().to_string(), "");
    assert_eq!(fault("[1,2]", &tree).leaf().to_string(), "[0]");
}

#[test]
fn wrong_element_in_a_two_dimensional_array_names_both_plc_indexes() {
    let tree = int_array_tree(&[(0, 1), (1, 2)]);
    assert_eq!(fault("[[1,2],[3,true]]", &tree).leaf().to_string(), "[1,2]");
}

#[test]
fn json_nested_300_levels_gives_a_fault_without_a_panic() {
    let json = format!("{}{}", "[".repeat(300), "]".repeat(300));
    let fault = fault(&json, &zone_tree());
    assert_eq!(fault.leaf().to_string(), "");
}

#[test]
fn invalid_json_gives_a_fault() {
    let fault = fault("{not json", &zone_tree());
    assert_eq!(fault.leaf().to_string(), "");
    assert_eq!(fault.expected(), "valid JSON text");
}

#[test]
fn display_names_the_leaf_and_the_expected_form() {
    let text = fault(r#""7""#, &primitive_tree(INT)).to_string();
    assert_eq!(
        text,
        "Value at leaf path \"\" does not match the type. Expected a number of type INT from -32768 to 32767."
    );
}

/// JSON text of `levels` nested arrays around the number 1.
fn nested_arrays(levels: usize) -> String {
    format!("{}1{}", "[".repeat(levels), "]".repeat(levels))
}

const DEPTH_EXPECTED: &str = "a JSON value with at most 32 levels";

#[test]
fn json_with_33_levels_gives_a_depth_fault_at_the_empty_path() {
    let fault = fault(&nested_arrays(33), &primitive_tree(INT));
    assert_eq!(fault.leaf().to_string(), "");
    assert_eq!(fault.expected(), DEPTH_EXPECTED);
}

#[test]
fn json_with_32_levels_gives_no_depth_fault() {
    let fault = fault(&nested_arrays(32), &primitive_tree(INT));
    assert_ne!(fault.expected(), DEPTH_EXPECTED);
}

#[test]
fn objects_count_as_levels_too() {
    let deep = format!("{}1{}", r#"{"a":"#.repeat(33), "}".repeat(33));
    assert_eq!(
        fault(&deep, &primitive_tree(INT)).expected(),
        DEPTH_EXPECTED
    );
}

#[test]
fn a_stored_value_with_33_levels_gives_the_depth_fault() {
    use crate::recipe::value::StoredValue;
    let json: serde_json::Value = serde_json::from_str(&nested_arrays(33)).unwrap();
    let stored = StoredValue::from(json);
    let fault = ParameterValue::try_from((&stored, &primitive_tree(INT))).unwrap_err();
    assert_eq!(fault.expected(), DEPTH_EXPECTED);
}
