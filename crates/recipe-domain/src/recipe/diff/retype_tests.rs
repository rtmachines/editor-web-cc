//! Tests of a type change: every leaf and every limit row of both sides (ADR-0050 (c)).

use rust_decimal::Decimal;
use serde_json::json;

use super::fixtures::{DINT, INT, LTIME, ROWS, TIME, diff_of};
use crate::plant::LeafPath;
use crate::recipe::snapshot::fixtures::{limited, param, range};
use crate::recipe::{DiffBounds, DiffKind, DiffPart, DiffRows, StoredLimit};

/// The parts of one section.
fn parts(rows: &DiffRows, kind: DiffKind) -> Vec<DiffPart> {
    rows.section(kind)
        .iter()
        .map(|row| row.part().clone())
        .collect()
}

/// The bounds with no `min` and the `max` text.
fn up_to(max: &str) -> Option<DiffBounds> {
    Some(DiffBounds {
        min: None,
        max: Some(max.to_owned()),
        max_length: None,
    })
}

/// The bounds `0` to `100`.
fn percent() -> Option<DiffBounds> {
    Some(DiffBounds {
        min: Some("0".to_owned()),
        max: Some("100".to_owned()),
        max_length: None,
    })
}

#[test]
fn type_change_keeps_the_stored_bound() {
    let max = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(Decimal::from(1_000)),
        max_length: None,
    };
    let hold = |type_n| limited(param(4, "HOLD", type_n, json!(500)), vec![max.clone()]);
    let rows = diff_of(vec![hold(TIME)], vec![hold(LTIME)], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Limits),
        vec![DiffPart::Limit {
            path: LeafPath::parse("").unwrap(),
            old: up_to("T#1s"),
            new: up_to("LT#1us"),
        }]
    );
}

#[test]
fn a_change_from_dint_to_time_gives_a_value_row() {
    let hold = |type_n| param(4, "HOLD", type_n, json!(5_000));
    let rows = diff_of(vec![hold(DINT)], vec![hold(TIME)], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Values),
        vec![DiffPart::Value {
            path: LeafPath::parse("").unwrap(),
            old: Some("5000".to_owned()),
            new: Some("T#5s".to_owned()),
            limit: None,
        }]
    );
}

#[test]
fn a_type_change_with_equal_texts_gives_the_value_row_and_the_limit_row() {
    let speed = |type_n| limited(param(1, "SPEED", type_n, json!(5)), vec![range("", 0, 100)]);
    let rows = diff_of(vec![speed(INT)], vec![speed(DINT)], ROWS);
    let path = LeafPath::parse("").unwrap();
    assert_eq!(
        parts(&rows, DiffKind::Values),
        vec![DiffPart::Value {
            path: path.clone(),
            old: Some("5".to_owned()),
            new: Some("5".to_owned()),
            limit: percent(),
        }]
    );
    assert_eq!(
        parts(&rows, DiffKind::Limits),
        vec![DiffPart::Limit {
            path,
            old: percent(),
            new: percent(),
        }]
    );
}
