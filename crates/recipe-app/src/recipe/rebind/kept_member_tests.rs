//! Tests of the writes and the audit entry of a rebind that keeps every member
//! (ADR-0146 (h), (i), design D5).
//!
//! The member `label` has two rows of one shape: the row `STRING(80)` of the machine and the seed
//! row `STRING` of `CodesysIec`. The rebind keeps the value and the limit of `LABEL_1`.

use std::collections::BTreeSet;

use recipe_domain::audit::AuditAction;
use recipe_domain::plant::{
    LeafPath, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape,
};
use recipe_domain::recipe::StoredLimit;
use serde_json::json;

use super::fixtures::{
    array_row, error_lines, primitive_id, request, run, seed_parameter, stored, typed_struct_row,
};
use crate::memory::fixtures::{Fixture, mid, tid, vid};
use crate::memory::seed_type_id;
use crate::recipe::DraftOutput;
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot};

const PLATFORM: PlcPlatform = PlcPlatform::CodesysIec;
/// `ST_Text@1`, the revision with the member `label` of type `STRING(80)`.
const TEXT_1: u128 = 940;
/// `ST_Text@2`, the revision with the member `label` of the seed type `STRING`.
const TEXT_2: u128 = 941;
/// The row `STRING(80)` of the machine 1.
const MACHINE_STRING: u128 = 942;
/// The parameter number of `LABEL_1`.
const LABEL: u128 = 21;

/// A string row of the machine 1 on `CodesysIec` with the given length.
///
/// The size follows `n + 2` bytes of a `STRING(n)` row (`string_length.rs`).
fn machine_string(id: u128, length: u16) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: Primitive::String.leaf_class(),
            size_bytes: u32::from(length) + 2,
            string_length: Some(length),
        },
    })
    .unwrap()
}

/// The seed row `STRING` of `CodesysIec`. The memory fixture stores it.
fn seed_string() -> PlcTypeId {
    seed_type_id(Primitive::String, Some(80)).unwrap()
}

fn max_length_40() -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(".label").unwrap(),
        min: None,
        max: None,
        max_length: Some(40),
    }
}

/// A `CodesysIec` draft with `LABEL_1` of type `ST_Text@1`.
///
/// `ST_Text@1` has `label` of the row `member_1`. `ST_Text@2` has `label` of the row `member_2`.
/// `LABEL_1` holds `"ABC"` and the limit `max_length` 40 on `.label`.
fn label_draft(member_1: PlcTypeId, member_2: PlcTypeId, rows: &[PlcType]) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    for row in rows {
        fixture.store.seed_type(row);
    }
    fixture.store.seed_type(&typed_struct_row(
        PLATFORM,
        (TEXT_1, "ST_Text", 1),
        None,
        &[("label", member_1)],
    ));
    fixture.store.seed_type(&typed_struct_row(
        PLATFORM,
        (TEXT_2, "ST_Text", 2),
        Some(TEXT_1),
        &[("label", member_2)],
    ));
    seed_parameter(
        &fixture,
        PLATFORM,
        (LABEL, "LABEL_1"),
        (TEXT_1, json!({"label": "ABC"})),
        vec![max_length_40()],
    );
    fixture
}

/// The draft of the rebind that keeps every member: `STRING(80)` and the seed `STRING`.
fn one_shape_draft() -> Fixture {
    label_draft(
        tid(MACHINE_STRING),
        seed_string(),
        &[machine_string(MACHINE_STRING, 80)],
    )
}

fn actions(fixture: &Fixture) -> Vec<AuditAction> {
    fixture
        .store
        .audit_entries()
        .iter()
        .map(|entry| entry.action())
        .collect()
}

#[tokio::test]
async fn a_rebind_across_two_rows_of_one_shape_keeps_the_value_and_the_limit() {
    let fixture = one_shape_draft();
    let before = stored(&fixture, LABEL);

    let output = run(&fixture, request(TEXT_1, TEXT_2)).await.unwrap();

    assert_eq!(
        output,
        DraftOutput {
            version: vid(DRAFT_VERSION),
            row_version: 2
        }
    );
    let after = stored(&fixture, LABEL);
    assert_eq!(after.plc_type, tid(TEXT_2));
    assert_eq!(after.value.as_json(), &json!({"label": "ABC"}));
    assert_eq!(after.value, before.value);
    assert_eq!(after.limits, vec![max_length_40()]);
    assert_eq!(after.id, before.id);
}

#[tokio::test]
async fn a_rebind_that_keeps_every_member_writes_one_entry_with_the_type_only() {
    let fixture = one_shape_draft();

    run(&fixture, request(TEXT_1, TEXT_2)).await.unwrap();

    assert_eq!(actions(&fixture), vec![AuditAction::TypeRevisionRebound]);
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    let before = entry.before().unwrap();
    let after = entry.after().unwrap();
    assert_eq!(before, &json!({"plc_type_id": tid(TEXT_1).to_string()}));
    assert_eq!(after, &json!({"plc_type_id": tid(TEXT_2).to_string()}));
    for diff in [before, after] {
        assert!(diff.get("value").is_none());
        assert!(diff.get("limits").is_none());
    }
}

/// `ST_Zone@1` and `ST_Zone@2` come from the fixtures. The member of both is `setpoint`.
const ZONE_STRUCT_1: u128 = 950;
const ZONE_STRUCT_2: u128 = 951;
/// `ARRAY[1..2] OF ST_Zone@1`, the inner level of `ZONES`.
const INNER_ZONES: u128 = 952;
/// `ARRAY[1..2] OF ARRAY[1..2] OF ST_Zone@1`, the type of `ZONES`.
const OUTER_ZONES: u128 = 953;
/// The parameter number of `ZONES`.
const ZONES: u128 = 22;

/// An `S7_1500` draft with `ZONES` of the stored type `ARRAY[1..2] OF ARRAY[1..2] OF ST_Zone@1`.
///
/// ADR-0148 (f): a stored array of an array stays valid, so the rebind must keep it.
fn nested_zones_draft() -> Fixture {
    let platform = PlcPlatform::S7_1500;
    let int = primitive_id(Primitive::Int);
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let rows = [
        typed_struct_row(
            platform,
            (ZONE_STRUCT_1, "ST_Zone", 1),
            None,
            &[("setpoint", int)],
        ),
        typed_struct_row(
            platform,
            (ZONE_STRUCT_2, "ST_Zone", 2),
            Some(ZONE_STRUCT_1),
            &[("setpoint", int)],
        ),
        array_row(platform, INNER_ZONES, tid(ZONE_STRUCT_1), 2),
        array_row(platform, OUTER_ZONES, tid(INNER_ZONES), 2),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
    seed_parameter(
        &fixture,
        platform,
        (ZONES, "ZONES"),
        (
            OUTER_ZONES,
            json!([[{"setpoint": 1}, {"setpoint": 2}], [{"setpoint": 3}, {"setpoint": 4}]]),
        ),
        Vec::new(),
    );
    fixture
}

/// The element of the array row `id`, or `None` for another row.
fn array_element(fixture: &Fixture, id: PlcTypeId) -> Option<PlcTypeId> {
    match fixture.store.state().plc_type(id)?.shape() {
        RecordShape::Array { element, .. } => Some(*element),
        _ => None,
    }
}

#[tokio::test]
async fn a_rebind_keeps_a_stored_array_of_an_array_on_s7_1500() {
    let fixture = nested_zones_draft();
    let before = stored(&fixture, ZONES);

    let output = run(&fixture, request(ZONE_STRUCT_1, ZONE_STRUCT_2))
        .await
        .unwrap();

    assert_eq!(output.row_version, 2);
    let after = stored(&fixture, ZONES);
    assert_eq!(after.value, before.value);
    assert_ne!(after.plc_type, tid(OUTER_ZONES));
    let inner = array_element(&fixture, after.plc_type).unwrap();
    assert_ne!(inner, tid(INNER_ZONES));
    assert_eq!(array_element(&fixture, inner), Some(tid(ZONE_STRUCT_2)));
}

#[tokio::test]
async fn a_longer_string_gives_rcp_028_and_rcp_029_and_writes_nothing() {
    let fixture = label_draft(
        tid(MACHINE_STRING),
        tid(MACHINE_STRING + 10),
        &[
            machine_string(MACHINE_STRING, 80),
            machine_string(MACHINE_STRING + 10, 81),
        ],
    );
    let before = DraftSnapshot::capture(&fixture);

    let error = run(&fixture, request(TEXT_1, TEXT_2)).await.unwrap_err();

    assert_eq!(
        error_lines(&error),
        ["E-RCP-028 '.label'", "E-RCP-029 '.label'"]
    );
    before.assert_unchanged(&fixture);
    assert_eq!(stored(&fixture, LABEL).plc_type, tid(TEXT_1));
}
