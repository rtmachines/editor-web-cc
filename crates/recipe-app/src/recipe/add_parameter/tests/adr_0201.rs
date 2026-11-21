//! Tests of `AddParameter` for the rule "WSTRING characters on CODESYS and TwinCAT"
//! (ADR-0201 (a), (c), (d)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::ParameterTag;
use serde_json::json;

use super::{AddParameterInput, assert_nothing_written, empty_draft, input, run};
use crate::recipe::rebind::fixtures::leaf_texts;
use crate::recipe::test_support::{expected_forms, iec_wstring, stored_parameters};

/// A request for the tag `HOST` of type `WSTRING(10)` with the JSON value `value`.
fn host_json(value: &str) -> AddParameterInput {
    AddParameterInput {
        tag: ParameterTag::parse("HOST").unwrap(),
        ..input("WSTRING(10)", value, Vec::new())
    }
}

/// A request for the tag `HOST` of type `WSTRING(10)` with the form text `text`.
fn host_text(text: &str) -> AddParameterInput {
    AddParameterInput {
        value: leaf_texts(&[("", text)]),
        ..host_json("null")
    }
}

#[tokio::test]
async fn adr_0201_a_form_text_outside_the_basic_plane_gives_rcp_005_on_twincat() {
    let fixture = empty_draft(PlcPlatform::TwinCat3);
    let error = run(&fixture, host_text("A\u{1F600}")).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(expected_forms(&error), vec![iec_wstring(10)]);
    assert!(stored_parameters(&fixture).is_empty());
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn adr_0201_a_json_value_outside_the_basic_plane_gives_rcp_005_on_both_platforms() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = empty_draft(platform);
        let error = run(&fixture, host_json(r#""Z𝄞""#)).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_005, "{platform:?}");
        assert_eq!(
            expected_forms(&error),
            vec![iec_wstring(10)],
            "{platform:?}"
        );
        assert!(stored_parameters(&fixture).is_empty(), "{platform:?}");
        assert_nothing_written(&fixture);
    }
}

#[tokio::test]
async fn adr_0201_characters_of_the_two_ranges_add_the_parameter() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = empty_draft(platform);
        run(&fixture, host_text("A\u{D7FF}\u{E000}\u{FFFF}"))
            .await
            .unwrap();
        let rows = stored_parameters(&fixture);
        assert_eq!(rows.len(), 1, "{platform:?}");
        assert_eq!(
            rows.first().unwrap().value.as_json(),
            &json!("A\u{D7FF}\u{E000}\u{FFFF}"),
            "{platform:?}"
        );
    }
}
