//! Test of ADR-0177 in `ApproveVersion`: a stored array of an array on S7-300 stays approvable.

use std::collections::BTreeSet;

use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafPath, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    Primitive, RecordShape, S7Address,
};
use recipe_domain::recipe::{ApprovalOutcome, StoredLimit, StoredParameter, StoredValue};
use serde_json::json;

use super::{run, seed_with};
use crate::memory::fixtures::{Fixture, mid, parameter_on, tid, uid};

/// An array row of machine 1 on `S7_300` with the bounds `1..=upper`.
fn array_row(id: u128, element: PlcTypeId, upper: i32) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PlcPlatform::S7_300]),
        shape: RecordShape::Array {
            element,
            dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(1, upper).unwrap()])
                .unwrap(),
        },
    })
    .unwrap()
}

/// The parameter `GRID` of type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` at `DB1.DBB0`.
fn grid_parameter(plc_type: PlcTypeId) -> StoredParameter {
    StoredParameter {
        plc_type,
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        value: StoredValue::from(json!([[1, 2, 3], [4, 5, 6]])),
        limits: vec![StoredLimit {
            leaf_path: LeafPath::parse("[*][*]").unwrap(),
            min: Some("0".parse().unwrap()),
            max: Some("255".parse().unwrap()),
            max_length: None,
        }],
        ..parameter_on(PlcPlatform::S7_300, 1, "GRID", uid(102), 0)
    }
}

#[tokio::test]
async fn adr_0177_a_stored_array_of_an_array_on_s7_300_stays_approvable() {
    let fixture = Fixture::new();
    let byte = fixture.seed_primitive_type(821, PlcPlatform::S7_300, Primitive::Byte);
    fixture.store.seed_type(&array_row(811, byte, 3));
    fixture.store.seed_type(&array_row(812, tid(811), 2));
    seed_with(&fixture, PlcPlatform::S7_300, &[grid_parameter(tid(812))]);
    let outcome = run(&fixture, 1, 1, false).await.unwrap();
    assert!(
        matches!(outcome, ApprovalOutcome::Approved { .. }),
        "{outcome:?}"
    );
}
