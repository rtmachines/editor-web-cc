//! Tests of `RebindTypeRevision` for the rule "Reserved names on CODESYS and TwinCAT"
//! (ADR-0203 (k)).

use recipe_domain::plant::{PlcPlatform, Primitive};
use serde_json::json;

use super::fixtures::{primitive_id, request, run, seed_parameter, typed_struct_row};
use crate::memory::fixtures::{Fixture, tid};
use crate::recipe::test_support::stored_parameters;

/// `VERSION@1` and `VERSION@2`, and `ST_Coil@1` and `ST_Coil@2` with the keyword member `END_IF`.
const VERSION_1: u128 = 991;
const VERSION_2: u128 = 992;
const COIL_1: u128 = 993;
const COIL_2: u128 = 994;

/// A draft on `platform` with two revisions of a reserved struct and of a keyword member struct.
fn draft_on(platform: PlcPlatform) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let int = primitive_id(Primitive::Int);
    let rows = [
        typed_struct_row(platform, (VERSION_1, "VERSION", 1), None, &[("a", int)]),
        typed_struct_row(
            platform,
            (VERSION_2, "VERSION", 2),
            Some(VERSION_1),
            &[("a", int)],
        ),
        typed_struct_row(
            platform,
            (COIL_1, "ST_Coil", 1),
            None,
            &[("END_IF", int), ("S", int)],
        ),
        typed_struct_row(
            platform,
            (COIL_2, "ST_Coil", 2),
            Some(COIL_1),
            &[("END_IF", int), ("S", int)],
        ),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
    fixture
}

#[tokio::test]
async fn adr_0203_k_a_rebind_keeps_a_stored_reserved_struct_on_codesys_and_twincat() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = draft_on(platform);
        seed_parameter(
            &fixture,
            platform,
            (11, "VER"),
            (VERSION_1, json!({"a": 1})),
            Vec::new(),
        );
        run(&fixture, request(VERSION_1, VERSION_2)).await.unwrap();
        let stored = stored_parameters(&fixture);
        assert_eq!(
            stored.first().unwrap().plc_type,
            tid(VERSION_2),
            "{platform:?}"
        );
    }
}

#[tokio::test]
async fn adr_0203_k_a_rebind_keeps_a_stored_keyword_member_on_codesys_and_twincat() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = draft_on(platform);
        seed_parameter(
            &fixture,
            platform,
            (12, "COIL"),
            (COIL_1, json!({"END_IF": 1, "S": 2})),
            Vec::new(),
        );
        run(&fixture, request(COIL_1, COIL_2)).await.unwrap();
        let stored = stored_parameters(&fixture);
        assert_eq!(
            stored.first().unwrap().plc_type,
            tid(COIL_2),
            "{platform:?}"
        );
    }
}
