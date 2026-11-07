use serde_json::{Value, json};

use crate::plant::{Primitive, RuleSet, TypeTree};
use crate::recipe::value::test_support::{
    CHAR, REAL, TIME, WCHAR, ZONE_JSON, primitive_tree, string_tree, wstring_tree, zone_tree,
};
use crate::recipe::value::{Expected, ParameterValue, StoredValue, ValueFault};

/// Read a stored JSON value against a tree.
fn read(json: Value, tree: &TypeTree) -> Result<ParameterValue, ValueFault> {
    ParameterValue::try_from((&StoredValue::from(json), tree))
}

fn zone_json() -> Value {
    serde_json::from_str(ZONE_JSON).unwrap()
}

#[test]
fn output_of_parse_survives_the_stored_form() {
    let tree = zone_tree();
    let parsed = ParameterValue::parse(ZONE_JSON, &tree).unwrap();
    let stored = parsed.to_stored();
    let read_back = ParameterValue::try_from((&stored, &tree)).unwrap();
    assert_eq!(read_back, parsed);
}

#[test]
fn stored_form_holds_the_canonical_json() {
    let parsed = ParameterValue::parse(ZONE_JSON, &zone_tree()).unwrap();
    assert_eq!(parsed.to_stored().as_json(), parsed.as_json());
}

#[test]
fn time_leaf_survives_the_stored_form_as_a_number() {
    let tree = primitive_tree(TIME);
    let parsed = ParameterValue::parse(r#""T#90m""#, &tree).unwrap();
    let stored = parsed.to_stored();
    assert_eq!(stored.as_json(), &json!(5_400_000));
    assert_eq!(ParameterValue::try_from((&stored, &tree)).unwrap(), parsed);
}

#[test]
fn stored_iec_literal_for_a_time_leaf_gives_a_fault() {
    let fault = read(json!("T#90m"), &primitive_tree(TIME)).unwrap_err();
    assert_eq!(fault.leaf().to_string(), "");
    assert_eq!(
        fault.expected(),
        Some(&Expected::Count {
            type_name: Primitive::Time,
        })
    );
}

#[test]
fn stored_count_outside_the_type_range_gives_a_fault() {
    assert!(read(json!(-1000), &primitive_tree(TIME)).is_err());
}

#[test]
fn stored_count_with_a_fraction_gives_a_fault() {
    let value: Value = serde_json::from_str("1.5").unwrap();
    assert!(read(value, &primitive_tree(TIME)).is_err());
}

#[test]
fn data_model_example_as_stored_json_passes() {
    let value = read(zone_json(), &zone_tree()).unwrap();
    assert_eq!(value.as_json(), &zone_json());
}

#[test]
fn stored_value_with_an_extra_key_gives_a_fault() {
    let mut json = zone_json();
    json["extra"] = json!(1);
    let fault = read(json, &zone_tree()).unwrap_err();
    assert_eq!(fault.leaf().to_string(), "");
}

#[test]
fn stored_numbers_are_canonicalized_again() {
    let value: Value = serde_json::from_str("1.5e2").unwrap();
    let read_back = read(value, &primitive_tree(REAL)).unwrap();
    assert_eq!(read_back.as_json().to_string(), "150");
}

#[test]
fn stored_value_of_another_shape_gives_a_fault() {
    assert!(read(json!("x"), &zone_tree()).is_err());
}

/// A stored text with U+0000 gives the `Expected` of its type under both rule sets (ADR-0137, D2).
#[test]
fn stored_text_with_nul_gives_a_fault_under_both_rule_sets() {
    let rows = [
        (
            "CHAR",
            primitive_tree(CHAR),
            "\u{0}",
            Expected::Char { max: 0xFF },
        ),
        (
            "WCHAR",
            primitive_tree(WCHAR),
            "\u{0}",
            Expected::Char { max: 0xFFFF },
        ),
        (
            "STRING(10)",
            string_tree(10),
            "A\u{0}",
            Expected::String { length: 10 },
        ),
        (
            "WSTRING(10)",
            wstring_tree(10),
            "A\u{0}",
            Expected::Wstring { length: 10 },
        ),
    ];
    let mut failures = Vec::new();
    for (name, tree, text, expected) in &rows {
        for rules in [RuleSet::Current, RuleSet::Earlier] {
            let stored = StoredValue::from(json!(text));
            let result = ParameterValue::from_stored(&stored, tree, rules);
            let got = result.as_ref().err().and_then(ValueFault::expected);
            if got != Some(expected) {
                failures.push(format!("{name} under {rules:?}: got {result:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "rows that failed:\n{}",
        failures.join("\n")
    );
}

#[test]
fn stored_value_records_the_root_type_of_the_tree() {
    let tree = zone_tree();
    let value = read(zone_json(), &tree).unwrap();
    let parsed = ParameterValue::parse(ZONE_JSON, &tree).unwrap();
    assert_eq!(value.root_type(), parsed.root_type());
}
