//! Tests of `Draft::load` and `Draft::parameter`.

use uuid::Uuid;

use super::super::support::{
    key, parameter, reference, stored_draft, stored_draft_with, tag, user,
};
use crate::error_codes::{ErrorCode, REGISTRY};
use crate::plant::{PlcPlatform, PlcTypeId};
use crate::recipe::{Draft, ParameterChange, RecipeError, VersionState, VersionStatus};

/// A type identifier that no tree resolves.
fn unknown_type() -> PlcTypeId {
    PlcTypeId::from(Uuid::from_u128(0xDEAD))
}

#[test]
fn a_live_draft_loads_with_its_version_data() {
    let draft = Draft::load(stored_draft(vec![parameter(1, "SPEED", 5)])).unwrap();
    assert_eq!(draft.reference(), &reference());
    assert_eq!(draft.number(), 3);
    assert_eq!(draft.row_version(), 7);
    assert_eq!(draft.platform(), PlcPlatform::TwinCat3);
    assert_eq!(draft.created_by(), user(1));
    let editors: Vec<_> = draft.editors().iter().copied().collect();
    assert_eq!(editors, vec![user(2), user(3)]);
}

#[test]
fn a_discarded_draft_gives_e_rcp_002_with_status_draft() {
    let stored = stored_draft_with(VersionState::Discarded, PlcPlatform::TwinCat3, Vec::new());
    let error = Draft::load(stored).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert!(matches!(
        error,
        RecipeError::VersionNotEditable {
            number: 3,
            status: VersionStatus::Draft,
            ..
        }
    ));
}

#[test]
fn an_approved_version_gives_e_rcp_002_with_status_approved() {
    let stored = stored_draft_with(VersionState::Approved, PlcPlatform::TwinCat3, Vec::new());
    let error = Draft::load(stored).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert_eq!(
        error.to_string(),
        "Version 3 of recipe R-1 is approved. Create a new draft to make changes."
    );
}

#[test]
fn a_retired_version_gives_e_rcp_002_with_status_retired() {
    let stored = stored_draft_with(VersionState::Retired, PlcPlatform::TwinCat3, Vec::new());
    let error = Draft::load(stored).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert!(matches!(
        error,
        RecipeError::VersionNotEditable {
            status: VersionStatus::Retired,
            ..
        }
    ));
}

#[test]
fn a_duplicate_tag_gives_e_rcp_014_with_the_tag_of_the_second_parameter() {
    let stored = stored_draft(vec![parameter(1, "SPEED", 5), parameter(2, "SPEED", 5)]);
    let error = Draft::load(stored).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_014);
    assert_eq!(
        error.to_string(),
        "Parameter tag SPEED is already used in version 3."
    );
}

#[test]
fn a_duplicate_key_gives_e_rcp_014_with_the_tag_of_the_second_parameter() {
    let mut second = parameter(2, "TEMP", 5);
    second.key = key(1);
    let stored = stored_draft(vec![parameter(1, "SPEED", 5), second]);
    let error = Draft::load(stored).unwrap_err();
    assert!(matches!(
        error,
        RecipeError::DuplicateTag { ref tag, number: 3 } if tag.as_str() == "TEMP"
    ));
}

#[test]
fn parameter_returns_the_parameter_of_a_key() {
    let draft = Draft::load(stored_draft(vec![
        parameter(1, "SPEED", 5),
        parameter(2, "TEMP", 5),
    ]))
    .unwrap();
    let found = draft.parameter(key(2)).unwrap();
    assert_eq!(found.tag(), &tag("TEMP"));
    assert_eq!(found.key(), key(2));
}

#[test]
fn an_unknown_key_gives_e_rcp_051() {
    let draft = Draft::load(stored_draft(vec![parameter(1, "SPEED", 5)])).unwrap();
    let error = draft.parameter(key(9)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_eq!(
        error.to_string(),
        format!(
            "Parameter {} does not exist in version 3 of recipe R-1.",
            key(9)
        )
    );
}

#[test]
fn a_parameter_whose_type_is_in_no_tree_still_loads() {
    let stored = stored_draft(vec![super::super::support::stored_parameter(
        1,
        "LOST",
        unknown_type(),
        None,
    )]);
    let draft = Draft::load(stored).unwrap();
    assert_eq!(draft.parameter(key(1)).unwrap().plc_type(), unknown_type());
}

#[test]
fn parameters_are_ordered_by_tag() {
    let draft = Draft::load(stored_draft(vec![
        parameter(1, "ZETA", 5),
        parameter(2, "ALPHA", 5),
        parameter(3, "MID", 5),
    ]))
    .unwrap();
    let tags: Vec<&str> = draft
        .parameters()
        .iter()
        .map(|parameter| parameter.tag().as_str())
        .collect();
    assert_eq!(tags, vec!["ALPHA", "MID", "ZETA"]);
}

#[test]
fn a_parameter_keeps_its_stored_fields() {
    let draft = Draft::load(stored_draft(vec![parameter(4, "SPEED", 5)])).unwrap();
    let loaded = draft.parameter(key(4)).unwrap();
    assert_eq!(loaded.name_en().as_str(), "Name");
    assert_eq!(loaded.description_en().as_str(), "Description");
    assert!(loaded.translations().is_empty());
    assert!(loaded.limits().is_empty());
    assert!(loaded.s7_address().is_none());
    assert_eq!(loaded.ordinal().value(), 4);
}

#[test]
fn a_parameter_change_keeps_the_old_and_the_new_parameter() {
    let draft = Draft::load(stored_draft(vec![parameter(1, "SPEED", 5)])).unwrap();
    let old = draft.parameter(key(1)).unwrap().clone();
    let change = ParameterChange::new(Some(old.clone()), None);
    assert_eq!(change.before(), Some(&old));
    assert!(change.after().is_none());
}

#[test]
fn the_three_new_codes_have_registry_rows() {
    for code in [ErrorCode::RCP_002, ErrorCode::RCP_014, ErrorCode::RCP_051] {
        assert!(REGISTRY.contains(&code), "{code}");
    }
    assert_eq!(ErrorCode::RCP_002.as_str(), "E-RCP-002");
    assert_eq!(ErrorCode::RCP_014.as_str(), "E-RCP-014");
    assert_eq!(ErrorCode::RCP_051.as_str(), "E-RCP-051");
}

#[test]
fn two_parameters_with_one_identifier_give_e_rcp_014_with_the_second_tag() {
    let first = parameter(1, "SPEED", 5);
    let mut second = parameter(2, "TEMP", 5);
    second.id = first.id;
    let error = Draft::load(stored_draft(vec![first, second])).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_014);
    assert!(matches!(
        error,
        RecipeError::DuplicateTag { tag, number: 3 } if tag.as_str() == "TEMP"
    ));
}
