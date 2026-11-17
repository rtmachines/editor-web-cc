//! Tests of `CreateVersion` for the rules "array bounds of a parameter with an S7 address"
//! (ADR-0178) and "BOOL array with three dimensions and an S7 address" (ADR-0187): a base with
//! a stored fault stays copyable, because the rules run in the commands and at approval only.

use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, PlcTypeId, S7Address};
use recipe_domain::recipe::{StoredLimit, StoredParameter, StoredValue};
use serde_json::Value;

use super::tests::{parameters_of, write_journal};
use super::{CreateVersion, CreateVersionInput};
use crate::memory::fixtures::{Fixture, actor, long_text, meta, parameter_on, reference, tid, uid};
use crate::recipe::test_support::{
    CUBE_ARRAY, WIDE_ARRAY, bool_cube, byte_elements_limit, seed_standard_access_rows,
    wide_array_value,
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

/// Run `CreateVersion` from the approved version 2 of a base that holds `parameter`. Assert that
/// the new draft 3 holds the same parameter.
async fn assert_base_is_copied(parameter: StoredParameter) {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    seed_standard_access_rows(&fixture, PlcPlatform::S7_300);
    let (_, _, approved) = fixture.seed_retired(&machine, 1, "R-1");
    fixture.store.seed_parameter(approved, &parameter);
    let output = CreateVersion::new(fixture.context())
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
        .unwrap();
    assert_eq!(output.number, 3);
    assert_eq!(fixture.store.journal(), write_journal(&[102]));
    let copy = parameters_of(&fixture, output.version).await;
    assert_eq!(copy.len(), 1);
    let copied = copy.first().unwrap();
    assert_eq!(copied.tag, parameter.tag);
    assert_eq!(copied.plc_type, parameter.plc_type);
    assert_eq!(copied.value, parameter.value);
    assert_eq!(copied.s7_address, parameter.s7_address);
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn adr_0178_a_base_with_a_stored_bound_above_32767_is_copied_to_a_new_draft() {
    let parameter = buf_parameter(
        tid(WIDE_ARRAY),
        wide_array_value(),
        vec![byte_elements_limit()],
    );
    assert_base_is_copied(parameter).await;
}

#[tokio::test]
async fn adr_0187_a_base_with_a_stored_bool_array_with_three_dimensions_is_copied_to_a_new_draft() {
    let parameter = buf_parameter(tid(CUBE_ARRAY), bool_cube(2), Vec::new());
    assert_base_is_copied(parameter).await;
}
