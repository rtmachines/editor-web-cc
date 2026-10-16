//! Tests of `UpdateParameterDefinition` with a retype from the texts of a form (ADR-0039 (c)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::{Expected, RecipeError};

use super::requests::{retyped, run, speed_request, with_type};
use crate::error::AppError;
use crate::recipe::rebind::fixtures::{error_lines, leaf_texts};
use crate::recipe::test_support::{
    DraftSnapshot, bounds, speed_key, stored_parameters, twincat_draft,
};

#[tokio::test]
async fn a_type_change_with_texts_stores_the_canonical_value() {
    let fixture = twincat_draft();
    let request = retyped(
        with_type(speed_request(), "REAL"),
        leaf_texts(&[("", "12.50")]),
        vec![bounds("0", "100")],
    );
    run(&fixture, request).await.unwrap();
    let row = stored_parameters(&fixture)
        .into_iter()
        .find(|row| row.key == speed_key())
        .unwrap();
    assert_eq!(row.value.as_json().to_string(), "12.5");
    assert_eq!(row.limits.len(), 1);
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn a_bad_text_and_a_bad_row_of_a_retype_give_one_list_and_change_nothing() {
    let fixture = twincat_draft();
    let before = DraftSnapshot::capture(&fixture);
    let request = retyped(
        speed_request(),
        leaf_texts(&[("", "abc")]),
        vec![bounds("250", "10")],
    );
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error_lines(&error), ["E-RCP-005 ''", "E-RCP-018 ''"]);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn missing_value_fields_after_a_type_change_give_rcp_005_for_each_leaf() {
    let fixture = twincat_draft();
    let before = DraftSnapshot::capture(&fixture);
    let request = retyped(
        with_type(speed_request(), "ARRAY[1..3] OF INT"),
        leaf_texts(&[("[1]", "1")]),
        Vec::new(),
    );
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(error_lines(&error), ["E-RCP-005 '[2]'", "E-RCP-005 '[3]'"]);
    assert!(
        matches!(&error, AppError::Recipes(list) if list.iter().all(|entry| matches!(
            entry,
            RecipeError::ValueShapeMismatch { expected: Expected::Leaf, .. }
        ))),
        "{error:?}"
    );
    before.assert_unchanged(&fixture);
}
