//! Tests of `ApproveVersion` for the rule "WSTRING characters on CODESYS and TwinCAT"
//! (ADR-0201 (g)): the approval checks every stored value with the current rule set.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::{
    ApprovalOutcome, Expected, ParameterTag, RecipeError, SnapshotError, SnapshotItem,
};
use serde_json::json;

use super::{only_the_read, run};
use crate::error::AppError;
use crate::recipe::test_support::{iec_wstring, seed_note_draft};

/// The tag and the expected form of each `E-RCP-005` parameter item of an `E-RCP-020` error.
fn value_items(error: &AppError) -> Vec<(ParameterTag, Expected)> {
    match error {
        AppError::Recipe(RecipeError::ValidationFailed { items }) => items
            .iter()
            .filter_map(|item| match item {
                SnapshotItem::Parameter {
                    tag,
                    error: SnapshotError::Recipe(RecipeError::ValueShapeMismatch { expected, .. }),
                } => Some((tag.clone(), expected.clone())),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[tokio::test]
async fn adr_0201_a_stored_character_outside_the_basic_plane_gives_rcp_020_with_the_tag() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = seed_note_draft(platform, 10, json!("A😀"));
        let error = run(&fixture, 1, 1, false).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_020, "{platform:?}");
        assert!(
            matches!(&error, AppError::Recipe(RecipeError::ValidationFailed { items })
                if items.len() == 1),
            "{platform:?}: {error:?}"
        );
        assert_eq!(
            value_items(&error),
            vec![(ParameterTag::parse("NOTE").unwrap(), iec_wstring(10))],
            "{platform:?}: {error:?}"
        );
        assert_eq!(fixture.store.journal(), only_the_read(), "{platform:?}");
        assert!(fixture.store.audit_entries().is_empty(), "{platform:?}");
    }
}

#[tokio::test]
async fn adr_0201_a_stored_text_of_the_basic_plane_stays_approvable() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = seed_note_draft(platform, 10, json!("A\u{D7FF}\u{E000}\u{FFFF}"));
        let outcome = run(&fixture, 1, 1, false).await.unwrap();
        assert!(
            matches!(outcome, ApprovalOutcome::Approved { .. }),
            "{platform:?}: {outcome:?}"
        );
    }
}
