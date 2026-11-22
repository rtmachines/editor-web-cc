//! Tests of `AddParameter` for the rule "no LF and no CR in a request text" (ADR-0210 (a)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::ParameterTag;

use super::{AddParameterInput, assert_nothing_written, empty_draft, input, run};
use crate::recipe::rebind::fixtures::leaf_texts;
use crate::recipe::test_support::stored_parameters;

/// A request for the tag `NAME` of type `STRING(10)` with the JSON value `value`.
fn name_json(value: &str) -> AddParameterInput {
    AddParameterInput {
        tag: ParameterTag::parse("NAME").unwrap(),
        ..input("STRING(10)", value, Vec::new())
    }
}

#[tokio::test]
async fn add_parameter_with_lf_gives_e_rcp_005() {
    let fixture = empty_draft(PlcPlatform::TwinCat3);
    let error = run(&fixture, name_json(r#""A\nB""#)).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert!(stored_parameters(&fixture).is_empty());
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn add_parameter_with_a_form_text_with_cr_gives_e_rcp_005() {
    let fixture = empty_draft(PlcPlatform::TwinCat3);
    let request = AddParameterInput {
        value: leaf_texts(&[("", "A\rB")]),
        ..name_json("null")
    };
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert!(stored_parameters(&fixture).is_empty());
    assert_nothing_written(&fixture);
}
