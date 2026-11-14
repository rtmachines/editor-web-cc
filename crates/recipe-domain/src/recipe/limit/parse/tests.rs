use rust_decimal::Decimal;

use crate::plant::fixtures::id;
use crate::plant::{LeafPath, PlcPlatform, Primitive, RuleSet, TypeTree};
use crate::recipe::PrecisionField;
use crate::recipe::limit::fault::LimitFaults;
use crate::recipe::limit::{LimitFault, LimitInput, ProcessLimit, StoredLimit};
use crate::recipe::value::test_support::{
    BOOL, BOUNDARIES, CHAR, INT, LREAL, REAL, ST_ZONE, TIME, integer_tree, primitive_tree,
    time_tree_on, zone_tree,
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
fn integer_types_accept_their_ends_and_refuse_the_values_next_to_them() {
    for row in &BOUNDARIES {
        let tree = integer_tree(row.primitive);
        let name = row.primitive;
        let ends = parse(&range("", Some(row.low), Some(row.high)), &tree).unwrap();
        assert_eq!(ends.min(), row.low.parse().ok(), "{name} min");
        assert_eq!(ends.max(), row.high.parse().ok(), "{name} max");
        let below = parse(&range("", Some(row.below_low), None), &tree).unwrap_err();
        let above = parse(&range("", None, Some(row.above_high)), &tree).unwrap_err();
        let refused = LimitFault::OutsideTypeRange {
            leaf: LeafPath::whole(),
        };
        assert_eq!(below, refused, "{name} min-1");
        assert_eq!(above, refused, "{name} max+1");
    }
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
    let limit = parse(&range("", Some("-100.0001"), Some("1234.567")), &tree).unwrap();
    assert_eq!(limit.min(), Some("-100.0001".parse().unwrap()));
    assert_eq!(limit.max(), Some("1234.567".parse().unwrap()));
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
    let limit = parse(&range("", None, Some("0.123456789012345")), &tree).unwrap();
    assert_eq!(limit.min(), None);
    assert_eq!(limit.max(), Some("0.123456789012345".parse().unwrap()));
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

#[test]
fn real_bound_with_an_exact_decimal_value_gives_the_bound() {
    let tree = primitive_tree(REAL);
    let zeros = format!("1.{}", "0".repeat(29));
    let rows: [(&str, &str); 5] = [
        ("150e-29", "0.0000000000000000000000000015"),
        ("10e-29", "0.0000000000000000000000000001"),
        ("100e-30", "0.0000000000000000000000000001"),
        (zeros.as_str(), "1"),
        ("0e99999999999999999999", "0"),
    ];
    for (text, expected) in rows {
        let limit = parse(&range("", None, Some(text)), &tree).unwrap();
        // `Decimal` equality ignores the scale, so compare the canonical text.
        let canonical_text = limit.max().map(|bound| bound.to_string());
        assert_eq!(canonical_text.as_deref(), Some(expected), "bound {text}");
    }
}

#[test]
fn real_bound_beyond_the_scale_or_range_of_a_decimal_is_outside_the_type_range() {
    let tree = primitive_tree(REAL);
    for text in ["1e-29", "1e29"] {
        let refused = LimitFault::OutsideTypeRange {
            leaf: LeafPath::whole(),
        };
        let min = parse(&range("", Some(text), None), &tree).unwrap_err();
        let max = parse(&range("", None, Some(text)), &tree).unwrap_err();
        assert_eq!(min, refused, "min {text}");
        assert_eq!(max, refused, "max {text}");
    }
}

const S5TIME_STEP_BREAKER: i64 = 15;
const LDATE_PART_OF_DAY: i64 = 1;
const S5TIME_VALID_MAX: i64 = 10_000;
const LDATE_VALID_MAX: i64 = 86_400_000_000_000;

#[test]
fn stored_s5time_max_between_two_steps_is_outside_the_type_range_in_both_sets() {
    let tree = time_tree_on(PlcPlatform::S7_1500, Primitive::S5Time);
    let row = stored("", None, Some(S5TIME_STEP_BREAKER), None);
    for rules in [RuleSet::Current, RuleSet::Earlier] {
        let fault = ProcessLimit::from_stored(&row, &tree, rules).unwrap_err();
        assert!(
            matches!(fault, LimitFault::OutsideTypeRange { .. }),
            "{rules:?}"
        );
    }
}

#[test]
fn stored_ldate_max_that_is_no_whole_day_is_outside_the_type_range_in_both_sets() {
    let tree = time_tree_on(PlcPlatform::CodesysIec, Primitive::Ldate);
    let row = stored("", None, Some(LDATE_PART_OF_DAY), None);
    for rules in [RuleSet::Current, RuleSet::Earlier] {
        let fault = ProcessLimit::from_stored(&row, &tree, rules).unwrap_err();
        assert!(
            matches!(fault, LimitFault::OutsideTypeRange { .. }),
            "{rules:?}"
        );
    }
}

#[test]
fn stored_s5time_max_on_a_step_gives_a_limit_in_both_sets() {
    let tree = time_tree_on(PlcPlatform::S7_1500, Primitive::S5Time);
    let row = stored("", None, Some(S5TIME_VALID_MAX), None);
    for rules in [RuleSet::Current, RuleSet::Earlier] {
        let limit = ProcessLimit::from_stored(&row, &tree, rules).unwrap();
        assert_eq!(limit.to_stored(), row, "{rules:?}");
    }
}

#[test]
fn stored_ldate_max_on_a_whole_day_gives_a_limit_in_both_sets() {
    let tree = time_tree_on(PlcPlatform::CodesysIec, Primitive::Ldate);
    let row = stored("", None, Some(LDATE_VALID_MAX), None);
    for rules in [RuleSet::Current, RuleSet::Earlier] {
        let limit = ProcessLimit::from_stored(&row, &tree, rules).unwrap();
        assert_eq!(limit.to_stored(), row, "{rules:?}");
    }
}

#[test]
fn stored_date_min_above_max_of_the_earlier_set_writes_literals_in_the_fault() {
    let tree = time_tree_on(PlcPlatform::S7_1500, Primitive::Date);
    let row = stored("", Some(72_800), Some(72_700), None);
    let fault = ProcessLimit::from_stored(&row, &tree, RuleSet::Earlier).unwrap_err();
    assert_eq!(
        fault,
        LimitFault::MinGreaterThanMax {
            leaf: LeafPath::parse("").unwrap(),
            min: "D#2169-04-27".to_owned(),
            max: "D#2169-01-17".to_owned(),
        }
    );
}

/// Return `true` when `text` is a sentence that starts with a capital and ends with a period.
fn is_sentence(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && text.ends_with('.')
}

// Design D9, E-RCP-019, E-RCP-006, E-RCP-018: each fault names the path of its row.
#[test]
fn design_d9_each_limit_fault_names_the_path_of_its_row() {
    let tree = zone_tree();
    let cases = [
        (range(".active", None, Some("1")), ".active"),
        (range(".ramps[*]", None, Some("40000")), ".ramps[*]"),
        (range(".temp_sp", Some("9"), Some("1")), ".temp_sp"),
    ];
    let faults: Vec<LimitFault> = cases
        .iter()
        .map(|(row, _)| parse(row, &tree).unwrap_err())
        .collect();
    assert!(matches!(faults[0], LimitFault::NotApplicable { .. }));
    assert!(matches!(faults[1], LimitFault::OutsideTypeRange { .. }));
    assert!(matches!(faults[2], LimitFault::MinGreaterThanMax { .. }));
    for (fault, (_, path)) in faults.iter().zip(&cases) {
        assert_eq!(fault.leaf().to_string(), *path);
        assert!(fault.to_string().contains(path), "{fault}");
        assert!(is_sentence(&fault.to_string()), "{fault}");
    }
}

// Design D9: the texts of the variants differ, also when the path is the same.
#[test]
fn design_d9_the_texts_of_the_limit_faults_differ_per_variant() {
    let leaf = LeafPath::parse(".x").unwrap();
    let faults = [
        LimitFault::NoLeaf { leaf: leaf.clone() },
        LimitFault::NotApplicable { leaf: leaf.clone() },
        LimitFault::OutsideTypeRange { leaf: leaf.clone() },
        LimitFault::MinGreaterThanMax {
            leaf: leaf.clone(),
            min: "9".into(),
            max: "1".into(),
        },
        precision_fault(leaf),
    ];
    let texts: std::collections::BTreeSet<String> =
        faults.iter().map(ToString::to_string).collect();
    assert_eq!(texts.len(), faults.len());
}

fn precision_fault(leaf: LeafPath) -> LimitFault {
    LimitFault::PrecisionExceeded {
        leaf,
        field: PrecisionField::LimitMin,
        type_name: Primitive::Real,
        max_digits: 7,
    }
}

// ADR-0117 (a), domain-model.md section 2.1: a REAL bound holds 7 significant digits. The text
// of the fault names the leaf, the type, and that count.
#[test]
fn adr_0117_the_text_of_a_precision_fault_names_the_leaf_the_type_and_the_digit_limit() {
    let fault = parse(&range(".temp_sp", Some("1.2345678"), None), &zone_tree()).unwrap_err();
    assert_eq!(fault, precision_fault(LeafPath::parse(".temp_sp").unwrap()));
    let text = fault.to_string();
    assert!(text.contains(".temp_sp"), "{text}");
    assert!(text.contains(&Primitive::Real.to_string()), "{text}");
    assert!(text.contains("at most 7 digits"), "{text}");
    assert!(is_sentence(&text), "{text}");
}

// Reliability.md section 4.4: a numeric leaf holds at most 28 significant digits. A stored bound
// of 29 digits is outside the type range in the earlier set, because `NUMERIC` has no limit.
#[test]
fn reliability_4_4_a_stored_bound_of_29_digits_is_outside_the_type_range_in_the_earlier_set() {
    let row = StoredLimit {
        leaf_path: LeafPath::whole(),
        min: Some(Decimal::MAX),
        max: None,
        max_length: None,
    };
    let tree = primitive_tree(LREAL);
    let refused = LimitFault::OutsideTypeRange {
        leaf: LeafPath::whole(),
    };
    assert_eq!(
        stored_fault(&row, &tree, RuleSet::Earlier),
        Some(refused.clone())
    );
    let max_row = StoredLimit {
        min: None,
        max: Some(Decimal::MAX),
        ..row
    };
    assert_eq!(
        stored_fault(&max_row, &tree, RuleSet::Earlier),
        Some(refused)
    );
}

// Design D9: a defensive path. A leaf without a numeric range and without a time type has no
// bound inside its type.
#[test]
fn design_d9_a_bool_leaf_has_no_bound_inside_its_type() {
    use super::is_inside_type;
    for rules in [RuleSet::Current, RuleSet::Earlier] {
        let is_inside = is_inside_type(Primitive::Bool, PlcPlatform::S7_1500, Decimal::ONE, rules);
        assert!(!is_inside, "{rules:?}");
    }
}

// Reliability.md section 4.4: a bound other than a time count is its canonical decimal text. A
// time type with a fraction has no literal.
#[test]
fn reliability_4_4_a_bound_text_of_a_time_type_with_a_fraction_is_its_decimal_text() {
    use crate::recipe::limit::bound_text;
    let platform = PlcPlatform::S7_1500;
    assert_eq!(
        bound_text(Primitive::Time, platform, Decimal::new(15, 1)),
        "1.5"
    );
    assert_eq!(
        bound_text(Primitive::Time, platform, Decimal::from(1500)),
        "T#1s500ms"
    );
    assert_eq!(
        bound_text(Primitive::Int, platform, Decimal::from(1500)),
        "1500"
    );
}
