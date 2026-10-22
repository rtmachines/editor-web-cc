//! Tests of the order of the limit rows of one parameter (ADR-0039 (k), ADR-0050 (d)).

use serde_json::json;

use super::fixtures::{GRID, MISSING, ORDER, ROWS, TEN, diff_of};
use crate::recipe::snapshot::fixtures::{limited, param, range};
use crate::recipe::{DiffKind, DiffPart, DiffRows, StoredLimit, StoredParameter};

/// The leaf path text of each row of the limits section.
fn limit_paths(rows: &DiffRows) -> Vec<String> {
    rows.section(DiffKind::Limits)
        .iter()
        .filter_map(|row| match row.part() {
            DiffPart::Limit { path, .. } => Some(path.to_string()),
            _ => None,
        })
        .collect()
}

/// `TEN` of type `ARRAY[1..10] OF INT` with the limit rows.
fn ten(limits: Vec<StoredLimit>) -> StoredParameter {
    let values: Vec<i32> = (1..=10).collect();
    limited(param(7, "TEN", TEN, json!(values)), limits)
}

/// `ORDER` of type `ST_Order` (members `zeta`, then `alpha`) with the limit rows.
fn order(limits: Vec<StoredLimit>) -> StoredParameter {
    limited(
        param(8, "ORDER", ORDER, json!({"zeta": 1, "alpha": 1})),
        limits,
    )
}

#[test]
fn element_limit_rows_come_in_preorder_whatever_the_input_order() {
    let old = ten((1..=10).map(|n| range(&format!("[{n}]"), 0, 100)).collect());
    for changed in [
        vec![range("[10]", 0, 50), range("[2]", 0, 50)],
        vec![range("[2]", 0, 50), range("[10]", 0, 50)],
    ] {
        let mut limits: Vec<StoredLimit> = (1..=10)
            .filter(|n| *n != 2 && *n != 10)
            .map(|n| range(&format!("[{n}]"), 0, 100))
            .collect();
        limits.splice(0..0, changed);
        let rows = diff_of(vec![old.clone()], vec![ten(limits)], ROWS);
        assert_eq!(limit_paths(&rows), ["[2]", "[10]"]);
    }
}

#[test]
fn member_limit_rows_come_in_the_order_of_the_type_tree() {
    let old = order(vec![range(".alpha", 0, 10), range(".zeta", 0, 10)]);
    let new = order(vec![range(".alpha", 0, 20), range(".zeta", 0, 20)]);
    let rows = diff_of(vec![old], vec![new], ROWS);
    assert_eq!(limit_paths(&rows), [".zeta", ".alpha"]);
}

#[test]
fn a_row_of_the_old_side_only_takes_the_position_of_its_path_in_the_old_tree() {
    let old = order(vec![range(".alpha", 0, 10), range(".zeta", 0, 10)]);
    let new = order(vec![range(".alpha", 0, 20)]);
    let rows = diff_of(vec![old], vec![new], ROWS);
    assert_eq!(limit_paths(&rows), [".zeta", ".alpha"]);
}

#[test]
fn nested_array_rows_come_in_the_order_of_their_first_covered_leaf() {
    let grid = |limits| limited(param(11, "GRID", GRID, json!([[1, 2], [3, 4]])), limits);
    let old = grid(vec![range("[2][1]", 0, 10), range("[1][2]", 0, 10)]);
    let new = grid(vec![range("[*][2]", 0, 10), range("[*][1]", 0, 10)]);
    let rows = diff_of(vec![old], vec![new], ROWS);
    assert_eq!(limit_paths(&rows), ["[*][1]", "[*][2]", "[1][2]", "[2][1]"]);
}

#[test]
fn rows_without_a_tree_come_last_in_the_order_of_their_paths() {
    let old = limited(
        param(7, "TEN", MISSING, json!(0)),
        vec![range("[3]", 0, 100), range("[1]", 0, 100)],
    );
    let new = ten(vec![range("[10]", 0, 50), range("[2]", 0, 50)]);
    let rows = diff_of(vec![old], vec![new], ROWS);
    assert_eq!(limit_paths(&rows), ["[2]", "[10]", "[1]", "[3]"]);
}
