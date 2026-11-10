use crate::plant::{LeafPath, TypeTree};
use crate::recipe::limit::{LimitInput, ProcessLimit, StoredLimit};
use crate::recipe::value::test_support::{
    INT, REAL, TIME, ZONE_JSON, int_array_tree, primitive_tree, string_tree, wstring_tree,
    zone_tree,
};
use crate::recipe::value::{LeafTexts, ParameterValue, ValueFault, ValueFaults};

fn limit(
    tree: &TypeTree,
    path: &str,
    bounds: (Option<&str>, Option<&str>),
    max_length: Option<u16>,
) -> ProcessLimit {
    let row = LimitInput {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: bounds.0.map(str::to_owned),
        max: bounds.1.map(str::to_owned),
        max_length,
    };
    ProcessLimit::parse(&row, tree).unwrap()
}

fn range(tree: &TypeTree, path: &str, min: Option<&str>, max: Option<&str>) -> ProcessLimit {
    limit(tree, path, (min, max), None)
}

fn value(json: &str, tree: &TypeTree) -> ParameterValue {
    ParameterValue::parse(json, tree).unwrap()
}

/// The three limits of `data-model.md` section 9.1.
fn zone_limits(tree: &TypeTree) -> Vec<ProcessLimit> {
    vec![
        range(tree, ".temp_sp", Some("0"), Some("120")),
        range(tree, ".ramps[*]", Some("0"), Some("100")),
        limit(tree, ".label", (None, None), Some(16)),
    ]
}

/// Return the fields of an `OutOfLimit` fault, or fail the test.
fn out_of_limit(fault: ValueFault) -> (String, String, String, String) {
    let ValueFault::OutOfLimit {
        leaf,
        value,
        min,
        max,
    } = fault
    else {
        return (
            format!("not OutOfLimit: {fault:?}"),
            String::new(),
            String::new(),
            String::new(),
        );
    };
    (leaf.to_string(), value, min, max)
}

fn texts(leaf: &str, found: &str, min: &str, max: &str) -> (String, String, String, String) {
    (leaf.into(), found.into(), min.into(), max.into())
}

#[test]
fn zone_value_with_its_three_limits_passes() {
    let tree = zone_tree();
    let parsed = value(ZONE_JSON, &tree);
    assert_eq!(parsed.check(&tree, &zone_limits(&tree)), Ok(()));
}

#[test]
fn value_without_limits_passes() {
    let tree = zone_tree();
    assert_eq!(value(ZONE_JSON, &tree).check(&tree, &[]), Ok(()));
}

#[test]
fn bounds_are_inclusive() {
    let tree = primitive_tree(INT);
    let limits = [range(&tree, "", Some("5"), Some("5"))];
    assert_eq!(value("5", &tree).check(&tree, &limits), Ok(()));
    assert!(value("6", &tree).check(&tree, &limits).is_err());
    assert!(value("4", &tree).check(&tree, &limits).is_err());
}

#[test]
fn all_elements_limit_names_the_element_above_the_bound() {
    let tree = zone_tree();
    let json = r#"{"temp_sp": 85.5, "active": true, "ramps": [10, 150, 30], "label": "Zone A"}"#;
    let fault = value(json, &tree)
        .check(&tree, &zone_limits(&tree))
        .unwrap_err();
    assert_eq!(out_of_limit(fault), texts(".ramps[2]", "150", "0", "100"));
}

#[test]
fn value_below_the_lower_bound_fails() {
    let tree = zone_tree();
    let json = r#"{"temp_sp": -0.5, "active": true, "ramps": [10, 20, 30], "label": "Zone A"}"#;
    let fault = value(json, &tree)
        .check(&tree, &zone_limits(&tree))
        .unwrap_err();
    assert_eq!(out_of_limit(fault), texts(".temp_sp", "-0.5", "0", "120"));
}

#[test]
fn faults_come_in_the_order_of_the_leaves() {
    let tree = zone_tree();
    let json = r#"{"temp_sp": 500, "active": true, "ramps": [10, 150, 30], "label": "Zone A"}"#;
    let fault = value(json, &tree)
        .check(&tree, &zone_limits(&tree))
        .unwrap_err();
    assert_eq!(fault.leaf().to_string(), ".temp_sp");
}

#[test]
fn string_longer_than_max_length_fails_with_its_length() {
    let tree = zone_tree();
    let json = r#"{"temp_sp": 1, "active": true, "ramps": [1, 2, 3], "label": "Zone A long"}"#;
    let limits = [limit(&tree, ".label", (None, None), Some(6))];
    let fault = value(json, &tree).check(&tree, &limits).unwrap_err();
    assert_eq!(out_of_limit(fault), texts(".label", "11", "0", "6"));
}

#[test]
fn string_as_long_as_max_length_passes() {
    let tree = string_tree(16);
    let limits = [limit(&tree, "", (None, None), Some(6))];
    assert_eq!(value(r#""ABCDEF""#, &tree).check(&tree, &limits), Ok(()));
}

#[test]
fn string_max_length_counts_characters_and_not_bytes() {
    let tree = string_tree(4);
    let limits = [limit(&tree, "", (None, None), Some(2))];
    assert_eq!(
        value("\"\u{e9}\u{e9}\"", &tree).check(&tree, &limits),
        Ok(())
    );
}

#[test]
fn wstring_length_counts_utf16_units() {
    let tree = wstring_tree(10);
    let limits = [limit(&tree, "", (None, None), Some(2))];
    let fault = value(r#""a😀""#, &tree).check(&tree, &limits).unwrap_err();
    assert_eq!(out_of_limit(fault), texts("", "3", "0", "2"));
}

#[test]
fn time_value_and_bounds_show_as_canonical_literals() {
    let tree = primitive_tree(TIME);
    let limits = [range(&tree, "", None, Some("T#1h"))];
    let fault = value(r#""T#90m""#, &tree)
        .check(&tree, &limits)
        .unwrap_err();
    assert_eq!(out_of_limit(fault), texts("", "T#1h30m", "T#0ms", "T#1h"));
}

#[test]
fn time_value_inside_its_limits_passes() {
    let tree = primitive_tree(TIME);
    let limits = [range(&tree, "", Some("T#1s"), Some("T#1h"))];
    assert_eq!(value(r#""T#59m""#, &tree).check(&tree, &limits), Ok(()));
}

#[test]
fn missing_lower_bound_shows_the_type_minimum() {
    let tree = primitive_tree(INT);
    let limits = [range(&tree, "", None, Some("100"))];
    let fault = value("150", &tree).check(&tree, &limits).unwrap_err();
    assert_eq!(out_of_limit(fault), texts("", "150", "-32768", "100"));
}

#[test]
fn missing_upper_bound_shows_the_type_maximum() {
    let tree = primitive_tree(INT);
    let limits = [range(&tree, "", Some("10"), None)];
    let fault = value("5", &tree).check(&tree, &limits).unwrap_err();
    assert_eq!(out_of_limit(fault), texts("", "5", "10", "32767"));
}

#[test]
fn two_dimensional_all_elements_limit_runs_in_row_major_order() {
    let tree = int_array_tree(&[(0, 1), (1, 2)]);
    let limits = [range(&tree, "[*]", None, Some("3"))];
    let fault = value("[[1, 2], [3, 4]]", &tree)
        .check(&tree, &limits)
        .unwrap_err();
    assert_eq!(out_of_limit(fault).0, "[1,2]");
}

#[test]
fn value_of_another_tree_gives_wrong_tree() {
    let zone = zone_tree();
    let int = primitive_tree(INT);
    let fault = value(ZONE_JSON, &zone).check(&int, &[]).unwrap_err();
    assert_eq!(fault, ValueFault::WrongTree);
}

#[test]
fn limit_of_another_tree_gives_wrong_tree() {
    let int = primitive_tree(INT);
    let real = primitive_tree(REAL);
    let foreign = [range(&real, "", None, Some("5"))];
    let fault = value("1", &int).check(&int, &foreign).unwrap_err();
    assert_eq!(fault, ValueFault::WrongTree);
}

#[test]
fn two_limits_for_one_path_give_a_conflict() {
    let tree = zone_tree();
    let limits = [
        range(&tree, ".temp_sp", None, Some("5")),
        range(&tree, ".temp_sp", Some("1"), None),
    ];
    let fault = value(ZONE_JSON, &tree).check(&tree, &limits).unwrap_err();
    assert!(matches!(&fault, ValueFault::LimitConflict { leaf } if leaf.to_string() == ".temp_sp"));
}

#[test]
fn all_elements_limit_and_index_limit_give_a_conflict() {
    let tree = zone_tree();
    let limits = [
        range(&tree, ".ramps[*]", None, Some("100")),
        range(&tree, ".ramps[2]", None, Some("100")),
    ];
    let fault = value(ZONE_JSON, &tree).check(&tree, &limits).unwrap_err();
    assert!(
        matches!(&fault, ValueFault::LimitConflict { leaf } if leaf.to_string() == ".ramps[2]")
    );
}

#[test]
fn conflict_is_reported_before_a_value_fault() {
    let tree = primitive_tree(INT);
    let limits = [
        range(&tree, "", None, Some("1")),
        range(&tree, "", None, Some("2")),
    ];
    let fault = value("9", &tree).check(&tree, &limits).unwrap_err();
    assert!(matches!(fault, ValueFault::LimitConflict { .. }));
}

#[test]
fn texts_with_a_bad_leaf_keep_the_conflict_first_and_the_leaves_in_tree_order() {
    let tree = int_array_tree(&[(1, 3)]);
    let limits = [
        range(&tree, "[*]", None, Some("10")),
        range(&tree, "[2]", None, Some("10")),
    ];
    let pairs = [("[1]", "20"), ("[2]", "x"), ("[3]", "30"), ("[9]", "1")]
        .map(|(path, text)| (LeafPath::parse(path).unwrap(), text.to_owned()));
    let texts = LeafTexts::parse(pairs.to_vec()).unwrap();
    let faults = ParameterValue::from_texts_checked(&tree, &texts, &limits).unwrap_err();
    let kinds: Vec<String> = faults
        .iter()
        .map(|fault| match fault {
            ValueFault::LimitConflict { leaf } => format!("conflict {leaf}"),
            ValueFault::OutOfLimit { leaf, .. } => format!("range {leaf}"),
            ValueFault::Mismatch { leaf, .. } => format!("shape {leaf}"),
            ValueFault::Precision { leaf, .. } => format!("precision {leaf}"),
            ValueFault::WrongTree => "wrong tree".to_owned(),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "conflict [2]",
            "range [1]",
            "shape [2]",
            "range [3]",
            "shape [9]"
        ]
    );
}

#[test]
fn limits_on_different_elements_do_not_conflict() {
    let tree = zone_tree();
    let limits = [
        range(&tree, ".ramps[1]", None, Some("100")),
        range(&tree, ".ramps[2]", None, Some("100")),
    ];
    assert_eq!(value(ZONE_JSON, &tree).check(&tree, &limits), Ok(()));
}

#[test]
fn new_faults_have_messages_with_their_paths() {
    let tree = primitive_tree(INT);
    let limits = [range(&tree, "", None, Some("1"))];
    let out = value("9", &tree).check(&tree, &limits).unwrap_err();
    assert!(out.to_string().contains('9'));
    assert!(out.to_string().contains("32768"));
    assert_eq!(out.leaf().to_string(), "");
    assert!(!ValueFault::WrongTree.to_string().is_empty());
    assert_eq!(ValueFault::WrongTree.leaf().to_string(), "");
}

#[test]
fn a_tree_of_another_platform_with_the_same_root_id_gives_wrong_tree() {
    use crate::plant::PlcPlatform;
    use crate::recipe::value::test_support::primitive_tree_on;
    let home = primitive_tree_on(PlcPlatform::TwinCat3, INT);
    let other = primitive_tree_on(PlcPlatform::S7_1500, INT);
    let parsed = value("5", &home);
    assert_eq!(parsed.check(&home, &[]), Ok(()));
    assert_eq!(parsed.check(&other, &[]), Err(ValueFault::WrongTree));
    let limits = [range(&other, "", Some("0"), Some("9"))];
    assert_eq!(parsed.check(&home, &limits), Err(ValueFault::WrongTree));
}

#[test]
fn out_of_limit_on_a_date_writes_the_bounds_of_the_earlier_set_as_literals() {
    use crate::plant::{PlcPlatform, Primitive, RuleSet};
    use crate::recipe::value::StoredValue;
    use crate::recipe::value::test_support::time_tree_on;
    let tree = time_tree_on(PlcPlatform::S7_1500, Primitive::Date);
    let stored_max = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(rust_decimal::Decimal::from(72_690)),
        max_length: None,
    };
    let limits = [ProcessLimit::from_stored(&stored_max, &tree, RuleSet::Earlier).unwrap()];
    let stored = StoredValue::from(serde_json::json!(72_700));
    let value = ParameterValue::from_stored(&stored, &tree, RuleSet::Earlier).unwrap();
    let faults = value
        .check_all_under(&tree, &limits, RuleSet::Earlier)
        .unwrap_err();
    assert_eq!(
        out_of_limit(faults.first().clone()),
        texts("", "D#2169-01-17", "D#1990-01-01", "D#2169-01-07")
    );
}

/// The zone value of `data-model.md` section 9.1 with two faults: `.temp_sp` and `.ramps[2]`.
fn two_fault_list() -> ValueFaults {
    let tree = zone_tree();
    let json = r#"{"temp_sp": 500, "active": true, "ramps": [10, 150, 30], "label": "Zone A"}"#;
    value(json, &tree)
        .check_all(&tree, &zone_limits(&tree))
        .unwrap_err()
}

// ADR-0041: a draft command reports every fault of a value at once.
#[test]
fn adr_0041_a_list_of_two_faults_has_the_length_2_and_is_not_empty() {
    let faults = two_fault_list();
    assert_eq!(faults.len(), 2);
    assert!(!faults.is_empty());
    assert_eq!(faults.iter().count(), 2);
}

// ADR-0041: the list holds at least one fault, so a list from one fault has the length 1.
#[test]
fn adr_0041_a_list_from_one_fault_has_the_length_1_and_is_not_empty() {
    let faults = ValueFaults::from(ValueFault::WrongTree);
    assert_eq!(faults.len(), 1);
    assert!(!faults.is_empty());
}

// ADR-0041 (b): the faults come in the order of the leaves, and the text lists each of them once.
#[test]
fn adr_0041_the_text_of_two_faults_lists_each_fault_once_with_a_separator_between() {
    let faults = two_fault_list();
    let each: Vec<String> = faults.iter().map(ToString::to_string).collect();
    let text = faults.to_string();
    assert!(
        each.iter()
            .all(|fault| text.matches(fault.as_str()).count() == 1)
    );
    assert!(text.starts_with(each[0].as_str()), "{text}");
    assert!(text.ends_with(each[1].as_str()), "{text}");
    assert!(each[0].contains(".temp_sp") && each[1].contains(".ramps[2]"));
    let separator = &text[each[0].len()..text.len() - each[1].len()];
    assert!(!separator.is_empty(), "{text}");
}

// ADR-0041: a list of one fault has no separator.
#[test]
fn adr_0041_the_text_of_one_fault_is_the_text_of_that_fault() {
    let fault = ValueFault::LimitConflict {
        leaf: LeafPath::parse(".temp_sp").unwrap(),
    };
    assert_eq!(
        ValueFaults::from(fault.clone()).to_string(),
        fault.to_string()
    );
}

// Design D9, E-RCP-019: a conflict names the leaf that two limits cover.
#[test]
fn a_limit_conflict_names_its_leaf_in_its_path_and_in_its_text() {
    let tree = zone_tree();
    let limits = [
        range(&tree, ".temp_sp", None, Some("5")),
        range(&tree, ".temp_sp", Some("1"), None),
    ];
    let fault = value(ZONE_JSON, &tree).check(&tree, &limits).unwrap_err();
    assert_eq!(fault.leaf().to_string(), ".temp_sp");
    assert!(fault.to_string().contains(".temp_sp"), "{fault}");
    assert_eq!(fault.expected(), None);
}

// ADR-0118 (b): a fault of a stored value names its tree. A value of another root type or of
// another platform has no leaf list in that tree.
#[test]
fn adr_0118_the_leaf_faults_of_a_tree_of_another_platform_give_wrong_tree() {
    use crate::plant::{PlcPlatform, RuleSet};
    use crate::recipe::value::test_support::primitive_tree_on;
    let home = primitive_tree_on(PlcPlatform::TwinCat3, INT);
    let other = primitive_tree_on(PlcPlatform::S7_1500, INT);
    let parsed = value("5", &home);
    assert_eq!(
        parsed.leaf_faults_under(&home, RuleSet::Current),
        Ok(Vec::new())
    );
    assert_eq!(
        parsed.leaf_faults_under(&other, RuleSet::Current),
        Err(ValueFault::WrongTree)
    );
}

// ADR-0118 (b): the same check for another root type on the same platform.
#[test]
fn adr_0118_the_leaf_faults_of_a_tree_of_another_root_type_give_wrong_tree() {
    use crate::plant::RuleSet;
    let parsed = value("5", &primitive_tree(INT));
    assert_eq!(
        parsed.leaf_faults_under(&primitive_tree(REAL), RuleSet::Current),
        Err(ValueFault::WrongTree)
    );
}

/// The tree of the root type 50 with the one leaf `primitive`.
fn root_50_tree(primitive: crate::plant::Primitive) -> TypeTree {
    use crate::plant::fixtures::primitive_row;
    crate::recipe::value::test_support::tree(vec![primitive_row(50, primitive)], 50)
}

// I7, ADR-0118: a length limit belongs to a string leaf. A limit of another tree with the same
// root id and platform, but a string leaf, meets a number and gives `WrongTree`.
#[test]
fn i7_a_length_limit_on_a_number_leaf_gives_wrong_tree() {
    use crate::plant::Primitive;
    let strings = string_tree(8);
    let numbers = root_50_tree(Primitive::Int);
    let limits = [limit(&strings, "", (None, None), Some(4))];
    let result = value("5", &numbers).check(&numbers, &limits);
    assert_eq!(result, Err(ValueFault::WrongTree));
}

// I7, ADR-0118: a range limit belongs to a number leaf. A limit of another tree with the same
// root id and platform, but an INT leaf, meets a string and gives `WrongTree`.
#[test]
fn i7_a_range_limit_on_a_string_leaf_gives_wrong_tree() {
    use crate::plant::Primitive;
    let numbers = root_50_tree(Primitive::Int);
    let strings = string_tree(8);
    let limits = [range(&numbers, "", None, Some("5"))];
    let result = value(r#""ab""#, &strings).check(&strings, &limits);
    assert_eq!(result, Err(ValueFault::WrongTree));
}

// Design D8: a value of another shape than its tree has no leaf list. The walk gives `None`.
#[test]
fn design_d8_the_leaves_of_a_value_of_another_shape_give_none() {
    use crate::recipe::value::leaves::value_leaves;
    let tree = zone_tree();
    let no_object = serde_json::json!(5);
    let no_array = serde_json::json!({"temp_sp": 1, "active": true, "ramps": 5, "label": "A"});
    let short_array =
        serde_json::json!({"temp_sp": 1, "active": true, "ramps": [1, 2], "label": "A"});
    let no_member = serde_json::json!({"temp_sp": 1, "active": true, "label": "A"});
    for json in [&no_object, &no_array, &short_array, &no_member] {
        assert!(value_leaves(json, &tree).is_none(), "{json}");
    }
    let whole: serde_json::Value = serde_json::from_str(ZONE_JSON).unwrap();
    let paths: Vec<String> = value_leaves(&whole, &tree)
        .unwrap()
        .iter()
        .map(|leaf| leaf.path.to_string())
        .collect();
    assert_eq!(
        paths,
        [
            ".temp_sp",
            ".active",
            ".ramps[1]",
            ".ramps[2]",
            ".ramps[3]",
            ".label"
        ]
    );
}
