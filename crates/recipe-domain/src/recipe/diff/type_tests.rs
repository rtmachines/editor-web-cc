//! Tests of the type of a definition row and of an added parameter (ADR-0050 (c), (d)).

use serde_json::json;

use super::fixtures::{DINT_TABLE, ROWS, TABLE, ZONES_1, ZONES_2, diff_of};
use crate::plant::PlcTypeId;
use crate::plant::fixtures::id;
use crate::recipe::snapshot::fixtures::param;
use crate::recipe::{DefinitionChange, DiffKind, DiffPart, DiffRows, StoredParameter};

/// The type changes of the definitions section.
fn type_changes(rows: &DiffRows) -> Vec<DefinitionChange> {
    rows.section(DiffKind::Definitions)
        .iter()
        .filter_map(|row| match row.part() {
            DiffPart::Definition(change @ DefinitionChange::Type { .. }) => Some(change.clone()),
            _ => None,
        })
        .collect()
}

/// `SPEED_TABLE` of the array type `type_n` with eight values.
fn table(type_n: u128) -> StoredParameter {
    param(2, "SPEED_TABLE", type_n, json!([1, 2, 3, 4, 5, 6, 7, 8]))
}

/// `ZONES` of the array type `type_n` with two zones.
fn zones(type_n: u128) -> StoredParameter {
    let zone = json!({"setpoint": 1});
    param(6, "ZONES", type_n, json!([zone, zone]))
}

#[test]
fn a_change_of_the_element_type_gives_a_type_row_with_two_different_types() {
    let rows = diff_of(vec![table(TABLE)], vec![table(DINT_TABLE)], ROWS);
    assert_eq!(
        type_changes(&rows),
        vec![DefinitionChange::Type {
            old: id(TABLE),
            new: id(DINT_TABLE),
        }]
    );
}

#[test]
fn a_rebind_of_an_array_of_a_struct_gives_a_type_row_with_two_different_types() {
    let rows = diff_of(vec![zones(ZONES_1)], vec![zones(ZONES_2)], ROWS);
    assert_eq!(
        type_changes(&rows),
        vec![DefinitionChange::Type {
            old: id(ZONES_1),
            new: id(ZONES_2),
        }]
    );
}

#[test]
fn an_added_array_parameter_carries_its_type() {
    let rows = diff_of(Vec::new(), vec![table(DINT_TABLE)], ROWS);
    let types: Vec<PlcTypeId> = rows
        .section(DiffKind::Added)
        .iter()
        .filter_map(|row| match row.part() {
            DiffPart::Parameter { plc_type, .. } => Some(*plc_type),
            _ => None,
        })
        .collect();
    assert_eq!(types, vec![id(DINT_TABLE)]);
}
