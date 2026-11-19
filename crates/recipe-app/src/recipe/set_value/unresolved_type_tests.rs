//! A stored type that does not resolve gives a storage defect (ADR-0195 (w)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    DerivedTypeReason, LeafPath, PlantError, PlatformTypes, PlcPlatform, PlcType, TypeTree,
};
use recipe_domain::recipe::{LimitInput, LimitList, StoredParameter, ValueInput};
use serde_json::json;

use super::{SetParameterValue, SetParameterValueInput};
use crate::memory::fixtures::{
    ARRAY_CYCLE_ROOT, Fixture, STRUCT_CYCLE_ROOT, actor, meta, mid, parameter_on, pkey, reference,
    seed_array_cycle, seed_struct_cycle, tid, uid, vid,
};
use crate::recipe::test_support::{DRAFT_VERSION, stored_parameters};
use crate::recipe::{ReplaceLimits, ReplaceLimitsInput};

const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
const CYCLE_PARAMETER: u128 = 5;

/// The draft `R-1` on `M1` with a parameter of the type `root`. The rows belong to `M1`.
fn draft_with_parameter_of(
    root: u128,
    seed_rows: fn(&Fixture, (u128, PlcPlatform)) -> Vec<PlcType>,
) -> (Fixture, Vec<PlcType>) {
    let fixture = crate::memory::fixtures::scenarios::zone_draft();
    let rows = seed_rows(&fixture, (1, PLATFORM));
    let cycle = StoredParameter {
        plc_type: tid(root),
        value: json!({"x": 1}).into(),
        limits: Vec::new(),
        ..parameter_on(PLATFORM, CYCLE_PARAMETER, "CYCLE", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &cycle);
    (fixture, rows)
}

fn resolution(rows: &[PlcType], root: u128) -> PlantError {
    let types = PlatformTypes::collect(mid(1), PLATFORM, rows.iter().cloned()).unwrap();
    TypeTree::resolve(tid(root), &types).unwrap_err()
}

fn assert_nothing_written(fixture: &Fixture) {
    assert!(fixture.store.audit_entries().is_empty());
    let cycle = stored_parameters(fixture)
        .into_iter()
        .find(|parameter| parameter.key == pkey(CYCLE_PARAMETER))
        .unwrap();
    assert_eq!(
        cycle.value.as_json(),
        &json!({"x": 1}),
        "the value must stay"
    );
    assert!(cycle.limits.is_empty(), "the limits must stay");
}

async fn set_value(fixture: &Fixture) -> ErrorCode {
    let input = SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(CYCLE_PARAMETER),
        value: ValueInput::Json("{\"x\": 2}".to_owned()),
    };
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
        .unwrap_err();
    error.code()
}

async fn replace_limits(fixture: &Fixture) -> ErrorCode {
    let rows = vec![LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some("5".to_owned()),
        max_length: None,
    }];
    let input = ReplaceLimitsInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(CYCLE_PARAMETER),
        limits: LimitList::parse(rows).unwrap(),
    };
    let error = ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
        .unwrap_err();
    error.code()
}

#[tokio::test]
async fn a_parameter_of_a_struct_cycle_gives_db_008_for_a_value() {
    let (fixture, rows) = draft_with_parameter_of(STRUCT_CYCLE_ROOT, seed_struct_cycle);
    assert!(matches!(
        resolution(&rows, STRUCT_CYCLE_ROOT),
        PlantError::TypeCycle { .. }
    ));
    assert_eq!(set_value(&fixture).await, ErrorCode::DB_008);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_parameter_of_a_struct_cycle_gives_db_008_for_limits() {
    let (fixture, rows) = draft_with_parameter_of(STRUCT_CYCLE_ROOT, seed_struct_cycle);
    assert!(matches!(
        resolution(&rows, STRUCT_CYCLE_ROOT),
        PlantError::TypeCycle { .. }
    ));
    assert_eq!(replace_limits(&fixture).await, ErrorCode::DB_008);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_parameter_of_an_array_only_cycle_gives_db_008_for_limits() {
    let (fixture, rows) = draft_with_parameter_of(ARRAY_CYCLE_ROOT, seed_array_cycle);
    assert!(matches!(
        resolution(&rows, ARRAY_CYCLE_ROOT),
        PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::RowShape,
            ..
        }
    ));
    assert_eq!(replace_limits(&fixture).await, ErrorCode::DB_008);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_parameter_of_an_array_only_cycle_gives_db_008_for_a_value() {
    let (fixture, _) = draft_with_parameter_of(ARRAY_CYCLE_ROOT, seed_array_cycle);
    assert_eq!(set_value(&fixture).await, ErrorCode::DB_008);
    assert_nothing_written(&fixture);
}
