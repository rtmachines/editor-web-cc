//! Tests of the read paths that take the STRING encoding of the machine (ADR-0202 (m)).

use std::collections::BTreeSet;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape, StringEncoding, StringKind,
    StringLength,
};
use recipe_domain::recipe::{Ordinal, StoredParameter, StoredValue};
use serde_json::json;

use super::parameter_form_tests::{form_as, input};
use super::parameters::{ListParameters, ListParametersInput};
use super::test_world::seed_users;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, pkey, reference, tid, uid, utf8_machine, vid,
};
use crate::recipe::{CreateVersion, CreateVersionInput};

/// The platform of `utf8_machine`.
const PLATFORM: PlcPlatform = PlcPlatform::CodesysIec;

/// The value of the stored parameter. It has 4 characters and 7 UTF-8 bytes.
const VALUE: &str = "Łódź";

/// The identifier of the `STRING(10)` row of the machine.
const STRING_TYPE: u128 = 951;

/// The machine row `STRING(10)` of `utf8_machine`.
fn string_row() -> PlcType {
    let length = StringLength::parse(StringKind::String, PLATFORM, 10).unwrap();
    PlcType::try_from(PlcTypeRecord {
        id: tid(STRING_TYPE),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: length.size_bytes(StringKind::String).unwrap(),
            string_length: Some(length.value()),
        },
    })
    .unwrap()
}

/// A `Utf8` machine `M1` with the retired version 1 of `R-1`.
///
/// Version 1 holds the parameter `LABEL` (key 1) of type `STRING(10)` with the value `"Łódź"`.
/// No use case can write that value before task 11.6, so the store gets it directly.
fn utf8_world() -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = utf8_machine(1, "M1");
    fixture.store.seed_machine(&machine);
    fixture.seed_retired(&machine, 1, "R-1");
    fixture.store.seed_type(&string_row());
    let label = StoredParameter {
        plc_type: tid(STRING_TYPE),
        value: StoredValue::from(json!(VALUE)),
        limits: Vec::new(),
        ordinal: Ordinal::parse(0).unwrap(),
        ..parameter_on(PLATFORM, 1, "LABEL", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(101), &label);
    fixture
}

#[tokio::test]
async fn adr_0202_machine_read_paths_carry_utf8() {
    let fixture = utf8_world();

    // The parameter query reads the stored value with the encoding of the machine.
    let list = ListParameters::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            ListParametersInput {
                reference: reference("R-1"),
                number: 1,
                unit: None,
                page: 0,
                lang: None,
            },
        )
        .await
        .unwrap();
    assert!(list.rule_faults.is_empty(), "{:?}", list.rule_faults);
    assert_eq!(
        list.value_texts.get(&pkey(1)).map(String::as_str),
        Some(VALUE)
    );

    // CreateVersion accepts the stored value.
    let created = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            CreateVersionInput {
                reference: reference("R-1"),
                based_on: Some(1),
                change_note: crate::memory::fixtures::long_text("Next version"),
            },
        )
        .await
        .unwrap();
    assert_eq!(created.number, 3);

    // The add form hands the encoding of the machine to the preview tree.
    let discarded = form_as(
        &utf8_world_with_draft(),
        Role::ProcessEngineer,
        input(None, Some("STRING(10)"), None),
    )
    .await
    .unwrap();
    let preview = discarded.preview.as_ref().unwrap();
    assert_eq!(preview.string_encoding(), StringEncoding::Utf8);
}

/// A `Utf8` machine `M1` with the live draft `R-1` that the add form needs.
fn utf8_world_with_draft() -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = utf8_machine(1, "M1");
    fixture.store.seed_machine(&machine);
    fixture.seed_draft(&machine, 1, "R-1");
    fixture
}
