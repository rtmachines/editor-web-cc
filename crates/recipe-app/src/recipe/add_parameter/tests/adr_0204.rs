//! Tests of the rule "Two members of a struct on CODESYS" in `AddParameter` (ADR-0204 (c)).

use recipe_domain::plant::{DerivedTypeReason, PlantError, PlcPlatform, Primitive};

use super::{AddParameterInput, assert_nothing_written, empty_draft, input, run};
use crate::error::AppError;
use crate::memory::fixtures::Fixture;
use crate::recipe::rebind::fixtures::{primitive_id, typed_struct_row};
use crate::recipe::test_support::stored_parameters;

/// Store `ST_One@1` with the member `a` and `ST_Pair@1` with the members `a` and `b`.
fn seed_structs(fixture: &Fixture, platform: PlcPlatform) {
    let int = primitive_id(Primitive::Int);
    let rows = [
        typed_struct_row(platform, (982, "ST_One", 1), None, &[("a", int)]),
        typed_struct_row(
            platform,
            (983, "ST_Pair", 1),
            None,
            &[("a", int), ("b", int)],
        ),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
}

fn reason_of(error: &AppError) -> Option<DerivedTypeReason> {
    match error {
        AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. }) => Some(*reason),
        _ => None,
    }
}

fn one_input(type_text: &str, value: &str) -> AddParameterInput {
    input(type_text, value, Vec::new())
}

#[tokio::test]
async fn adr_0204_c3_a_stored_one_member_struct_on_codesys_gives_plt_013() {
    for (type_text, value) in [
        ("ST_One@1", "{\"a\":1}"),
        (
            "ARRAY[1..4] OF ST_One@1",
            "[{\"a\":1},{\"a\":2},{\"a\":3},{\"a\":4}]",
        ),
    ] {
        let fixture = empty_draft(PlcPlatform::CodesysIec);
        seed_structs(&fixture, PlcPlatform::CodesysIec);
        let error = run(&fixture, one_input(type_text, value))
            .await
            .unwrap_err();
        assert_eq!(
            reason_of(&error),
            Some(DerivedTypeReason::TooFewMembers),
            "{type_text}: {error:?}"
        );
        assert_nothing_written(&fixture);
        assert!(stored_parameters(&fixture).is_empty());
    }
}

#[tokio::test]
async fn adr_0204_c3_a_stored_one_member_struct_on_twincat_adds_the_parameter() {
    let fixture = empty_draft(PlcPlatform::TwinCat3);
    seed_structs(&fixture, PlcPlatform::TwinCat3);
    run(&fixture, one_input("ST_One@1", "{\"a\":1}"))
        .await
        .unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

#[tokio::test]
async fn adr_0204_c3_a_stored_two_member_struct_on_codesys_adds_the_parameter() {
    let fixture = empty_draft(PlcPlatform::CodesysIec);
    seed_structs(&fixture, PlcPlatform::CodesysIec);
    run(&fixture, one_input("ST_Pair@1", "{\"a\":1,\"b\":2}"))
        .await
        .unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}
