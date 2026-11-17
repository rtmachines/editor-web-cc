//! Tests of `ApproveVersion` for the rules "array bounds of a parameter with an S7 address"
//! (ADR-0178) and "BOOL array with three dimensions and an S7 address" (ADR-0187): a stored
//! fault blocks the approval, and the draft stays a draft.

use recipe_domain::plant::{PlcPlatform, PlcTypeId, S7Address};
use recipe_domain::recipe::{
    S7AccessReason, StoredLimit, StoredParameter, StoredValue, VersionState,
};
use serde_json::Value;

use super::{only_the_read, run, seed_with};
use crate::memory::fixtures::{Fixture, parameter_on, rid, tid, uid};
use crate::recipe::test_support::{
    CUBE_ARRAY, WIDE_ARRAY, assert_not_addressable, bool_cube, byte_elements_limit,
    seed_standard_access_rows, wide_array_value,
};

/// The parameter `BUF` of `plc_type` at `DB1.DBB0`, stored before the two rules.
fn buf_parameter(plc_type: PlcTypeId, value: Value, limits: Vec<StoredLimit>) -> StoredParameter {
    StoredParameter {
        plc_type,
        value: StoredValue::from(value),
        limits,
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        ..parameter_on(PlcPlatform::S7_300, 1, "BUF", uid(102), 0)
    }
}

/// Assert that the approval of the draft of `parameter` gives `E-RCP-058` and writes nothing.
async fn assert_approval_refused(parameter: StoredParameter, reason: S7AccessReason) {
    let fixture = Fixture::new();
    seed_standard_access_rows(&fixture, PlcPlatform::S7_300);
    seed_with(&fixture, PlcPlatform::S7_300, &[parameter]);
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_not_addressable(&error, "BUF", reason);
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
    let state = fixture.store.state();
    let version = state.recipe(rid(1)).unwrap().version(1).unwrap().state();
    assert_eq!(version, VersionState::LiveDraft);
}

#[tokio::test]
async fn adr_0178_a_stored_bound_above_32767_with_an_address_blocks_the_approval() {
    let parameter = buf_parameter(
        tid(WIDE_ARRAY),
        wide_array_value(),
        vec![byte_elements_limit()],
    );
    assert_approval_refused(parameter, S7AccessReason::ArrayBoundOutsideInt).await;
}

#[tokio::test]
async fn adr_0187_a_stored_bool_array_with_three_dimensions_and_an_address_blocks_the_approval() {
    let parameter = buf_parameter(tid(CUBE_ARRAY), bool_cube(2), Vec::new());
    assert_approval_refused(parameter, S7AccessReason::BoolArrayWithThreeDimensions).await;
}

#[tokio::test]
async fn adr_0178_a_stored_bound_above_32767_without_an_address_stays_approvable() {
    let fixture = Fixture::new();
    let platform = PlcPlatform::S7_1500;
    seed_standard_access_rows(&fixture, platform);
    let parameter = StoredParameter {
        s7_address: None,
        ..buf_parameter(
            tid(WIDE_ARRAY),
            wide_array_value(),
            vec![byte_elements_limit()],
        )
    };
    seed_with(&fixture, platform, &[parameter]);
    let outcome = run(&fixture, 1, 1, false).await.unwrap();
    assert!(
        matches!(
            outcome,
            recipe_domain::recipe::ApprovalOutcome::Approved { .. }
        ),
        "{outcome:?}"
    );
}
