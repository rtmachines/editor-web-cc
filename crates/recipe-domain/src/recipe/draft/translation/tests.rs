//! Tests of the translation commands and `Draft::remove_parameter` (design D6).

use super::super::add::support::{UNRESOLVED, draft_with, draft_with_speed, snapshot};
use super::super::edit_support::{draft_with_content, translation};
use super::super::support::{key, reference, stored_parameter, tag};
use crate::error_codes::{ErrorCode, REGISTRY};
use crate::plant::PlcPlatform;
use crate::plant::fixtures::id;
use crate::recipe::{Draft, LangCode, RecipeError, Translation};
use crate::text::{LongText, Text};

fn pl_of(draft: &Draft) -> Vec<Translation> {
    let speed = draft.parameter(key(1)).unwrap();
    speed.translations().values().cloned().collect()
}

#[test]
fn a_missing_translation_has_code_registry_row_and_message() {
    let error = RecipeError::TranslationNotFound {
        reference: reference(),
        number: 3,
        tag: tag("SPEED"),
        lang: LangCode::parse("de-AT").unwrap(),
    };
    assert_eq!(error.code(), ErrorCode::RCP_050);
    assert_eq!(error.code().as_str(), "E-RCP-050");
    assert!(REGISTRY.contains(&ErrorCode::RCP_050));
    assert_eq!(
        error.to_string(),
        "Parameter SPEED in version 3 of recipe R-1 has no translation for de-AT."
    );
}

#[test]
fn a_second_upsert_of_pl_keeps_one_translation_with_the_new_name() {
    let mut draft = draft_with_speed();
    draft
        .upsert_translation(key(1), translation("pl", "Pierwsza"))
        .unwrap();
    let change = draft
        .upsert_translation(key(1), translation("pl", "Druga"))
        .unwrap();
    let after = change.after().unwrap();
    assert_eq!(after.translations().len(), 1);
    let names: Vec<&str> = after
        .translations()
        .values()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, vec!["Druga"]);
    assert_eq!(pl_of(&draft).len(), 1);
    let before_names: Vec<&str> = change
        .before()
        .unwrap()
        .translations()
        .values()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(before_names, vec!["Pierwsza"]);
}

#[test]
fn an_upsert_of_another_language_adds_a_second_translation() {
    let mut draft = draft_with_content();
    let change = draft
        .upsert_translation(key(1), translation("de-AT", "Geschwindigkeit"))
        .unwrap();
    assert_eq!(change.after().unwrap().translations().len(), 2);
    assert_eq!(change.before().unwrap().translations().len(), 1);
}

#[test]
fn an_upsert_keeps_the_value_and_the_limits() {
    let mut draft = draft_with_content();
    let old = draft.parameter(key(1)).unwrap().clone();
    let change = draft
        .upsert_translation(key(1), translation("de", "Tempo"))
        .unwrap();
    let after = change.after().unwrap();
    assert_eq!(after.value(), old.value());
    assert_eq!(after.limits(), old.limits());
}

#[test]
fn removing_a_translation_that_does_not_exist_gives_e_rcp_050() {
    let mut draft = draft_with_content();
    let before = snapshot(&draft);
    let lang = LangCode::parse("de-AT").unwrap();
    let error = draft.remove_translation(key(1), &lang).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_050);
    assert_eq!(
        error.to_string(),
        "Parameter SPEED in version 3 of recipe R-1 has no translation for de-AT."
    );
    assert_eq!(snapshot(&draft), before);
}

#[test]
fn removing_an_existing_translation_deletes_only_that_language() {
    let mut draft = draft_with_content();
    draft
        .upsert_translation(key(1), translation("de", "Tempo"))
        .unwrap();
    let lang = LangCode::parse("pl").unwrap();
    let change = draft.remove_translation(key(1), &lang).unwrap();
    let after = change.after().unwrap();
    assert!(!after.translations().contains_key(&lang));
    assert_eq!(after.translations().len(), 1);
    assert_eq!(change.before().unwrap().translations().len(), 2);
    assert_eq!(draft.parameter(key(1)).unwrap(), after);
}

#[test]
fn removing_a_parameter_deletes_it_with_its_limits_and_translations() {
    let mut draft = draft_with_content();
    let old = draft.parameter(key(1)).unwrap().clone();
    let change = draft.remove_parameter(key(1)).unwrap();
    assert_eq!(change.before(), Some(&old));
    assert_eq!(change.after(), None);
    assert!(draft.parameter(key(1)).is_err());
    let tags: Vec<&str> = draft
        .parameters()
        .iter()
        .map(|p| p.tag().as_str())
        .collect();
    assert_eq!(tags, vec!["TEMP"]);
}

#[test]
fn a_parameter_whose_type_did_not_resolve_can_be_removed() {
    let broken = stored_parameter(1, "SPEED", id(UNRESOLVED), None);
    let mut draft = draft_with(PlcPlatform::TwinCat3, vec![broken]);
    let change = draft.remove_parameter(key(1)).unwrap();
    assert!(change.before().is_some());
    assert!(draft.parameters().is_empty());
}

#[test]
fn an_unknown_key_gives_e_rcp_051_for_every_command() {
    let mut draft = draft_with_speed();
    let before = snapshot(&draft);
    let lang = LangCode::parse("pl").unwrap();
    let errors = [
        draft
            .upsert_translation(key(77), translation("pl", "X"))
            .unwrap_err(),
        draft.remove_translation(key(77), &lang).unwrap_err(),
        draft.remove_parameter(key(77)).unwrap_err(),
    ];
    for error in &errors {
        assert_eq!(error.code(), ErrorCode::RCP_051, "{error}");
    }
    assert_eq!(snapshot(&draft), before);
}

#[test]
fn a_translation_name_of_201_characters_fails_before_the_draft() {
    assert!(Text::parse(&"n".repeat(200)).is_ok());
    assert!(Text::parse(&"n".repeat(201)).is_err());
}

#[test]
fn a_translation_description_of_4001_characters_fails_before_the_draft() {
    assert!(LongText::parse(&"d".repeat(4_000)).is_ok());
    assert!(LongText::parse(&"d".repeat(4_001)).is_err());
}
