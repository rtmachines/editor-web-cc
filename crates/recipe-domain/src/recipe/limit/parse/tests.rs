use rust_decimal::Decimal;

use crate::plant::fixtures::id;
use crate::plant::{LeafPath, Primitive, RuleSet, TypeTree};
use crate::recipe::PrecisionField;
use crate::recipe::limit::fault::LimitFaults;
use crate::recipe::limit::{LimitFault, LimitInput, ProcessLimit, StoredLimit};
use crate::recipe::value::test_support::{
    BOOL, CHAR, INT, LREAL, REAL, ST_ZONE, TIME, primitive_tree, zone_tree,
};

fn input(path: &str, min: Option<&str>, max: Option<&str>, max_length: Option<u16>) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: min.map(str::to_owned),
        max: max.map(str::to_owned),
        max_length,
    }
}

fn range(path: &str, min: Option<&str>, max: Option<&str>) -> LimitInput {
    input(path, min, max, None)
}

fn parse(row: &LimitInput, tree: &TypeTree) -> Result<ProcessLimit, LimitFault> {
    ProcessLimit::parse(row, tree)
}

fn stored(path: &str, min: Option<i64>, max: Option<i64>, max_length: Option<u16>) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: min.map(Decimal::from),
        max: max.map(Decimal::from),
        max_length,
    }
}

fn dec(value: i64) -> Option<Decimal> {
    Some(Decimal::from(value))
}

#[test]
fn max_length_on_an_int_leaf_is_not_applicable() {
    let tree = primitive_tree(INT);
    let fault = parse(&input("", None, None, Some(4)), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::NotApplicable { .. }));
}

#[test]
fn min_on_a_bool_leaf_is_not_applicable() {
    let tree = primitive_tree(BOOL);
    let fault = parse(&range("", Some("0"), None), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::NotApplicable { .. }));
}

#[test]
fn max_length_on_a_bool_leaf_is_not_applicable() {
    let tree = primitive_tree(BOOL);
    let fault = parse(&input("", None, None, Some(1)), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::NotApplicable { .. }));
}

#[test]
fn min_on_a_char_leaf_is_not_applicable() {
    let tree = primitive_tree(CHAR);
    let fault = parse(&range("", Some("0"), None), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::NotApplicable { .. }));
}

#[test]
fn min_on_a_string_leaf_is_not_applicable() {
    let fault = parse(&range(".label", Some("1"), None), &zone_tree()).unwrap_err();
    assert!(matches!(fault, LimitFault::NotApplicable { .. }));
}

#[test]
fn row_without_a_field_of_the_class_is_not_applicable() {
    let fault = parse(&input(".label", None, None, None), &zone_tree()).unwrap_err();
    assert!(matches!(fault, LimitFault::NotApplicable { .. }));
    let fault = parse(&input("", None, None, None), &primitive_tree(INT)).unwrap_err();
    assert!(matches!(fault, LimitFault::NotApplicable { .. }));
}

#[test]
fn max_above_the_int_range_is_outside_the_type_range() {
    let tree = primitive_tree(INT);
    let fault = parse(&range("", None, Some("40000")), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn min_below_the_int_range_is_outside_the_type_range() {
    let tree = primitive_tree(INT);
    let fault = parse(&range("", Some("-32769"), None), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn ends_of_the_int_range_pass() {
    let tree = primitive_tree(INT);
    let limit = parse(&range("", Some("-32768"), Some("32767")), &tree).unwrap();
    assert_eq!(limit.min(), dec(-32768));
    assert_eq!(limit.max(), dec(32767));
}

#[test]
fn fraction_for_an_integer_type_is_outside_the_type_range() {
    let tree = primitive_tree(INT);
    let fault = parse(&range("", None, Some("1.5")), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn bound_text_that_is_no_number_is_outside_the_type_range() {
    let tree = primitive_tree(INT);
    let fault = parse(&range("", None, Some("abc")), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn bound_text_with_an_exponent_is_a_decimal() {
    let tree = primitive_tree(INT);
    let limit = parse(&range("", None, Some("1e2")), &tree).unwrap();
    assert_eq!(limit.max(), dec(100));
}

#[test]
fn real_leaf_takes_a_fraction() {
    let zone = zone_tree();
    let limit = parse(&range(".temp_sp", Some("0.5"), Some("120.25")), &zone).unwrap();
    assert_eq!(limit.min(), Some("0.5".parse().unwrap()));
    assert_eq!(limit.max(), Some("120.25".parse().unwrap()));
}

#[test]
fn min_above_max_gives_the_texts_of_both_bounds() {
    let tree = primitive_tree(INT);
    let fault = parse(&range("", Some("10"), Some("5")), &tree).unwrap_err();
    assert_eq!(
        fault,
        LimitFault::MinGreaterThanMax {
            leaf: LeafPath::parse("").unwrap(),
            min: "10".to_owned(),
            max: "5".to_owned(),
        }
    );
}

#[test]
fn min_equal_to_max_passes() {
    let tree = primitive_tree(INT);
    let limit = parse(&range("", Some("5"), Some("5")), &tree).unwrap();
    assert_eq!(limit.min(), limit.max());
}

#[test]
fn only_one_bound_passes() {
    let tree = primitive_tree(INT);
    let limit = parse(&range("", None, Some("5")), &tree).unwrap();
    assert_eq!((limit.min(), limit.max()), (None, dec(5)));
}

#[test]
fn max_length_zero_is_outside_the_type_range() {
    let fault = parse(&input(".label", None, None, Some(0)), &zone_tree()).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn max_length_above_the_string_length_is_outside_the_type_range() {
    let fault = parse(&input(".label", None, None, Some(17)), &zone_tree()).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn max_length_inside_the_string_length_passes() {
    for length in [1_u16, 12, 16] {
        let limit = parse(&input(".label", None, None, Some(length)), &zone_tree()).unwrap();
        assert_eq!(limit.max_length(), Some(length));
        assert_eq!((limit.min(), limit.max()), (None, None));
    }
}

#[test]
fn time_bound_is_stored_as_the_count() {
    let tree = primitive_tree(TIME);
    let limit = parse(&range("", None, Some("T#5s")), &tree).unwrap();
    assert_eq!(limit.max(), dec(5000));
    assert_eq!(limit.min(), None);
}

#[test]
fn time_bounds_in_the_wrong_order_show_their_canonical_literals() {
    let tree = primitive_tree(TIME);
    let fault = parse(&range("", Some("T#2s"), Some("T#1s")), &tree).unwrap_err();
    assert_eq!(
        fault,
        LimitFault::MinGreaterThanMax {
            leaf: LeafPath::parse("").unwrap(),
            min: "T#2s".to_owned(),
            max: "T#1s".to_owned(),
        }
    );
}

#[test]
fn time_bound_of_another_type_or_without_prefix_is_outside_the_type_range() {
    let tree = primitive_tree(TIME);
    for text in ["LT#5s", "5000", "T#-1x"] {
        let fault = parse(&range("", None, Some(text)), &tree).unwrap_err();
        assert!(
            matches!(fault, LimitFault::OutsideTypeRange { .. }),
            "{text}"
        );
    }
}

#[test]
fn path_without_a_leaf_is_no_leaf() {
    let zone = zone_tree();
    for path in [".setpoint", ".ramps[4]", ".ramps", ""] {
        let fault = parse(&range(path, None, Some("5")), &zone).unwrap_err();
        assert!(matches!(fault, LimitFault::NoLeaf { .. }), "{path}");
    }
}

#[test]
fn leaf_check_runs_before_the_class_check() {
    let fault = parse(&input(".setpoint", None, None, Some(3)), &zone_tree()).unwrap_err();
    assert!(matches!(fault, LimitFault::NoLeaf { .. }));
}

#[test]
fn all_elements_row_passes() {
    let limit = parse(&range(".ramps[*]", Some("0"), Some("100")), &zone_tree()).unwrap();
    assert_eq!(limit.leaf_path().to_string(), ".ramps[*]");
    assert_eq!((limit.min(), limit.max()), (dec(0), dec(100)));
}

#[test]
fn limit_records_the_root_type_of_the_tree() {
    let limit = parse(&range(".temp_sp", None, Some("5")), &zone_tree()).unwrap();
    assert_eq!(limit.root_type(), id(ST_ZONE));
}

#[test]
fn fault_names_its_leaf_and_shows_it_in_the_message() {
    let fault = parse(&range(".setpoint", None, Some("5")), &zone_tree()).unwrap_err();
    assert_eq!(fault.leaf().to_string(), ".setpoint");
    assert!(fault.to_string().contains(".setpoint"));
    let fault = parse(&range("", Some("9"), Some("1")), &primitive_tree(INT)).unwrap_err();
    assert!(fault.to_string().contains('9'));
    assert!(fault.to_string().contains('1'));
}

#[test]
fn stored_form_round_trips() {
    let zone = zone_tree();
    let rows = [
        range(".temp_sp", Some("0.5"), Some("120")),
        range(".ramps[*]", None, Some("100")),
        input(".label", None, None, Some(12)),
    ];
    for row in rows {
        let limit = parse(&row, &zone).unwrap();
        let back = ProcessLimit::try_from((&limit.to_stored(), &zone)).unwrap();
        assert_eq!(back, limit);
    }
}

#[test]
fn stored_form_of_a_time_limit_holds_the_count() {
    let tree = primitive_tree(TIME);
    let limit = parse(&range("", Some("T#1s"), Some("T#5s")), &tree).unwrap();
    let row = limit.to_stored();
    assert_eq!(
        (row.min, row.max, row.max_length),
        (dec(1000), dec(5000), None)
    );
    assert_eq!(ProcessLimit::try_from((&row, &tree)).unwrap(), limit);
}

#[test]
fn stored_row_with_a_bound_outside_the_type_range_is_rejected() {
    let int = primitive_tree(INT);
    let fault = ProcessLimit::try_from((&stored("", None, Some(40000), None), &int)).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
    let time = primitive_tree(TIME);
    let too_big = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some("99999999999999".parse().unwrap()),
        max_length: None,
    };
    let fault = ProcessLimit::try_from((&too_big, &time)).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
    let fraction = StoredLimit {
        max: Some("1.5".parse().unwrap()),
        ..too_big
    };
    let fault = ProcessLimit::try_from((&fraction, &time)).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn stored_row_with_a_wrong_field_or_path_is_rejected() {
    let int = primitive_tree(INT);
    let fault = ProcessLimit::try_from((&stored("", None, None, Some(3)), &int)).unwrap_err();
    assert!(matches!(fault, LimitFault::NotApplicable { .. }));
    let fault = ProcessLimit::try_from((&stored(".x", None, Some(3), None), &int)).unwrap_err();
    assert!(matches!(fault, LimitFault::NoLeaf { .. }));
    let fault = ProcessLimit::try_from((&stored("", Some(9), Some(1), None), &int)).unwrap_err();
    assert!(matches!(fault, LimitFault::MinGreaterThanMax { .. }));
}

#[test]
fn bound_text_with_an_underscore_or_a_plus_sign_is_outside_the_type_range() {
    let tree = primitive_tree(INT);
    for text in ["1_000", "+5"] {
        let fault = parse(&range("", None, Some(text)), &tree).unwrap_err();
        assert!(
            matches!(fault, LimitFault::OutsideTypeRange { .. }),
            "{text}"
        );
    }
}

/// A stored row with decimal bounds on the whole value.
fn stored_decimal(min: Option<&str>, max: Option<&str>) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: min.map(|text| text.parse().unwrap()),
        max: max.map(|text| text.parse().unwrap()),
        max_length: None,
    }
}

/// The fault that `from_stored` gives, or `None` when the row passes.
fn stored_fault(row: &StoredLimit, tree: &TypeTree, rules: RuleSet) -> Option<LimitFault> {
    ProcessLimit::from_stored(row, tree, rules).err()
}

fn precision(primitive: Primitive, max_digits: u8, field: PrecisionField) -> LimitFault {
    LimitFault::PrecisionExceeded {
        leaf: LeafPath::parse("").unwrap(),
        field,
        type_name: primitive,
        max_digits,
    }
}

#[test]
fn adr_0117_a_request_real_max_with_8_digits_gives_a_precision_fault() {
    let tree = primitive_tree(REAL);
    let fault = parse(&range("", None, Some("100.00001")), &tree).unwrap_err();
    assert_eq!(
        fault,
        precision(Primitive::Real, 7, PrecisionField::LimitMax)
    );
}

#[test]
fn adr_0117_a_request_real_min_with_8_digits_gives_a_precision_fault() {
    let tree = primitive_tree(REAL);
    let fault = parse(&range("", Some("-100.00001"), None), &tree).unwrap_err();
    assert_eq!(
        fault,
        precision(Primitive::Real, 7, PrecisionField::LimitMin)
    );
}

/// The faults that `parse_all` gives, or an empty list when the row passes.
fn all_faults(row: &LimitInput, tree: &TypeTree) -> Vec<LimitFault> {
    ProcessLimit::parse_all(row, tree)
        .err()
        .map(LimitFaults::into_vec)
        .unwrap_or_default()
}

#[test]
fn adr_0117_a_request_real_row_with_8_digits_in_both_bounds_gives_two_precision_faults() {
    let tree = primitive_tree(REAL);
    let row = range("", Some("-100.00001"), Some("100.00001"));
    assert_eq!(
        all_faults(&row, &tree),
        [
            precision(Primitive::Real, 7, PrecisionField::LimitMin),
            precision(Primitive::Real, 7, PrecisionField::LimitMax),
        ]
    );
    assert_eq!(
        parse(&row, &tree).unwrap_err(),
        precision(Primitive::Real, 7, PrecisionField::LimitMin)
    );
}

#[test]
fn adr_0117_a_request_lreal_row_with_16_digits_in_both_bounds_gives_two_precision_faults() {
    let tree = primitive_tree(LREAL);
    let row = range("", Some("-0.1234567890123456"), Some("0.1234567890123456"));
    assert_eq!(
        all_faults(&row, &tree),
        [
            precision(Primitive::Lreal, 15, PrecisionField::LimitMin),
            precision(Primitive::Lreal, 15, PrecisionField::LimitMax),
        ]
    );
}

#[test]
fn adr_0117_a_request_row_with_one_bad_bound_gives_one_precision_fault() {
    let tree = primitive_tree(REAL);
    let bad_min = range("", Some("-100.00001"), Some("100.0001"));
    assert_eq!(
        all_faults(&bad_min, &tree),
        [precision(Primitive::Real, 7, PrecisionField::LimitMin)]
    );
    let bad_max = range("", Some("-100.0001"), Some("100.00001"));
    assert_eq!(
        all_faults(&bad_max, &tree),
        [precision(Primitive::Real, 7, PrecisionField::LimitMax)]
    );
}

#[test]
fn adr_0117_a_bound_outside_the_type_range_hides_the_precision_faults() {
    let tree = primitive_tree(REAL);
    let row = range("", Some("-1.00000001e40"), Some("100.00001"));
    let faults = all_faults(&row, &tree);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(matches!(faults[0], LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn adr_0117_a_request_real_bound_with_7_digits_passes() {
    let tree = primitive_tree(REAL);
    let limit = parse(&range("", Some("-100.0001"), Some("1234.567")), &tree);
    assert!(limit.is_ok());
}

#[test]
fn adr_0117_a_request_lreal_bound_with_16_digits_gives_a_precision_fault() {
    let tree = primitive_tree(LREAL);
    for (row, field) in [
        (
            range("", None, Some("0.1234567890123456")),
            PrecisionField::LimitMax,
        ),
        (
            range("", Some("-0.1234567890123456"), None),
            PrecisionField::LimitMin,
        ),
    ] {
        let fault = parse(&row, &tree).unwrap_err();
        assert_eq!(fault, precision(Primitive::Lreal, 15, field));
    }
}

#[test]
fn adr_0117_a_request_lreal_bound_with_15_digits_passes() {
    let tree = primitive_tree(LREAL);
    let limit = parse(&range("", None, Some("0.123456789012345")), &tree);
    assert!(limit.is_ok());
}

#[test]
fn adr_0117_a_bound_outside_the_type_range_stays_outside_type_range() {
    let tree = primitive_tree(REAL);
    let fault = parse(&range("", None, Some("1e40")), &tree).unwrap_err();
    assert!(matches!(fault, LimitFault::OutsideTypeRange { .. }));
}

#[test]
fn adr_0117_a_stored_real_min_with_8_digits_fails_in_the_current_set() {
    let tree = primitive_tree(REAL);
    let row = stored_decimal(Some("-100.00001"), None);
    let fault = stored_fault(&row, &tree, RuleSet::Current);
    assert_eq!(
        fault,
        Some(precision(Primitive::Real, 7, PrecisionField::LimitMin))
    );
}

#[test]
fn adr_0117_a_stored_real_max_with_8_digits_fails_in_the_current_set() {
    let tree = primitive_tree(REAL);
    let row = stored_decimal(None, Some("100.00001"));
    let fault = stored_fault(&row, &tree, RuleSet::Current);
    assert_eq!(
        fault,
        Some(precision(Primitive::Real, 7, PrecisionField::LimitMax))
    );
}

#[test]
fn adr_0120_a_stored_real_bound_with_8_digits_passes_in_the_earlier_set() {
    let tree = primitive_tree(REAL);
    let row = stored_decimal(Some("-100.00001"), Some("100.00001"));
    assert_eq!(stored_fault(&row, &tree, RuleSet::Earlier), None);
    assert!(ProcessLimit::try_from((&row, &tree)).is_ok());
}

#[test]
fn adr_0117_a_stored_lreal_bound_with_16_digits_fails_only_in_the_current_set() {
    let tree = primitive_tree(LREAL);
    for (row, field) in [
        (
            stored_decimal(None, Some("0.1234567890123456")),
            PrecisionField::LimitMax,
        ),
        (
            stored_decimal(Some("-0.1234567890123456"), None),
            PrecisionField::LimitMin,
        ),
    ] {
        let fault = stored_fault(&row, &tree, RuleSet::Current);
        assert_eq!(fault, Some(precision(Primitive::Lreal, 15, field)));
        assert_eq!(stored_fault(&row, &tree, RuleSet::Earlier), None);
    }
}

#[test]
fn adr_0117_the_earlier_set_keeps_the_order_check_of_the_bounds() {
    let tree = primitive_tree(REAL);
    let row = stored_decimal(Some("100.00002"), Some("100.00001"));
    let fault = stored_fault(&row, &tree, RuleSet::Earlier);
    assert!(matches!(fault, Some(LimitFault::MinGreaterThanMax { .. })));
}
