//! Tests of the `Utf8` string encoding on the leaf checks (ADR-0202).

use serde_json::json;

use super::tests::{fault, stored_text};
use crate::plant::fixtures::{PLATFORM, id, machine, primitive_row, wstring_row};
use crate::plant::{LeafPath, PlatformTypes, Primitive, RuleSet, StringEncoding, TypeTree};
use crate::recipe::limit::{LimitInput, ProcessLimit};
use crate::recipe::value::test_support::{CHAR, encoded_string_tree};
use crate::recipe::value::{Expected, LeafFault, ParameterValue, StoredValue, ValueFault};
/// The tree of one `STRING(length)` parameter on a machine with the encoding `Utf8`.
fn utf8_string_tree(length: u16) -> TypeTree {
    encoded_string_tree(length, StringEncoding::Utf8)
}

#[test]
fn adr_0202_utf8_polish_letters_fit_the_byte_length() {
    // "Łódź" has 4 characters and 7 UTF-8 bytes.
    assert_eq!(stored_text(r#""Łódź""#, &utf8_string_tree(10)), r#""Łódź""#);
}

#[test]
fn adr_0202_utf8_scalar_outside_the_basic_plane_fits_four_bytes() {
    assert_eq!(stored_text(r#""😀""#, &utf8_string_tree(4)), r#""😀""#);
}

#[test]
fn adr_0202_utf8_two_byte_scalars_respect_capacity() {
    let tree = utf8_string_tree(4);
    assert_eq!(stored_text(r#""éé""#, &tree), r#""éé""#);
    assert_eq!(
        fault(r#""ééé""#, &tree).expected(),
        Some(&Expected::StringUtf8 { length: 4 })
    );
}

#[test]
fn adr_0202_utf8_too_many_bytes_name_the_length_in_bytes() {
    let fault = fault(r#""Łódź""#, &utf8_string_tree(6));
    assert_eq!(fault.leaf().to_string(), "");
    assert_eq!(fault.expected(), Some(&Expected::StringUtf8 { length: 6 }));
}

#[test]
fn adr_0202_utf8_non_text_value_names_the_length_in_bytes() {
    assert_eq!(
        fault("5", &utf8_string_tree(4)).expected(),
        Some(&Expected::StringUtf8 { length: 4 })
    );
}

#[test]
fn adr_0202_utf8_refuses_u0000() {
    let tree = utf8_string_tree(10);
    for text in [r#""\u0000""#, r#""A\u0000""#, r#""A\u0000B""#] {
        assert_eq!(
            fault(text, &tree).expected(),
            Some(&Expected::StringUtf8 { length: 10 }),
            "{text}"
        );
    }
}

#[test]
fn adr_0202_utf8_accepts_u0001() {
    assert_eq!(
        stored_text(r#""\u0001""#, &utf8_string_tree(1)),
        r#""\u0001""#
    );
}

#[test]
fn adr_0202_utf8_latin1_machine_keeps_the_latin_1_rule() {
    let tree = encoded_string_tree(10, StringEncoding::Latin1);
    assert_eq!(
        fault(r#""Łódź""#, &tree).expected(),
        Some(&Expected::String { length: 10 })
    );
    assert_eq!(stored_text(r#""éééé""#, &tree), r#""éééé""#);
}

/// The tree of the root `root` on a `Utf8` machine over the rows `rows`.
fn utf8_tree(rows: Vec<crate::plant::PlcType>, root: u128) -> TypeTree {
    let types = PlatformTypes::collect(machine(), PLATFORM, StringEncoding::Utf8, rows).unwrap();
    TypeTree::resolve(id(root), &types).unwrap()
}

#[test]
fn adr_0202_utf8_keeps_the_rule_of_a_wstring() {
    // The test platform is TwinCAT. A `WSTRING` counts UTF-16 units and stops at U+FFFF.
    let tree = utf8_tree(vec![wstring_row(50, 2)], 50);
    assert_eq!(stored_text(r#""łó""#, &tree), r#""łó""#);
    assert_eq!(
        fault(r#""😀""#, &tree).expected(),
        Some(&Expected::WstringIec { length: 2 })
    );
}

#[test]
fn adr_0202_utf8_keeps_the_rule_of_a_char() {
    let tree = utf8_tree(vec![primitive_row(CHAR, Primitive::Char)], CHAR);
    assert_eq!(stored_text(r#""é""#, &tree), r#""é""#);
    assert!(ParameterValue::parse(r#""ł""#, &tree).is_err());
}

#[test]
fn adr_0202_utf8_max_length_counts_bytes() {
    let tree = utf8_string_tree(10);
    let limits = [string_limit(&tree, 3)];
    let value = ParameterValue::parse(r#""éé""#, &tree).unwrap();
    let fault = value.check(&tree, &limits).unwrap_err();
    assert_eq!(
        fault,
        ValueFault::OutOfLimit {
            leaf: LeafPath::parse("").unwrap(),
            value: "4".into(),
            min: "0".into(),
            max: "3".into(),
        }
    );
}

#[test]
fn adr_0202_utf8_max_length_accepts_the_exact_byte_count() {
    let tree = utf8_string_tree(10);
    let limits = [string_limit(&tree, 4)];
    let value = ParameterValue::parse(r#""éé""#, &tree).unwrap();
    assert_eq!(value.check(&tree, &limits), Ok(()));
}

#[test]
fn adr_0202_utf8_max_length_on_a_latin1_machine_counts_characters() {
    let tree = encoded_string_tree(10, StringEncoding::Latin1);
    let limits = [string_limit(&tree, 2)];
    let value = ParameterValue::parse(r#""éé""#, &tree).unwrap();
    assert_eq!(value.check(&tree, &limits), Ok(()));
}

#[test]
fn adr_0202_utf8_stored_value_uses_the_byte_rule() {
    let tree = utf8_string_tree(6);
    let stored = StoredValue::from(json!("Łódź"));
    let fault = ParameterValue::from_stored(&stored, &tree, RuleSet::Current).unwrap_err();
    assert_eq!(fault.expected(), Some(&Expected::StringUtf8 { length: 6 }));
    let fits = StoredValue::from(json!("Łód"));
    assert!(ParameterValue::from_stored(&fits, &tree, RuleSet::Current).is_ok());
}

/// A limit of `max_length` on the single leaf of a `STRING` tree.
fn string_limit(tree: &TypeTree, max_length: u16) -> ProcessLimit {
    let row = LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: None,
        max_length: Some(max_length),
    };
    ProcessLimit::parse(&row, tree).unwrap()
}

#[test]
fn adr_0202_utf8_expected_form_names_bytes() {
    assert_eq!(
        Expected::StringUtf8 { length: 6 }.to_string(),
        "a string of at most 6 UTF-8 bytes without U+0000"
    );
}

#[test]
fn adr_0202_utf8_request_leaf_and_text_count_bytes() {
    use crate::recipe::value::{request_leaf, request_text};
    let tree = utf8_string_tree(7);
    let on_utf8 = (PLATFORM, StringEncoding::Utf8);
    let on_latin1 = (PLATFORM, StringEncoding::Latin1);
    assert_eq!(
        request_leaf(&json!("Łódź"), tree.root(), on_utf8),
        Ok(json!("Łódź"))
    );
    assert_eq!(
        request_text("Łódź", tree.root(), on_utf8),
        Ok(json!("Łódź"))
    );
    let short = utf8_string_tree(6);
    for result in [
        request_leaf(&json!("Łódź"), short.root(), on_utf8),
        request_text("Łódź", short.root(), on_utf8),
    ] {
        assert_eq!(
            result.unwrap_err(),
            LeafFault::from(Expected::StringUtf8 { length: 6 })
        );
    }
    assert_eq!(
        request_text("Łódź", tree.root(), on_latin1).unwrap_err(),
        LeafFault::from(Expected::String { length: 7 })
    );
}
