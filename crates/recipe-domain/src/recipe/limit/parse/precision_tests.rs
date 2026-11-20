//! Tests of the precision faults and the stored decimal bounds of `ProcessLimit`.

use super::tests::{parse, range};
use crate::plant::{LeafPath, Primitive, RuleSet, TypeTree};
use crate::recipe::PrecisionField;
use crate::recipe::limit::fault::LimitFaults;
use crate::recipe::limit::{LimitFault, LimitInput, ProcessLimit, StoredLimit};
use crate::recipe::value::test_support::{LREAL, REAL, primitive_tree};

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
pub(super) fn stored_fault(
    row: &StoredLimit,
    tree: &TypeTree,
    rules: RuleSet,
) -> Option<LimitFault> {
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
