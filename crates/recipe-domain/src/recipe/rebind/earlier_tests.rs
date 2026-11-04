//! Tests of the rebind on kept data that is valid under the earlier rule set only (ADR-0120).
//!
//! A rebind reads the kept value and the kept limit rows of a parameter and does not change them.
//! It checks them under the earlier rule set. It checks the values of the request under the
//! current rule set.

use rust_decimal::Decimal;
use serde_json::json;

use super::test_support::{draft_on, key, number, param, request_with, request_with_texts};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{
    id, machine, on_platform, primitive_row, struct_row_revision, types_for,
};
use crate::plant::{LeafPath, PlcPlatform, Primitive, TypeTrees};
use crate::recipe::{Draft, DraftError, RebindRequest, StoredLimit};

const INT: u128 = 1;
const REAL: u128 = 2;
const DATE: u128 = 3;
const PROCESS_1: u128 = 10;
const PROCESS_2: u128 = 11;
const PROCESS_REAL_2: u128 = 12;

/// The trees of `ST_Process` on `S7_1500`. Revision 2 adds the member `extra`.
fn process_trees() -> TypeTrees {
    let members: [(&str, u128); 2] = [("level", REAL), ("due", DATE)];
    let with_extra: [(&str, u128); 3] = [("level", REAL), ("due", DATE), ("extra", INT)];
    let with_real_extra: [(&str, u128); 3] = [("level", REAL), ("due", DATE), ("extra", REAL)];
    let rows = vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(DATE, Primitive::Date),
        struct_row_revision(PROCESS_1, "ST_Process", 1, &members),
        struct_row_revision(PROCESS_2, "ST_Process", 2, &with_extra),
        struct_row_revision(PROCESS_REAL_2, "ST_Process", 3, &with_real_extra),
    ];
    TypeTrees::resolve(
        [PROCESS_1, PROCESS_2, PROCESS_REAL_2].map(id),
        &types_for(
            machine(),
            PlcPlatform::S7_1500,
            on_platform(PlcPlatform::S7_1500, rows),
        ),
    )
}

/// A limit row on `path` with the given bounds.
fn bounds(path: &str, min: Option<&str>, max: Option<&str>) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: min.map(|text| text.parse::<Decimal>().unwrap()),
        max: max.map(|text| text.parse::<Decimal>().unwrap()),
        max_length: None,
    }
}

/// A draft on `S7_1500` with one `ST_Process` parameter that holds the given value and limits.
fn process_draft(value: serde_json::Value, limits: Vec<StoredLimit>) -> Draft {
    draft_on(
        PlcPlatform::S7_1500,
        vec![param(1, "PROCESS", PROCESS_1, value, limits)],
    )
}

fn extra_request() -> RebindRequest {
    request_with(PROCESS_1, PROCESS_2, &[(1, ".extra", number(4))])
}

/// Plan the rebind and apply the plan to the draft.
fn rebind(draft: &mut Draft, request: &RebindRequest) -> Result<(), DraftError> {
    let trees = process_trees();
    let plan = draft.plan_rebind(request, &trees)?;
    draft
        .apply_rebind(plan, &std::collections::BTreeMap::new(), &trees)
        .map(|_| ())
}

#[test]
fn rebind_keeps_a_real_limit_with_8_significant_digits() {
    let kept = bounds(".level", Some("0"), Some("100.00001"));
    let mut draft = process_draft(json!({"level": 1, "due": 8000}), vec![kept.clone()]);

    rebind(&mut draft, &extra_request()).unwrap();

    let parameter = draft.parameter(key(1)).unwrap();
    assert_eq!(parameter.limits(), &[kept][..]);
    assert_eq!(parameter.plc_type(), id(PROCESS_2));
}

#[test]
fn plan_keeps_the_row_of_a_real_limit_with_8_significant_digits() {
    let kept = bounds(".level", Some("0"), Some("100.00001"));
    let draft = process_draft(json!({"level": 1, "due": 8000}), vec![kept.clone()]);

    let plan = draft
        .plan_rebind(&extra_request(), &process_trees())
        .unwrap();

    assert_eq!(plan.limits(key(1)), Some(&[kept][..]));
}

#[test]
fn rebind_keeps_a_date_above_the_current_end_with_a_limit_that_has_no_max() {
    let kept = bounds(".due", Some("70000"), None);
    let mut draft = process_draft(json!({"level": 1, "due": 72_700}), vec![kept.clone()]);

    rebind(&mut draft, &extra_request()).unwrap();

    let parameter = draft.parameter(key(1)).unwrap();
    assert_eq!(parameter.limits(), &[kept][..]);
    assert_eq!(
        parameter.value().as_json(),
        &json!({"level": 1, "due": 72_700, "extra": 4})
    );
}

#[test]
fn rebind_refuses_a_date_above_the_earlier_end() {
    let mut draft = process_draft(json!({"level": 1, "due": 72_900}), Vec::new());
    let before = draft.clone();

    let error = rebind(&mut draft, &extra_request()).unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005, "{error}");
    assert_eq!(draft.parameters(), before.parameters());
}

#[test]
fn rebind_refuses_a_new_real_value_with_8_significant_digits() {
    let mut draft = process_draft(json!({"level": 1, "due": 8000}), Vec::new());
    let request = request_with_texts(PROCESS_1, PROCESS_REAL_2, &[(1, ".extra", "100.00001")]);

    let error = rebind(&mut draft, &request).unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_055, "{error}");
}
