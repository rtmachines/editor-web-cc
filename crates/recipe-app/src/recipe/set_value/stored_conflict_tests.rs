//! Two stored limit rows on one leaf give a storage defect for a value (ADR-0195 (f)).

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafPath, PlcPlatform, PlcType, PlcTypeRecord, Primitive,
    RecordShape,
};
use recipe_domain::recipe::{StoredLimit, StoredParameter, ValueInput};
use serde_json::json;

use super::{SetParameterValue, SetParameterValueInput};
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, pkey, reference, tid, uid, vid,
};
use crate::recipe::test_support::{DRAFT_VERSION, stored_parameters};

const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
const INT_ROW: u128 = 970;
const ARRAY_ROW: u128 = 971;

fn limit_on(path: &str, max: i64) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: Some(max.into()),
        max_length: None,
    }
}

/// The draft `R-1` with the parameter `RAMPS` (`ARRAY[1..3] OF INT`) and the limits `limits`.
fn draft_with_limits(limits: Vec<StoredLimit>) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let element = fixture.seed_primitive_type(INT_ROW, PLATFORM, Primitive::Int);
    let dimensions = ArrayDimensions::parse(vec![ArrayDimension::parse(1, 3).unwrap()]).unwrap();
    let array = PlcType::try_from(PlcTypeRecord {
        id: tid(ARRAY_ROW),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Array {
            element,
            dimensions,
        },
    })
    .unwrap();
    fixture.store.seed_type(&array);
    let ramps = StoredParameter {
        plc_type: tid(ARRAY_ROW),
        value: json!([1, 2, 3]).into(),
        limits,
        ..parameter_on(PLATFORM, 1, "RAMPS", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &ramps);
    fixture
}

async fn set_value(fixture: &Fixture) -> ErrorCode {
    let input = SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(1),
        value: ValueInput::Json("[4,5,6]".to_owned()),
    };
    SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
        .unwrap_err()
        .code()
}

#[tokio::test]
async fn two_stored_limits_on_one_leaf_give_db_008_and_write_nothing() {
    let fixture = draft_with_limits(vec![limit_on("[*]", 10), limit_on("[2]", 10)]);
    assert_eq!(set_value(&fixture).await, ErrorCode::DB_008);
    assert!(fixture.store.audit_entries().is_empty());
    let parameters = stored_parameters(&fixture);
    assert_eq!(parameters.len(), 1);
    assert_eq!(
        parameters.first().unwrap().value.as_json(),
        &json!([1, 2, 3]),
        "the value must stay"
    );
}

#[tokio::test]
async fn two_stored_limits_on_different_leaves_let_the_value_pass() {
    let fixture = draft_with_limits(vec![limit_on("[1]", 10), limit_on("[2]", 10)]);
    let input = SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(1),
        value: ValueInput::Json("[4,5,6]".to_owned()),
    };
    // Leaf 3 has no limit, so the value passes the limit check and the command succeeds.
    SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
        .unwrap();
}
