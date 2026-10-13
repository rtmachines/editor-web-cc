//! Tests of the new array types, the apply step, and the writes of `RebindTypeRevision`
//! (design D14, steps 4 and 5).

use recipe_domain::audit::{AggregateType, AuditAction};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{ArrayDimensions, LeafPath, PlcPlatform, PlcTypeId, RecordShape};
use recipe_domain::recipe::StoredLimit;
use serde_json::json;

use super::fixtures::{
    BIG_2, BIG_ARRAY, PAIR_1, ZONE_1, ZONE_2, ZONES_1, arrays_of, big_request, empty_draft,
    machine_rows, near_budget_draft, request, run, s7_zones_draft, seed_parameter, stored,
    with_value, zone_request, zones_draft, zones_request, zones_value,
};
use crate::memory::WriteKind;
use crate::memory::fixtures::{Fixture, actor, rid, tid, vid};
use crate::recipe::DraftOutput;
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot};

fn zone_limit() -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(".setpoint").unwrap(),
        min: Some("0".parse().unwrap()),
        max: Some("9".parse().unwrap()),
        max_length: None,
    }
}

fn actions(fixture: &Fixture) -> Vec<AuditAction> {
    fixture
        .store
        .audit_entries()
        .iter()
        .map(|entry| entry.action())
        .collect()
}

/// The element and the dimensions of the array row `id`, or `None` for another row.
fn array_of(fixture: &Fixture, id: PlcTypeId) -> Option<(PlcTypeId, ArrayDimensions)> {
    let state = fixture.store.state();
    match state.plc_type(id)?.shape() {
        RecordShape::Array {
            element,
            dimensions,
        } => Some((*element, dimensions.clone())),
        _ => None,
    }
}

#[tokio::test]
async fn a_struct_parameter_moves_with_the_mapped_value_limits_and_translations() {
    let fixture = empty_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (11, "ZONE1"),
        (ZONE_1, json!({"setpoint": 1})),
        vec![zone_limit()],
    );
    let before = stored(&fixture, 11);
    let output = run(&fixture, zone_request()).await.unwrap();
    assert_eq!(
        output,
        DraftOutput {
            version: vid(DRAFT_VERSION),
            row_version: 2
        }
    );
    let after = stored(&fixture, 11);
    assert_eq!(after.plc_type, tid(ZONE_2));
    assert_eq!(after.value.as_json(), &json!({"setpoint": 1, "extra": 7}));
    assert_eq!(after.limits, vec![zone_limit()]);
    assert_eq!(after.translations, before.translations);
    assert_eq!(after.id, before.id);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::TypeRevisionRebound);
    assert_eq!(entry.target().entity_type(), "parameter");
    assert_eq!(entry.target().entity_id(), after.id.as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert_eq!(
        entry.before(),
        Some(&json!({
            "plc_type_id": tid(ZONE_1).to_string(),
            "value": {".extra": null}
        }))
    );
    assert_eq!(
        entry.after(),
        Some(&json!({
            "plc_type_id": tid(ZONE_2).to_string(),
            "value": {".extra": 7}
        }))
    );
    assert_eq!(
        fixture.store.state().editors_of(vid(DRAFT_VERSION)),
        vec![actor(Role::ProcessEngineer).user()]
    );
}

#[tokio::test]
async fn a_moved_parameter_writes_its_definition_value_and_limits() {
    let fixture = empty_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (11, "ZONE1"),
        (ZONE_1, json!({"setpoint": 1})),
        vec![zone_limit()],
    );
    let key = stored(&fixture, 11).key;
    run(&fixture, zone_request()).await.unwrap();
    let writes: Vec<_> = fixture
        .store
        .draft_writes()
        .into_iter()
        .map(|write| (write.kind, write.key))
        .collect();
    assert_eq!(
        writes,
        vec![
            (WriteKind::Definition, key),
            (WriteKind::Value, key),
            (WriteKind::Limits, key),
        ]
    );
}

#[tokio::test]
async fn an_array_parameter_creates_the_new_array_row_once_before_the_rebind_entry() {
    let fixture = zones_draft();
    run(&fixture, zones_request()).await.unwrap();
    let rows = arrays_of(&fixture, ZONE_2).await;
    assert_eq!(rows.len(), 1);
    let row = rows.first().unwrap();
    assert_eq!(stored(&fixture, 12).plc_type, row.id());
    let old_dimensions = array_of(&fixture, tid(ZONES_1)).unwrap().1;
    assert_eq!(
        array_of(&fixture, row.id()),
        Some((tid(ZONE_2), old_dimensions))
    );
    assert_eq!(
        actions(&fixture),
        vec![
            AuditAction::DerivedTypeCreated,
            AuditAction::TypeRevisionRebound
        ]
    );
    let entries = fixture.store.audit_entries();
    let created = entries.first().unwrap();
    assert_eq!(created.target().entity_id(), row.id().as_uuid());
}

#[tokio::test]
async fn two_parameters_of_one_array_shape_share_one_new_array_row() {
    let fixture = zones_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (13, "ZONES_B"),
        (ZONES_1, zones_value(4)),
        Vec::new(),
    );
    let input = (1..=4).fold(zones_request(), |input, index| {
        with_value(input, (13, &format!("[{index}].extra")), &json!(7))
    });
    run(&fixture, input).await.unwrap();
    assert_eq!(arrays_of(&fixture, ZONE_2).await.len(), 1);
    assert_eq!(stored(&fixture, 12).plc_type, stored(&fixture, 13).plc_type);
    assert_eq!(
        actions(&fixture),
        vec![
            AuditAction::DerivedTypeCreated,
            AuditAction::TypeRevisionRebound,
            AuditAction::TypeRevisionRebound
        ]
    );
}

#[tokio::test]
async fn two_array_shapes_give_two_rows_and_each_parameter_gets_its_own() {
    let fixture = zones_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (13, "PAIR"),
        (PAIR_1, zones_value(2)),
        Vec::new(),
    );
    let input = (1..=2).fold(zones_request(), |input, index| {
        with_value(input, (13, &format!("[{index}].extra")), &json!(7))
    });
    run(&fixture, input).await.unwrap();
    assert_eq!(arrays_of(&fixture, ZONE_2).await.len(), 2);
    let zones = stored(&fixture, 12).plc_type;
    let pair = stored(&fixture, 13).plc_type;
    assert_ne!(zones, pair);
    let zones_dimensions = array_of(&fixture, tid(ZONES_1)).unwrap().1;
    let pair_dimensions = array_of(&fixture, tid(PAIR_1)).unwrap().1;
    assert_eq!(
        array_of(&fixture, zones),
        Some((tid(ZONE_2), zones_dimensions))
    );
    assert_eq!(
        array_of(&fixture, pair),
        Some((tid(ZONE_2), pair_dimensions))
    );
    assert_eq!(
        actions(&fixture),
        vec![
            AuditAction::DerivedTypeCreated,
            AuditAction::DerivedTypeCreated,
            AuditAction::TypeRevisionRebound,
            AuditAction::TypeRevisionRebound
        ]
    );
}

#[tokio::test]
async fn a_larger_revision_that_overlaps_the_next_address_gives_rcp_022_and_writes_nothing() {
    let fixture = s7_zones_draft();
    let rows_before = machine_rows(&fixture).await.len();
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, zones_request()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_022);
    before.assert_unchanged(&fixture);
    assert_eq!(machine_rows(&fixture).await.len(), rows_before);
}

#[tokio::test]
async fn a_rebind_above_the_leaf_budget_gives_rcp_021_and_writes_nothing() {
    let fixture = near_budget_draft();
    let rows_before = machine_rows(&fixture).await.len();
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, big_request()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_021);
    before.assert_unchanged(&fixture);
    assert_eq!(machine_rows(&fixture).await.len(), rows_before);
    assert!(arrays_of(&fixture, BIG_2).await.is_empty());
    assert_eq!(stored(&fixture, 40).plc_type, tid(BIG_ARRAY));
}

#[tokio::test]
async fn a_failed_plan_creates_no_array_row() {
    let fixture = zones_draft();
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, request(ZONE_1, ZONE_2)).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_028);
    before.assert_unchanged(&fixture);
    assert!(arrays_of(&fixture, ZONE_2).await.is_empty());
}
