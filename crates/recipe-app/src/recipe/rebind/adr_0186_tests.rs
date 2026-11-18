//! Tests of `RebindTypeRevision` for the rule "Siemens reserved struct names on S7" (ADR-0186 (i)).

use recipe_domain::plant::{PlcPlatform, Primitive};
use serde_json::json;

use super::fixtures::{array_row, primitive_id, request, run, seed_parameter, typed_struct_row};
use crate::memory::fixtures::{Fixture, tid};

/// `PID_Compact@1` with the member `a`.
const PID_1: u128 = 971;
/// `PID_Compact@2` with the member `a`.
const PID_2: u128 = 972;
/// `ARRAY[1..2] OF PID_Compact@1`.
const PIDS_1: u128 = 973;

/// A draft on S7-1500 with the stored structs `PID_Compact@1` and `PID_Compact@2`.
fn pid_draft() -> Fixture {
    let platform = PlcPlatform::S7_1500;
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let int = primitive_id(Primitive::Int);
    let rows = [
        typed_struct_row(platform, (PID_1, "PID_Compact", 1), None, &[("a", int)]),
        typed_struct_row(
            platform,
            (PID_2, "PID_Compact", 2),
            Some(PID_1),
            &[("a", int)],
        ),
        array_row(platform, PIDS_1, tid(PID_1), 2),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
    fixture
}

#[tokio::test]
async fn adr_0186_a_rebind_of_a_stored_siemens_struct_moves_the_parameter() {
    let fixture = pid_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::S7_1500,
        (11, "PID"),
        (PID_1, json!({"a": 1})),
        Vec::new(),
    );
    run(&fixture, request(PID_1, PID_2)).await.unwrap();
}

#[tokio::test]
async fn adr_0186_a_rebind_of_an_array_of_a_stored_siemens_struct_moves_the_parameter() {
    let fixture = pid_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::S7_1500,
        (12, "PIDS"),
        (PIDS_1, json!([{"a": 1}, {"a": 2}])),
        Vec::new(),
    );
    run(&fixture, request(PID_1, PID_2)).await.unwrap();
}
