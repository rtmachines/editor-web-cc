//! Tests of `RebindTypeRevision` for the rule "Two members of a struct on CODESYS"
//! (ADR-0204 (e)).

use recipe_domain::plant::{PlcPlatform, Primitive};
use serde_json::json;

use super::fixtures::{primitive_id, request, run, seed_parameter, typed_struct_row};
use crate::memory::fixtures::{Fixture, tid};
use crate::recipe::test_support::stored_parameters;

/// `ST_One@1` and `ST_One@2`, two revisions of a struct with one member.
const ONE_1: u128 = 995;
const ONE_2: u128 = 996;

#[tokio::test]
async fn adr_0204_e_a_rebind_keeps_a_stored_one_member_struct_on_codesys() {
    let platform = PlcPlatform::CodesysIec;
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let int = primitive_id(Primitive::Int);
    for row in [
        typed_struct_row(platform, (ONE_1, "ST_One", 1), None, &[("a", int)]),
        typed_struct_row(platform, (ONE_2, "ST_One", 2), Some(ONE_1), &[("a", int)]),
    ] {
        fixture.store.seed_type(&row);
    }
    seed_parameter(
        &fixture,
        platform,
        (11, "ONE"),
        (ONE_1, json!({"a": 1})),
        Vec::new(),
    );
    run(&fixture, request(ONE_1, ONE_2)).await.unwrap();
    let stored = stored_parameters(&fixture);
    assert_eq!(stored.first().unwrap().plc_type, tid(ONE_2));
}
