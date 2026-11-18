//! Tests that a broken set of type rows gives `E-DB-008` in the draft commands (ADR-0195).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::Primitive;
use recipe_domain::recipe::ValueInput;

use crate::memory::fixtures::{actor, meta, reference};
use crate::memory::seed_type_id;
use crate::recipe::test_support::{DraftSnapshot, seed_speed_draft, speed_key};
use crate::recipe::{SetParameterValue, SetParameterValueInput};

#[tokio::test]
async fn a_duplicate_type_row_gives_e_db_008_in_set_parameter_value_and_commits_nothing() {
    let fixture = crate::memory::fixtures::Fixture::new();
    seed_speed_draft(&fixture);
    let int = seed_type_id(Primitive::Int, None).unwrap();
    fixture.store.duplicate_type_row_on_call(1, int);
    let before = DraftSnapshot::capture(&fixture);
    let request = SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: speed_key(),
        value: ValueInput::Json("130".to_owned()),
    };

    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008, "{error:?}");
    before.assert_unchanged(&fixture);
}
