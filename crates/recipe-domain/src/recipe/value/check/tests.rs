use crate::plant::{LeafPath, TypeTree};
use crate::recipe::limit::{LimitInput, ProcessLimit};
use crate::recipe::value::test_support::{
    INT, REAL, TIME, ZONE_JSON, int_array_tree, primitive_tree, string_tree, wstring_tree,
    zone_tree,
};
use crate::recipe::value::{LeafTexts, ParameterValue, ValueFault};

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
