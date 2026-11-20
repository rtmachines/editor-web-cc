//! Tests of the rule "Vendor aliases of type names" in `UpdateParameterDefinition` (ADR-0206).

use recipe_domain::plant::{
    DerivedTypeReason, PlantError, PlcPlatform, Primitive, RuleSet, TypeExpr,
};

use super::requests::{run, speed_request, with_retype, with_type};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, parameter_on, uid, vid};
use crate::recipe::rebind::fixtures::primitive_id;
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot, stored_parameters};

/// A draft on `CodesysIec` with the `INT` parameter `SPEED`.
fn draft_with_int_speed() -> Fixture {
    let fixture = Fixture::new();
    fixture.store.seed_time_primitive_types();
    let machine = fixture.seed_machine_on(PlcPlatform::CodesysIec, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let speed = parameter_on(PlcPlatform::CodesysIec, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    fixture
}

#[tokio::test]
async fn adr_0206_an_edit_with_the_alias_tod_stores_time_of_day() {
    let fixture = draft_with_int_speed();
    let request = with_retype(
        with_type(speed_request(), "TOD"),
        "\"TOD#06:00:00\"",
        Vec::new(),
    );

    run(&fixture, request).await.unwrap();

    assert_eq!(
        stored_parameters(&fixture).first().unwrap().plc_type,
        primitive_id(Primitive::TimeOfDay)
    );
}

#[tokio::test]
async fn adr_0206_an_edit_with_an_alias_and_a_revision_number_gives_plt_013_with_reserved_name() {
    let fixture = draft_with_int_speed();
    let before = DraftSnapshot::capture(&fixture);
    let request = with_retype(
        super::UpdateParameterDefinitionInput {
            type_expr: TypeExpr::from_stored("DT@1", RuleSet::Earlier).unwrap(),
            ..speed_request()
        },
        "{\"a\":1,\"b\":2}",
        Vec::new(),
    );

    let error = run(&fixture, request).await.unwrap_err();

    assert!(
        matches!(
            error,
            AppError::Plant(PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::ReservedName,
                ..
            })
        ),
        "{error:?}"
    );
    before.assert_unchanged(&fixture);
    assert!(fixture.store.audit_entries().is_empty());
}
