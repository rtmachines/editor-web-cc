//! Tests of the text and the check of a stored `DATE` under the earlier rule set (design D7).
//!
//! The tests use `S7_1500`. `S7_300` keeps the same end of the `DATE` range in both rule sets.

use serde_json::json;

use super::fixtures::ROWS;
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{id, machine, on_platform, primitive_row, struct_row, types_for};
use crate::plant::{LeafPath, PlcPlatform, Primitive, TypeTrees};
use crate::recipe::snapshot::fixtures::{limited, param, range};
use crate::recipe::{
    DiffKind, DiffPair, DiffPart, DiffRows, Parameter, ParameterKey, ParameterValue, RecipeError,
    StoredParameter, diff_pairs,
};

/// The `DATE` primitive.
const DATE: u128 = 1;
/// `ST_Zone` with the member `due` of type `DATE`.
const ZONE: u128 = 2;
/// `ST_Line` with the member `zone` of type `ST_Zone`.
const LINE: u128 = 3;
/// A count that the earlier set accepts and the current set refuses.
const EARLIER_ONLY: i64 = 72_700;
/// A count that both sets refuse.
const NEITHER: i64 = 72_900;
/// The first count of `DATE` on `S7_1500` in both rule sets.
const FIRST_DAY: i64 = 7_305;
/// The text of [`EARLIER_ONLY`].
const EARLIER_ONLY_TEXT: &str = "D#2169-01-17";

/// The trees of `DATE`, `ST_Zone`, and `ST_Line` on `S7_1500`.
fn trees() -> TypeTrees {
    let rows = on_platform(
        PlcPlatform::S7_1500,
        vec![
            primitive_row(DATE, Primitive::Date),
            struct_row(ZONE, "ST_Zone", &[("due", DATE)]),
            struct_row(LINE, "ST_Line", &[("zone", ZONE)]),
        ],
    );
    let ids = [DATE, ZONE, LINE].map(id);
    TypeTrees::resolve(ids, &types_for(machine(), PlcPlatform::S7_1500, rows))
}

/// `DUE` of type `DATE` with the stored count `count`.
fn due(count: i64) -> StoredParameter {
    param(1, "DUE", DATE, json!(count))
}

/// `LINE_A` of type `ST_Line` with the stored count `count` at `.zone.due`.
fn line(count: i64) -> StoredParameter {
    param(2, "LINE_A", LINE, json!({"zone": {"due": count}}))
}

/// The text and the path of each leaf of the stored value of `stored`.
fn leaf_texts(stored: StoredParameter, trees: &TypeTrees) -> Vec<(String, String)> {
    let parameter = Parameter::from(stored);
    let tree = trees.tree(parameter.plc_type()).unwrap();
    let value = ParameterValue::try_from((parameter.value(), tree)).unwrap();
    value
        .texts(tree)
        .unwrap()
        .into_iter()
        .map(|leaf| (leaf.path.to_string(), leaf.text))
        .collect()
}

/// Return the result of `check_stored` for one added parameter.
fn check_of(stored: StoredParameter) -> Result<(), RecipeError> {
    let new = [Parameter::from(stored)];
    let keys: Vec<ParameterKey> = new.iter().map(Parameter::key).collect();
    DiffPair::match_by_key(&keys, &[], &new)
        .iter()
        .try_for_each(|pair| pair.check_stored(&trees()))
}

/// Diff one added parameter.
fn diff_added(stored: StoredParameter) -> DiffRows {
    let new = [Parameter::from(stored)];
    let keys: Vec<ParameterKey> = new.iter().map(Parameter::key).collect();
    let pairs = DiffPair::match_by_key(&keys, &[], &new);
    diff_pairs(&pairs, &trees(), ROWS)
}

#[test]
fn the_text_of_a_date_count_in_the_earlier_range_is_its_literal() {
    let texts = leaf_texts(due(EARLIER_ONLY), &trees());

    assert_eq!(texts, [(String::new(), EARLIER_ONLY_TEXT.to_owned())]);
}

#[test]
fn the_text_of_a_date_leaf_in_a_struct_is_the_same_literal() {
    let texts = leaf_texts(line(EARLIER_ONLY), &trees());

    assert_eq!(
        texts,
        [(".zone.due".to_owned(), EARLIER_ONLY_TEXT.to_owned())]
    );
}

#[test]
fn the_text_of_a_date_limit_bound_in_the_earlier_range_is_its_literal() {
    let stored = limited(due(FIRST_DAY), vec![range("", FIRST_DAY, EARLIER_ONLY)]);

    let rows = diff_added(stored);

    let texts: Vec<Option<String>> = rows
        .section(DiffKind::Limits)
        .iter()
        .filter_map(|row| match row.part() {
            DiffPart::Limit { path, new, .. } if *path == LeafPath::whole() => {
                Some(new.as_ref().and_then(|bounds| bounds.max.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(texts, [Some(EARLIER_ONLY_TEXT.to_owned())]);
}

#[test]
fn check_stored_accepts_a_date_count_in_the_earlier_range() {
    let result = check_of(due(EARLIER_ONLY));

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn check_stored_refuses_a_date_count_outside_the_earlier_range_with_rcp_005() {
    let result = check_of(due(NEITHER));

    assert_eq!(
        result.as_ref().err().map(RecipeError::code),
        Some(ErrorCode::RCP_005),
        "{result:?}"
    );
}

#[test]
fn check_stored_accepts_a_date_limit_bound_in_the_earlier_range() {
    let stored = limited(due(FIRST_DAY), vec![range("", FIRST_DAY, EARLIER_ONLY)]);

    let result = check_of(stored);

    assert!(result.is_ok(), "{result:?}");
}
