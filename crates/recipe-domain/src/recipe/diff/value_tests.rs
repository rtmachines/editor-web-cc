//! Tests of the diff of the values and the limit rows (A43, ADR-0022, ADR-0050).

use serde_json::{Value, json};

use super::fixtures::{LINE, PAIR, PAIR_C, ROWS, TABLE, TIME, check_of, diff_of};
use crate::error_codes::ErrorCode;
use crate::plant::LeafPath;
use crate::recipe::snapshot::fixtures::{limited, param, range};
use crate::recipe::{DiffBounds, DiffKind, DiffPart, DiffRows, RecipeError, StoredLimit};

/// The bounds `min` to `max` as decimal text.
fn bounds(min: &str, max: &str) -> Option<DiffBounds> {
    Some(DiffBounds {
        min: Some(min.to_owned()),
        max: Some(max.to_owned()),
        max_length: None,
    })
}

fn path(text: &str) -> LeafPath {
    LeafPath::parse(text).unwrap()
}

fn text(value: &str) -> Option<String> {
    Some(value.to_owned())
}

/// The parts of one section.
fn parts(rows: &DiffRows, kind: DiffKind) -> Vec<DiffPart> {
    rows.section(kind)
        .iter()
        .map(|row| row.part().clone())
        .collect()
}

/// `LINE_A` of type `ST_Line` with the setpoint `setpoint` and the limit 0 to 100.
fn line(setpoint: Value) -> crate::recipe::StoredParameter {
    let value = json!({"zone": {"setpoint": setpoint}});
    limited(
        param(1, "LINE_A", LINE, value),
        vec![range(".zone.setpoint", 0, 100)],
    )
}

/// `SPEED_TABLE` of type `ARRAY[1..8] OF INT` with the values and the limits.
fn table(values: [i32; 8], limits: Vec<StoredLimit>) -> crate::recipe::StoredParameter {
    limited(param(2, "SPEED_TABLE", TABLE, json!(values)), limits)
}

/// The limit rows `[1]` to `[8]` from 0 to 100.
fn element_limits() -> Vec<StoredLimit> {
    (1..=8).map(|n| range(&format!("[{n}]"), 0, 100)).collect()
}

#[test]
fn a_value_in_a_struct_in_a_struct_gives_a_row_of_its_leaf() {
    let rows = diff_of(vec![line(json!(1))], vec![line(json!(2))], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Values),
        vec![DiffPart::Value {
            path: path(".zone.setpoint"),
            old: text("1"),
            new: text("2"),
            limit: bounds("0", "100"),
        }]
    );
    assert!(rows.section(DiffKind::Limits).is_empty());
    assert_eq!(rows.unchanged(), 0);
}

#[test]
fn value_with_a_decimal_comma_in_the_ui_language() {
    let rows = diff_of(vec![line(json!(85.5))], vec![line(json!(90))], ROWS);
    let texts: Vec<(Option<String>, Option<String>)> = parts(&rows, DiffKind::Values)
        .into_iter()
        .filter_map(|part| match part {
            DiffPart::Value { old, new, .. } => Some((old, new)),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec![(text("85.5"), text("90"))]);
}

#[test]
fn limit_of_every_element() {
    let values = [1, 2, 3, 4, 5, 6, 7, 8];
    let old = table(values, vec![range("[*]", 0, 100)]);
    let new = table(values, vec![range("[*]", 10, 100)]);
    let rows = diff_of(vec![old], vec![new], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Limits),
        vec![DiffPart::Limit {
            path: path("[*]"),
            old: bounds("0", "100"),
            new: bounds("10", "100"),
        }]
    );
    assert!(rows.section(DiffKind::Values).is_empty());
}

#[test]
fn a_changed_element_limit_gives_one_row_of_its_element() {
    let values = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut new_limits = element_limits();
    new_limits.retain(|limit| limit.leaf_path != path("[3]"));
    new_limits.push(range("[3]", 0, 20));
    let rows = diff_of(
        vec![table(values, element_limits())],
        vec![table(values, new_limits)],
        ROWS,
    );
    assert_eq!(
        parts(&rows, DiffKind::Limits),
        vec![DiffPart::Limit {
            path: path("[3]"),
            old: bounds("0", "100"),
            new: bounds("0", "20"),
        }]
    );
}

#[test]
fn change_of_the_limit_mode() {
    let values = [1, 2, 3, 4, 5, 6, 7, 8];
    let old = table(values, vec![range("[*]", 0, 100)]);
    let new = table(values, element_limits());
    let rows = diff_of(vec![old], vec![new], ROWS);
    let elements = (1..=8).map(|n| DiffPart::Limit {
        path: path(&format!("[{n}]")),
        old: None,
        new: bounds("0", "100"),
    });
    let all = DiffPart::Limit {
        path: path("[*]"),
        old: bounds("0", "100"),
        new: None,
    };
    let expected: Vec<DiffPart> = std::iter::once(all).chain(elements).collect();
    assert_eq!(parts(&rows, DiffKind::Limits), expected);
}

#[test]
fn each_value_row_holds_the_effective_limit_of_the_new_version() {
    let old = table([1, 2, 3, 4, 5, 6, 7, 8], vec![range("[*]", 0, 100)]);
    let all = table([1, 9, 3, 4, 5, 6, 7, 8], vec![range("[*]", 0, 100)]);
    let rows = diff_of(vec![old.clone()], vec![all], ROWS);
    let limit_of = |rows: &DiffRows| -> Vec<Option<DiffBounds>> {
        parts(rows, DiffKind::Values)
            .into_iter()
            .filter_map(|part| match part {
                DiffPart::Value { limit, .. } => Some(limit),
                _ => None,
            })
            .collect()
    };
    assert_eq!(limit_of(&rows), vec![bounds("0", "100")]);
    let mut own = element_limits();
    own.retain(|limit| limit.leaf_path != path("[2]"));
    own.push(range("[2]", 0, 50));
    let element = table([1, 9, 3, 4, 5, 6, 7, 8], own);
    let rows = diff_of(vec![old], vec![element], ROWS);
    assert_eq!(limit_of(&rows), vec![bounds("0", "50")]);
}

#[test]
fn a_type_change_compares_the_leaves_by_leaf_path() {
    let old = limited(
        param(3, "PAIR", PAIR, json!({"a": 1, "b": 2})),
        vec![range(".a", 0, 10), range(".b", 0, 10)],
    );
    let new = limited(
        param(3, "PAIR", PAIR_C, json!({"a": 1, "c": 3})),
        vec![range(".a", 0, 10), range(".c", 0, 30)],
    );
    let rows = diff_of(vec![old], vec![new], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Values),
        vec![
            DiffPart::Value {
                path: path(".a"),
                old: text("1"),
                new: text("1"),
                limit: bounds("0", "10"),
            },
            DiffPart::Value {
                path: path(".c"),
                old: None,
                new: text("3"),
                limit: bounds("0", "30"),
            },
            DiffPart::Value {
                path: path(".b"),
                old: text("2"),
                new: None,
                limit: None,
            },
        ]
    );
    let limit_paths: Vec<String> = parts(&rows, DiffKind::Limits)
        .into_iter()
        .filter_map(|part| match part {
            DiffPart::Limit { path, .. } => Some(path.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(limit_paths, [".a", ".b", ".c"]);
    assert_eq!(rows.section(DiffKind::Definitions).len(), 1);
}

#[test]
fn time_value() {
    let hold = |count: i64| {
        limited(
            param(4, "HOLD", TIME, json!(count)),
            vec![range("", 1_000, 60_000)],
        )
    };
    let rows = diff_of(vec![hold(5_000)], vec![hold(7_000)], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Values),
        vec![DiffPart::Value {
            path: path(""),
            old: text("T#5s"),
            new: text("T#7s"),
            limit: bounds("T#1s", "T#1m"),
        }]
    );
}

/// `HOLD` of type `TIME` with the stored count `count` and no limit row.
fn hold(count: i64) -> crate::recipe::StoredParameter {
    param(4, "HOLD", TIME, json!(count))
}

#[test]
fn a_stored_time_count_outside_the_type_range_on_any_side_fails_the_check_with_rcp_005() {
    let cases = [
        (
            "new side of a changed pair",
            vec![hold(5_000)],
            vec![hold(-1_000)],
        ),
        (
            "old side of a changed pair",
            vec![hold(-1_000)],
            vec![hold(5_000)],
        ),
        ("removed pair", vec![hold(-1_000)], Vec::new()),
        ("added pair", Vec::new(), vec![hold(-1_000)]),
    ];
    for (case, old, new) in cases {
        let result = check_of(old, new);

        assert!(
            matches!(&result, Err(RecipeError::ValueShapeMismatch { tag, .. }) if tag.to_string() == "HOLD"),
            "case: {case}, result: {result:?}"
        );
    }
}

#[test]
fn a_stored_time_bound_outside_the_type_range_fails_the_check_with_rcp_006() {
    let bound = |min: i64| limited(hold(5_000), vec![range("", min, 60_000)]);

    let result = check_of(vec![bound(1_000)], vec![bound(-1_000)]);

    assert_eq!(
        result.as_ref().err().map(RecipeError::code),
        Some(ErrorCode::RCP_006)
    );
    assert!(
        matches!(&result, Err(RecipeError::LimitOutsideTypeRange { tag, .. }) if tag.to_string() == "HOLD"),
        "{result:?}"
    );
}

#[test]
fn a_part_with_more_rows_than_leaf_rows_gives_the_first_rows_and_the_count() {
    let old = table([1, 2, 3, 4, 5, 6, 7, 8], vec![range("[*]", 0, 100)]);
    let new = table([11, 12, 13, 14, 15, 16, 17, 18], vec![range("[*]", 0, 100)]);
    let rows = diff_of(vec![old], vec![new], 3);
    let shown: Vec<String> = parts(&rows, DiffKind::Values)
        .into_iter()
        .map(|part| match part {
            DiffPart::Value { path, .. } => path.to_string(),
            DiffPart::Omitted { count } => format!("+{count}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(shown, ["[1]", "[2]", "[3]", "+5"]);
}
