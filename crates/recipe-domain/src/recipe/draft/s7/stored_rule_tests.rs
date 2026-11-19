//! Tests that the command path stores the platform rule defects of ADR-0195 (i).
//!
//! A command skips the address rules. A stored defect of another parameter must give `Stored`
//! with the key of that parameter and not an error of the user.

use super::super::add::support::{
    DINT as SUPPORT_DINT, INT as SUPPORT_INT, UNRESOLVED, draft_with, new_parameter_at, trees_on,
};
use super::super::support::{address, assert_stored, key, stored_parameter};
use crate::error_codes::ErrorCode;
use crate::plant::PlcPlatform;
use crate::plant::fixtures::id;
use crate::recipe::{Draft, DraftError, LimitList, ValueInput};

/// Write the value of the parameter with the key 2 and return the error.
fn set_value_of_key_2(draft: &mut Draft, platform: PlcPlatform) -> DraftError {
    draft
        .set_value(
            key(2),
            &ValueInput::Json("5".to_owned()),
            &trees_on(platform),
        )
        .unwrap_err()
}

/// A draft with a healthy `FEED` (key 2) and the stored parameter `SPEED` (key 1).
fn draft_beside(platform: PlcPlatform, stored: crate::recipe::StoredParameter) -> Draft {
    let feed_address = match platform {
        PlcPlatform::TwinCat3 | PlcPlatform::CodesysIec => None,
        _ => Some(address("DB1.DBW8")),
    };
    draft_with(
        platform,
        vec![
            stored,
            stored_parameter(2, "FEED", id(SUPPORT_INT), feed_address),
        ],
    )
}

#[test]
fn set_value_beside_an_s7_300_parameter_without_an_address_gives_stored_e_rcp_003() {
    let platform = PlcPlatform::S7_300;
    let mut draft = draft_beside(
        platform,
        stored_parameter(1, "SPEED", id(SUPPORT_INT), None),
    );

    let error = set_value_of_key_2(&mut draft, platform);

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_003);
}

#[test]
fn set_value_beside_a_twincat3_parameter_with_an_address_gives_stored_e_rcp_053() {
    let platform = PlcPlatform::TwinCat3;
    let stored = stored_parameter(1, "SPEED", id(SUPPORT_INT), Some(address("DB1.DBW0")));
    let mut draft = draft_beside(platform, stored);

    let error = set_value_of_key_2(&mut draft, platform);

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_053);
}

#[test]
fn set_value_beside_a_codesys_parameter_with_an_address_gives_stored_e_rcp_053() {
    let platform = PlcPlatform::CodesysIec;
    let stored = stored_parameter(1, "SPEED", id(SUPPORT_INT), Some(address("DB1.DBW0")));
    let mut draft = draft_beside(platform, stored);

    let error = set_value_of_key_2(&mut draft, platform);

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_053);
}

#[test]
fn an_unresolved_s7_300_parameter_without_an_address_gives_stored_e_rcp_003() {
    let platform = PlcPlatform::S7_300;
    let mut draft = draft_beside(platform, stored_parameter(1, "LOST", id(UNRESOLVED), None));

    let error = set_value_of_key_2(&mut draft, platform);

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_003);
}

#[test]
fn an_unresolved_twincat3_parameter_with_an_address_gives_stored_e_rcp_053() {
    let platform = PlcPlatform::TwinCat3;
    let stored = stored_parameter(1, "LOST", id(UNRESOLVED), Some(address("DB1.DBW0")));
    let mut draft = draft_beside(platform, stored);

    let error = set_value_of_key_2(&mut draft, platform);

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_053);
}

#[test]
fn two_stored_defects_give_the_key_of_the_first_tag_and_not_the_first_key() {
    let platform = PlcPlatform::S7_300;
    let mut draft = draft_with(
        platform,
        vec![
            stored_parameter(1, "ZETA", id(SUPPORT_INT), None),
            stored_parameter(2, "ALPHA", id(SUPPORT_INT), None),
            stored_parameter(3, "MID", id(SUPPORT_INT), Some(address("DB1.DBW8"))),
        ],
    );

    let error = draft
        .set_value(
            key(3),
            &ValueInput::Json("5".to_owned()),
            &trees_on(platform),
        )
        .unwrap_err();

    assert_stored(&error, Some(key(2)), ErrorCode::RCP_003);
}

/// A live draft on `S7_300` with a stored `DINT` `SPEED` at `DB1.DBW0` and an `INT` `FEED`.
///
/// The stored address of `SPEED` breaks the width rule of the earlier rule set.
fn draft_with_a_stored_width_defect() -> Draft {
    draft_with(
        PlcPlatform::S7_300,
        vec![
            stored_parameter(1, "SPEED", id(SUPPORT_DINT), Some(address("DB1.DBW0"))),
            stored_parameter(2, "FEED", id(SUPPORT_INT), Some(address("DB1.DBW8"))),
        ],
    )
}

#[test]
fn set_value_on_a_parameter_beside_a_stored_width_defect_gives_stored_with_its_key() {
    let mut draft = draft_with_a_stored_width_defect();
    let trees = trees_on(PlcPlatform::S7_300);

    let error = draft
        .set_value(key(2), &ValueInput::Json("5".to_owned()), &trees)
        .unwrap_err();

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_054);
}

#[test]
fn the_approval_check_on_a_stored_width_defect_gives_e_rcp_054_and_not_stored() {
    let draft = draft_with_a_stored_width_defect();
    let trees = trees_on(PlcPlatform::S7_300);

    let error = draft.check_s7_layout_for_approval(&trees).unwrap_err();

    assert!(matches!(error, DraftError::Recipe(_)), "{error:?}");
    assert_eq!(error.code(), ErrorCode::RCP_054);
}

#[test]
fn replace_limits_on_a_parameter_with_its_own_misaligned_address_gives_stored() {
    let mut draft = draft_with(
        PlcPlatform::S7_300,
        vec![stored_parameter(
            1,
            "SPEED",
            id(SUPPORT_INT),
            Some(address("DB1.DBW3")),
        )],
    );
    let trees = trees_on(PlcPlatform::S7_300);
    let limits = LimitList::parse(Vec::new()).unwrap();

    let error = draft.replace_limits(key(1), &limits, &trees).unwrap_err();

    assert_stored(&error, Some(key(1)), ErrorCode::RCP_023);
}

#[test]
fn add_parameter_with_a_request_address_of_the_wrong_width_gives_e_rcp_054_and_not_stored() {
    let mut draft = draft_with(PlcPlatform::S7_300, Vec::new());
    let trees = trees_on(PlcPlatform::S7_300);

    let error = draft
        .add_parameter(
            new_parameter_at(1, "SPEED", SUPPORT_DINT, "DB1.DBW0"),
            &trees,
        )
        .unwrap_err();

    assert!(matches!(error, DraftError::Recipe(_)), "{error:?}");
    assert_eq!(error.code(), ErrorCode::RCP_054);
}
