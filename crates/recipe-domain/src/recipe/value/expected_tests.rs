//! Tests of `Expected`, the typed form of a shape fault (design D3 of `web-recipe-editor`).

use rust_decimal::Decimal;
use serde_json::json;

use crate::plant::{MemberName, PlcPlatform, Primitive, TypeTree};
use crate::recipe::value::test_support::{
    BOOL, CHAR, INT, REAL, TIME, WCHAR, int_array_tree, primitive_tree, string_tree, wstring_tree,
    zone_tree,
};
use crate::recipe::value::{Expected, ParameterValue, StoredValue, ValueFault, request_leaf};

fn member(name: &str) -> MemberName {
    MemberName::parse(name).unwrap()
}

/// Parse `json` against `tree` and return the typed expectation of the fault.
fn expected_of(json: &str, tree: &TypeTree) -> Expected {
    ParameterValue::parse(json, tree)
        .unwrap_err()
        .expected()
        .cloned()
        .unwrap()
}

fn int_range() -> Expected {
    Expected::NumberRange {
        type_name: Primitive::Int,
        low: Decimal::from(-32_768),
        high: Decimal::from(32_767),
    }
}

#[test]
fn display_of_a_shape_variant_keeps_the_english_phrase() {
    let cases = [
        (Expected::Json, "valid JSON text"),
        (
            Expected::Depth { levels: 32 },
            "a JSON value with at most 32 levels",
        ),
        (
            Expected::Object {
                members: vec![member("temp_sp"), member("active")],
            },
            "an object with the members temp_sp, active",
        ),
        (Expected::Array, "an array"),
        (Expected::ArrayCount { count: 3 }, "an array of 3 elements"),
        (Expected::Index, "an index inside the INTEGER range"),
        (Expected::Scalar, "a scalar value"),
        (
            Expected::ParameterType,
            "a value of the type of the parameter",
        ),
        (Expected::NewValue, "a new value for the new type"),
        (Expected::Leaf, "a value for this field"),
        (Expected::KnownLeaf, "a path of the type of the parameter"),
    ];
    for (expected, text) in cases {
        assert_eq!(expected.to_string(), text, "{expected:?}");
    }
}

#[test]
fn display_of_a_text_variant_keeps_the_english_phrase() {
    let cases = [
        (Expected::Bool, "true or false"),
        (
            Expected::Decimal,
            "a decimal number with a dot, for example 1.5",
        ),
        (
            Expected::Wstring { length: 2 },
            "a string of at most 2 UTF-16 units",
        ),
        (
            Expected::String { length: 10 },
            "a string of at most 10 characters from Latin-1",
        ),
        (
            Expected::Char { max: 0xFF },
            "one character from U+0000 to U+00FF",
        ),
        (
            Expected::Char { max: 0xFFFF },
            "one character from U+0000 to U+FFFF",
        ),
    ];
    for (expected, text) in cases {
        assert_eq!(expected.to_string(), text, "{expected:?}");
    }
}

#[test]
fn display_of_a_number_variant_keeps_the_english_phrase() {
    let cases = [
        (
            Expected::Literal {
                type_name: Primitive::Time,
            },
            "an IEC literal of type TIME".to_owned(),
        ),
        (
            Expected::Count {
                type_name: Primitive::Time,
            },
            "a count of type TIME".to_owned(),
        ),
        (
            Expected::Integer {
                type_name: Primitive::Int,
            },
            "an integer of type INT".to_owned(),
        ),
        (
            int_range(),
            "a number of type INT from -32768 to 32767".to_owned(),
        ),
        (
            Expected::NumberDigits {
                type_name: Primitive::Real,
                digits: 28,
            },
            "a number of type REAL with at most 28 significant digits".to_owned(),
        ),
    ];
    for (expected, text) in cases {
        assert_eq!(expected.to_string(), text, "{expected:?}");
    }
}

#[test]
fn shape_faults_of_parse_carry_typed_expectations() {
    assert_eq!(expected_of("{not json", &zone_tree()), Expected::Json);
    let deep = format!("{}1{}", "[".repeat(33), "]".repeat(33));
    assert_eq!(
        expected_of(&deep, &primitive_tree(INT)),
        Expected::Depth { levels: 32 }
    );
    let missing = r#"{"temp_sp": 1, "active": true, "ramps": [1, 2, 3]}"#;
    assert_eq!(
        expected_of(missing, &zone_tree()),
        Expected::Object {
            members: vec![
                member("temp_sp"),
                member("active"),
                member("ramps"),
                member("label")
            ],
        }
    );
    let short = r#"{"temp_sp": 1, "active": true, "ramps": [1, 2], "label": ""}"#;
    assert_eq!(
        expected_of(short, &zone_tree()),
        Expected::ArrayCount { count: 3 }
    );
    let tree = int_array_tree(&[(0, 1), (1, 2)]);
    assert_eq!(
        expected_of("[1,2,3,4]", &tree),
        Expected::ArrayCount { count: 2 }
    );
}

#[test]
fn numeric_faults_of_parse_carry_typed_expectations() {
    assert_eq!(expected_of(r#""7""#, &primitive_tree(INT)), int_range());
    assert_eq!(
        expected_of("1.5", &primitive_tree(INT)),
        Expected::Integer {
            type_name: Primitive::Int,
        }
    );
    assert_eq!(
        expected_of("1.2345678901234567890123456789", &primitive_tree(REAL)),
        Expected::NumberDigits {
            type_name: Primitive::Real,
            digits: 28,
        }
    );
}

#[test]
fn text_faults_of_parse_carry_typed_expectations() {
    assert_eq!(expected_of("1", &primitive_tree(BOOL)), Expected::Bool);
    assert_eq!(
        expected_of(r#""Łódź""#, &string_tree(10)),
        Expected::String { length: 10 }
    );
    assert_eq!(
        expected_of(r#""a😀""#, &wstring_tree(2)),
        Expected::Wstring { length: 2 }
    );
    assert_eq!(
        expected_of(r#""ab""#, &primitive_tree(CHAR)),
        Expected::Char { max: 0xFF }
    );
    assert_eq!(
        expected_of(r#""😀""#, &primitive_tree(WCHAR)),
        Expected::Char { max: 0xFFFF }
    );
    assert_eq!(
        expected_of(r#""T#2h1d""#, &primitive_tree(TIME)),
        Expected::Literal {
            type_name: Primitive::Time,
        }
    );
}

#[test]
fn a_stored_literal_for_a_time_leaf_expects_the_count_of_its_type() {
    let stored = StoredValue::from(json!("T#90m"));
    let fault = ParameterValue::try_from((&stored, &primitive_tree(TIME))).unwrap_err();
    assert_eq!(
        fault.expected(),
        Some(&Expected::Count {
            type_name: Primitive::Time,
        })
    );
}

#[test]
fn a_request_leaf_at_a_struct_node_expects_a_scalar() {
    let tree = zone_tree();
    let result = request_leaf(&json!(1), tree.root(), PlcPlatform::TwinCat3);
    assert_eq!(result, Err(Expected::Scalar));
}

#[test]
fn a_fault_of_the_wrong_tree_expects_the_parameter_type() {
    assert_eq!(
        ValueFault::WrongTree.expected(),
        Some(&Expected::ParameterType)
    );
}

#[test]
fn a_limit_fault_has_no_expectation() {
    let leaf = crate::plant::LeafPath::whole();
    assert_eq!(ValueFault::LimitConflict { leaf }.expected(), None);
}
