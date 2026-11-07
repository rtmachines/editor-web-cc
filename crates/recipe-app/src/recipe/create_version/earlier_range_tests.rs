//! Tests of `CreateVersion` with a stored value or address that the current rules refuse
//! (ADR-0139 (f), ADR-0140 (e)).

use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, Primitive};
use recipe_domain::recipe::{StoredLimit, StoredParameter, StoredValue};
use serde_json::json;

use super::tests::{parameters_of, s7_parameter, write_journal};
use super::{CreateVersion, CreateVersionInput};
use crate::memory::fixtures::{Fixture, actor, long_text, meta, parameter_on, reference, uid};
use crate::memory::seed_type_id;

/// The identifier of the `LDT` row that the test adds.
const LDT_TYPE: u128 = 952;

/// The count of 1 January 2300 in nanoseconds. It is after the end of the current range.
const LDT_AFTER_CURRENT_RANGE: u64 = 10_413_792_000_000_000_000;

#[tokio::test]
async fn a_twincat3_base_with_an_ldt_after_the_current_range_is_copied_unchanged() {
    // ADR-0139 (f): the stored count is above 2^63-1, so the current rules refuse it.
    // The earlier rule set ends at 2^64-1, so `CreateVersion` copies the parameter unchanged.
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let ldt = fixture.seed_primitive_type(LDT_TYPE, PlcPlatform::TwinCat3, Primitive::Ldt);
    let parameter = StoredParameter {
        plc_type: ldt,
        value: StoredValue::from(json!(LDT_AFTER_CURRENT_RANGE)),
        limits: Vec::<StoredLimit>::new(),
        ..parameter_on(PlcPlatform::TwinCat3, 1, "DUE", uid(102), 0)
    };
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
    assert_eq!(copied.plc_type, parameter.plc_type);
    assert_eq!(copied.value, parameter.value);
    assert_ne!(copied.id, parameter.id);
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn an_s7_1500_base_with_a_data_block_above_59999_is_copied_unchanged() {
    // ADR-0140 (e): `SPEED` is at DB60000.DBW0, so the current rules refuse the address.
    // The earlier rule set has no data block rule, so `CreateVersion` copies the parameter unchanged.
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let parameter = s7_parameter(
        1,
        "SPEED",
        (int, StoredValue::from(json!(120))),
        (Vec::<StoredLimit>::new(), "DB60000.DBW0"),
    );
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
    assert_eq!(copied.s7_address, parameter.s7_address);
    assert_eq!(copied.plc_type, parameter.plc_type);
    assert_eq!(copied.value, parameter.value);
    assert_ne!(copied.id, parameter.id);
    assert_eq!(fixture.store.audit_entries().len(), 1);
}
