//! Tests of the approval checks of a draft (design D5, D12, ADR-0027, ADR-0028).

use serde_json::json;

use super::super::support::{address, stored_draft_with, user};
use crate::error_codes::{ErrorCode, REGISTRY};
use crate::plant::{PlcPlatform, PlcTypeId};
use crate::recipe::VersionState;
use crate::recipe::snapshot::fixtures::{
    ARCHIVED_UNIT, BOOL, INT, RAMPS, ST_RECIPE, STRING_20, ZONE_2, length, limited, on_unit, param,
    parameter_tag, range, trees, trees_on, units,
};
use crate::recipe::{Draft, DraftError, RecipeError, RecipeVersionId, StoredParameter};

fn draft_on(platform: PlcPlatform, parameters: Vec<StoredParameter>) -> Draft {
    Draft::load(stored_draft_with(
        VersionState::LiveDraft,
        platform,
        parameters,
    ))
    .unwrap()
}

fn draft(parameters: Vec<StoredParameter>) -> Draft {
    draft_on(PlcPlatform::TwinCat3, parameters)
}

fn recipe_value() -> serde_json::Value {
    json!({"temp_sp": 10, "ramps": [1, 2, 3], "label": "A", "active": true})
}

fn zone(n: u128, tag: &str) -> StoredParameter {
    limited(
        param(n, tag, ZONE_2, json!({"temp": 1})),
        vec![range(".temp", 0, 10)],
    )
}

fn limited_speed(n: u128, tag: &str) -> StoredParameter {
    limited(param(n, tag, INT, json!(5)), vec![range("", 0, 100)])
}

/// Return the entries of a failed list as code and message.
fn list_of(error: DraftError) -> Vec<(ErrorCode, String)> {
    match error {
        DraftError::List(errors) => errors
            .iter()
            .map(|entry| (entry.code(), entry.to_string()))
            .collect(),
        other => vec![(other.code(), format!("not a list: {other}"))],
    }
}

fn plain_code(error: &DraftError) -> ErrorCode {
    assert!(
        matches!(error, DraftError::Recipe(_)),
        "a single error expected: {error}"
    );
    error.code()
}

#[test]
fn a_parameter_without_a_limit_gives_a_list_with_one_e_rcp_016_entry() {
    let draft = draft(vec![param(1, "SPEED", INT, json!(5))]);
    let error = draft.check_limits(&trees()).unwrap_err();
    assert_eq!(
        list_of(error),
        vec![(
            ErrorCode::RCP_016,
            "SPEED has no process limit. Set a limit before approval.".to_owned()
        )]
    );
}

#[test]
fn a_struct_with_limits_on_every_numeric_and_string_leaf_passes() {
    let limits = vec![
        range(".temp_sp", 0, 100),
        range(".ramps[*]", 0, 10),
        length(".label", 20),
    ];
    let parameter = limited(param(1, "RECIPE", ST_RECIPE, recipe_value()), limits);
    assert!(draft(vec![parameter]).check_limits(&trees()).is_ok());
}

#[test]
fn bool_char_and_wchar_leaves_need_no_limit() {
    let draft = draft(vec![param(1, "ACTIVE", BOOL, json!(true))]);
    assert!(draft.check_limits(&trees()).is_ok());
}

#[test]
fn an_array_without_a_limit_gives_one_entry_for_each_element() {
    let limits = vec![range(".temp_sp", 0, 100), length(".label", 20)];
    let parameter = limited(param(1, "RECIPE", ST_RECIPE, recipe_value()), limits);
    let error = draft(vec![parameter]).check_limits(&trees()).unwrap_err();
    let entries = list_of(error);
    let messages: Vec<&str> = entries.iter().map(|(_, text)| text.as_str()).collect();
    assert_eq!(
        messages,
        vec![
            "RECIPE.ramps[1] has no process limit. Set a limit before approval.",
            "RECIPE.ramps[2] has no process limit. Set a limit before approval.",
            "RECIPE.ramps[3] has no process limit. Set a limit before approval.",
        ]
    );
    assert!(entries.iter().all(|(code, _)| *code == ErrorCode::RCP_016));
}

#[test]
fn a_limit_on_one_element_leaves_the_other_elements_missing() {
    let limits = vec![
        range(".temp_sp", 0, 100),
        range(".ramps[2]", 0, 10),
        length(".label", 20),
    ];
    let parameter = limited(param(1, "RECIPE", ST_RECIPE, recipe_value()), limits);
    let error = draft(vec![parameter]).check_limits(&trees()).unwrap_err();
    let messages: Vec<String> = list_of(error).into_iter().map(|(_, text)| text).collect();
    assert_eq!(messages.len(), 2);
    assert!(messages.first().unwrap().starts_with("RECIPE.ramps[1] "));
    assert!(messages.last().unwrap().starts_with("RECIPE.ramps[3] "));
}

#[test]
fn the_list_is_ordered_by_tag_and_then_by_leaf() {
    let draft = draft(vec![
        param(1, "B_SPEED", INT, json!(5)),
        param(2, "A_TEXT", STRING_20, json!("x")),
        param(3, "A_RAMPS", RAMPS, json!([1, 2, 3])),
    ]);
    let error = draft.check_limits(&trees()).unwrap_err();
    let tags: Vec<String> = list_of(error)
        .into_iter()
        .map(|(_, text)| {
            text.split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert_eq!(
        tags,
        vec![
            "A_RAMPS[1]",
            "A_RAMPS[2]",
            "A_RAMPS[3]",
            "A_TEXT",
            "B_SPEED"
        ]
    );
}

#[test]
fn an_empty_draft_gives_e_rcp_008() {
    let error = draft(Vec::new())
        .check_for_approval(&units(false), &trees(), false)
        .unwrap_err();
    assert_eq!(plain_code(&error), ErrorCode::RCP_008);
    assert_eq!(
        error.to_string(),
        "Version 3 of recipe R-1 has no parameters. Add at least one parameter before approval."
    );
}

#[test]
fn an_outdated_struct_gives_e_rcp_024_without_allow_outdated() {
    let draft = draft(vec![zone(1, "ZONE")]);
    let error = draft.check_outdated(&trees(), false).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_024);
    assert_eq!(
        error.to_string(),
        "ZONE uses revision 2 of ST_Zone. The newest revision is 3. Rebind the parameter."
    );
    let error = draft
        .check_for_approval(&units(false), &trees(), false)
        .unwrap_err();
    assert_eq!(plain_code(&error), ErrorCode::RCP_024);
}

#[test]
fn check_outdated_names_the_first_outdated_parameter_in_tag_order() {
    let draft = draft(vec![zone(1, "B_ZONE"), zone(2, "A_ZONE")]);
    let error = draft.check_outdated(&trees(), false).unwrap_err();
    assert!(error.to_string().starts_with("A_ZONE uses revision 2"));
}

#[test]
fn allow_outdated_gives_a_token_with_the_flag() {
    let draft = draft(vec![zone(1, "ZONE")]);
    assert!(draft.check_outdated(&trees(), true).is_ok());
    let ready = draft
        .check_for_approval(&units(false), &trees(), true)
        .unwrap();
    assert!(ready.allow_outdated());
}

#[test]
fn the_token_carries_the_version_number_row_version_creator_and_editors() {
    let checked = draft(vec![limited_speed(1, "SPEED")]);
    let ready = checked
        .check_for_approval(&units(false), &trees(), false)
        .unwrap();
    assert_eq!(
        ready.version(),
        RecipeVersionId::from(uuid::Uuid::from_u128(30))
    );
    assert_eq!(ready.number(), 3);
    assert_eq!(ready.row_version(), 7);
    assert_eq!(ready.creator(), user(1));
    let editors: Vec<_> = ready.editors().iter().copied().collect();
    assert_eq!(editors, vec![user(2), user(3)]);
    assert!(!ready.allow_outdated());
}

#[test]
fn an_archived_unit_gives_e_rcp_020_before_the_limit_check() {
    let parameter = on_unit(param(1, "SPEED", INT, json!(5)), ARCHIVED_UNIT);
    let error = draft(vec![parameter])
        .check_for_approval(&units(false), &trees(), false)
        .unwrap_err();
    assert_eq!(plain_code(&error), ErrorCode::RCP_020);
    assert_eq!(
        error.to_string(),
        "1 parameters are not valid. See the list."
    );
}

#[test]
fn an_archived_machine_gives_e_rcp_020() {
    let error = draft(vec![limited_speed(1, "SPEED")])
        .check_for_approval(&units(true), &trees(), false)
        .unwrap_err();
    assert_eq!(plain_code(&error), ErrorCode::RCP_020);
}

#[test]
fn a_missing_limit_gives_the_list_before_the_outdated_check() {
    let draft = draft(vec![param(1, "ZONE", ZONE_2, json!({"temp": 1}))]);
    let error = draft
        .check_for_approval(&units(false), &trees(), false)
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_016);
    assert!(matches!(error, DraftError::List(_)));
}

#[test]
fn an_s7_overlap_gives_e_rcp_022_before_the_outdated_check() {
    let mut first = limited_speed(1, "A_SPEED");
    first.s7_address = Some(address("DB1.DBW0"));
    let mut second = limited_speed(2, "B_SPEED");
    second.s7_address = Some(address("DB1.DBW0"));
    let draft = draft_on(PlcPlatform::S7_300, vec![first, second]);
    let error = draft
        .check_for_approval(&units(false), &trees_on(PlcPlatform::S7_300), false)
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_022);
}

#[test]
fn a_valid_draft_gives_a_token() {
    let limits = vec![
        range(".temp_sp", 0, 100),
        range(".ramps[*]", 0, 10),
        length(".label", 20),
    ];
    let parameters = vec![
        limited(param(1, "RECIPE", ST_RECIPE, recipe_value()), limits),
        limited_speed(2, "SPEED"),
    ];
    assert!(
        draft(parameters)
            .check_for_approval(&units(false), &trees(), false)
            .is_ok()
    );
}

#[test]
fn a_type_that_the_trees_do_not_know_stays_out_of_the_limit_check() {
    let unknown = PlcTypeId::from(uuid::Uuid::from_u128(99));
    let mut parameter = param(1, "LOST", INT, json!(5));
    parameter.plc_type = unknown;
    assert!(draft(vec![parameter]).check_limits(&trees()).is_ok());
}

#[test]
fn the_new_variants_have_their_codes_registry_rows_and_messages() {
    let reference = crate::recipe::ReferenceNumber::parse("R-1").unwrap();
    let cases = [
        (
            RecipeError::DraftHasNoParameters {
                reference,
                number: 3,
            },
            ErrorCode::RCP_008,
            "Version 3 of recipe R-1 has no parameters. Add at least one parameter before approval.",
        ),
        (
            RecipeError::LimitMissing {
                tag: parameter_tag("SPEED"),
                leaf: crate::plant::LeafPath::parse(".a[1]").unwrap(),
            },
            ErrorCode::RCP_016,
            "SPEED.a[1] has no process limit. Set a limit before approval.",
        ),
        (
            RecipeError::TypeRevisionOutdated {
                tag: parameter_tag("ZONE"),
                type_name: crate::plant::TypeName::parse("ST_Zone").unwrap(),
                current: 2,
                newest: 3,
            },
            ErrorCode::RCP_024,
            "ZONE uses revision 2 of ST_Zone. The newest revision is 3. Rebind the parameter.",
        ),
    ];
    for (error, code, message) in cases {
        assert_eq!(error.code(), code);
        assert!(REGISTRY.contains(&code));
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn trees_that_do_not_hold_a_type_of_the_draft_give_e_plt_013() {
    let draft = draft(vec![limited_speed(1, "SPEED")]);
    let partial = crate::recipe::snapshot::fixtures::trees_asking(&[BOOL]);
    let error = draft.check_limits(&partial).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013, "{error}");
    let error = draft.check_outdated(&partial, false).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013, "{error}");
}

#[test]
fn trees_of_another_machine_or_platform_give_e_plt_013_in_every_whole_draft_check() {
    use crate::plant::fixtures::other_machine;
    use crate::recipe::snapshot::fixtures::trees_of;
    let draft = draft(vec![limited_speed(1, "SPEED")]);
    for wrong in [
        trees_of(other_machine(), PlcPlatform::TwinCat3),
        trees_of(crate::plant::fixtures::machine(), PlcPlatform::S7_1500),
    ] {
        let errors = [
            draft.check_for_approval(&units(false), &wrong, false),
            draft.check_for_approval(&units(false), &wrong, true),
        ]
        .map(|result| result.map(|_| ()).unwrap_err());
        for error in errors {
            assert_eq!(error.code(), ErrorCode::PLT_013, "{error}");
        }
        assert_eq!(
            draft.check_limits(&wrong).unwrap_err().code(),
            ErrorCode::PLT_013
        );
        assert_eq!(
            draft.check_outdated(&wrong, false).unwrap_err().code(),
            ErrorCode::PLT_013
        );
        assert_eq!(
            draft.check_s7_layout(&wrong).unwrap_err().code(),
            ErrorCode::PLT_013
        );
    }
}

#[test]
fn units_of_another_machine_give_e_plt_020_items() {
    let draft = draft(vec![limited_speed(1, "SPEED")]);
    let error = draft
        .check_for_approval(
            &crate::recipe::snapshot::fixtures::foreign_units(),
            &trees(),
            false,
        )
        .map(|_| ())
        .unwrap_err();
    assert_eq!(plain_code(&error), ErrorCode::RCP_020);
    let codes: Vec<ErrorCode> = match &error {
        DraftError::Recipe(RecipeError::ValidationFailed { items }) => {
            items.iter().map(|item| item.code()).collect()
        }
        _ => Vec::new(),
    };
    assert_eq!(codes, vec![ErrorCode::PLT_020]);
}

#[test]
fn an_empty_draft_accepts_trees_and_units_of_any_machine() {
    let empty = draft(Vec::new());
    let error = empty
        .check_for_approval(&units(false), &trees(), false)
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_008);
}
