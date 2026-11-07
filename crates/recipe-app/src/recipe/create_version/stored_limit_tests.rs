//! Tests of `CreateVersion` with a stored limit that breaks a count rule (ADR-0120 (b)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, PlcTypeId, Primitive};
use recipe_domain::recipe::{RecipeError, SnapshotItem, StoredValue};
use serde_json::json;

use super::tests::{max_limit, s7_parameter};
use super::{CreateVersion, CreateVersionInput};
use crate::error::AppError;
use crate::memory::JournalEntry;
use crate::memory::fixtures::{Fixture, actor, long_text, meta, reference, rid};
use crate::ports::Isolation;

/// The identifier of the `S5TIME` row that the test adds.
const S5TIME_TYPE: u128 = 951;

/// Store an `S5TIME` row for `S7_1500`, because the seed has no `S5TIME` row.
fn seed_s5time_type(fixture: &Fixture) -> PlcTypeId {
    fixture.seed_primitive_type(S5TIME_TYPE, PlcPlatform::S7_1500, Primitive::S5Time)
}

/// Return the codes of the items of an `E-RCP-020` error.
fn item_codes(error: &AppError) -> Vec<ErrorCode> {
    match error {
        AppError::Recipe(RecipeError::ValidationFailed { items }) => {
            items.iter().map(SnapshotItem::code).collect()
        }
        _ => Vec::new(),
    }
}

#[tokio::test]
async fn an_s5time_limit_between_two_steps_gives_rcp_020_and_no_draft() {
    // ADR-0120 (b): the `S5TIME` step below 9 990 ms is 10 ms, so the stored bound 15 fails.
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let time = (seed_s5time_type(&fixture), StoredValue::from(json!(0)));
    let parameter = s7_parameter(1, "HOLD", time, (vec![max_limit("15")], "DB1.DBW0"));
    let (_, _, approved) = fixture.seed_retired(&machine, 1, "R-1");
    fixture.store.seed_parameter(approved, &parameter);

    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            CreateVersionInput {
                reference: reference("R-1"),
                based_on: Some(2),
                change_note: long_text("Next version"),
            },
        )
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_020);
    assert_eq!(item_codes(&error), vec![ErrorCode::RCP_006]);
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
    let state = fixture.store.state();
    assert!(state.recipe(rid(1)).unwrap().version(3).is_none());
}
