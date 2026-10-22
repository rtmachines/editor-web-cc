//! Tests of the presence of the parameter of a row in the new version (ADR-0050 (d)).
//!
//! A page links the tag of a row to a version that holds the parameter. So each row tells if the
//! new version holds its parameter, also a limit row that only the old side holds.

use serde_json::json;

use super::fixtures::{INT, ROWS, diff_of};
use crate::recipe::snapshot::fixtures::{limited, param, range};
use crate::recipe::{DiffKind, DiffRows, StoredLimit, StoredParameter};

/// `KEPT` of type `INT` with the value `value` and the limit rows `limits`.
fn kept(value: i64, limits: Vec<StoredLimit>) -> StoredParameter {
    limited(param(1, "KEPT", INT, json!(value)), limits)
}

/// `GONE` of type `INT`, which only the old version holds.
fn gone() -> StoredParameter {
    limited(param(2, "GONE", INT, json!(3)), vec![range("", 0, 10)])
}

/// `ADDED` of type `INT`, which only the new version holds.
fn added() -> StoredParameter {
    limited(param(3, "ADDED", INT, json!(4)), vec![range("", 0, 10)])
}

/// The diff of the three parameters. The new `KEPT` has no limit row, so its one limit row only
/// has the old side.
fn rows() -> DiffRows {
    diff_of(
        vec![kept(5, vec![range("", 0, 100)]), gone()],
        vec![kept(6, Vec::new()), added()],
        ROWS,
    )
}

/// Return the section and the presence of each row of the parameter `tag`.
fn presence_of(rows: &DiffRows, tag: &str) -> Vec<(DiffKind, bool)> {
    DiffKind::ALL
        .into_iter()
        .flat_map(|kind| {
            rows.section(kind)
                .iter()
                .filter(|row| row.tag().as_str() == tag)
                .map(move |row| (kind, row.is_in_new_version()))
        })
        .collect()
}

#[test]
fn every_row_of_a_kept_parameter_is_in_the_new_version_also_an_old_only_limit_row() {
    let rows = rows();
    assert_eq!(
        presence_of(&rows, "KEPT"),
        vec![(DiffKind::Limits, true), (DiffKind::Values, true)]
    );
}

#[test]
fn every_row_of_a_removed_parameter_is_not_in_the_new_version() {
    let rows = rows();
    assert_eq!(
        presence_of(&rows, "GONE"),
        vec![(DiffKind::Limits, false), (DiffKind::Removed, false)]
    );
}

#[test]
fn every_row_of_an_added_parameter_is_in_the_new_version() {
    let rows = rows();
    assert_eq!(
        presence_of(&rows, "ADDED"),
        vec![(DiffKind::Limits, true), (DiffKind::Added, true)]
    );
}
