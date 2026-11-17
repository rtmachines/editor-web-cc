//! Tests of the rule "no array of an array on S7-300" in `AddParameter` (ADR-0177).

use recipe_domain::plant::{PlcPlatform, S7Address};

use super::{
    NESTED, NESTED_VALUE, assert_nothing_written, assert_refuses_nesting, byte_draft, input, run,
    stored_parameters,
};

#[tokio::test]
async fn adr_0177_an_array_of_an_array_on_s7_300_gives_plt_013_and_creates_no_type() {
    let fixture = byte_draft(PlcPlatform::S7_300);
    let types_before = fixture.store.state().types.len();
    let request = super::AddParameterInput {
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        ..input(NESTED, NESTED_VALUE, Vec::new())
    };
    let error = run(&fixture, request).await.unwrap_err();
    assert_refuses_nesting(&error);
    assert_eq!(fixture.store.state().types.len(), types_before);
    assert!(stored_parameters(&fixture).is_empty());
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn adr_0177_an_array_with_two_dimensions_on_s7_300_adds_the_parameter() {
    let fixture = byte_draft(PlcPlatform::S7_300);
    let request = super::AddParameterInput {
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        ..input("ARRAY[1..2,1..3] OF BYTE", NESTED_VALUE, Vec::new())
    };
    run(&fixture, request).await.unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

#[tokio::test]
async fn adr_0177_an_array_of_an_array_on_codesys_iec_adds_the_parameter() {
    let fixture = byte_draft(PlcPlatform::CodesysIec);
    run(&fixture, input(NESTED, NESTED_VALUE, Vec::new()))
        .await
        .unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}
