//! Tests of the rule "no array of an array on S7-300" in `DefineUdt` (ADR-0177).

use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;

use super::{assert_member_refused, fixture, input, run};
use crate::plant::define_udt::DefineUdtMode;

const NESTED_BOOL: &str = "ARRAY[1..3] OF ARRAY[1..3] OF BOOL";

#[tokio::test]
async fn adr_0177_a_member_array_of_an_array_on_s7_300_gives_plt_006_and_creates_no_type() {
    let fixture = fixture(PlcPlatform::S7_300);
    let before = fixture.store.state().types.len();
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Grid", &[("cells", NESTED_BOOL)]),
    )
    .await
    .unwrap_err();
    assert_member_refused(&error, ("ST_Grid", "cells"), PlcPlatform::S7_300);
    assert_eq!(fixture.store.state().types.len(), before);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn adr_0177_a_member_array_with_two_dimensions_on_s7_300_succeeds() {
    let fixture = fixture(PlcPlatform::S7_300);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Grid",
            &[("cells", "ARRAY[1..3,1..3] OF BOOL")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}

#[tokio::test]
async fn adr_0177_a_member_array_of_an_array_on_codesys_iec_succeeds() {
    let fixture = fixture(PlcPlatform::CodesysIec);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Grid",
            &[("cells", NESTED_BOOL), ("count", "INT")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}
