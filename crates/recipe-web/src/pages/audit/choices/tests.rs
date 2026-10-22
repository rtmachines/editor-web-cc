//! Tests of the choices of the audit filter form (ADR-0051 (d)).

use std::collections::BTreeSet;

use recipe_app::audit::is_recipe_action;
use recipe_domain::audit::AuditAction;

use super::{action_groups, entity_choices};
use crate::i18n::{Bundles, Lang};
use crate::pages::texts::Texts;

#[test]
fn choices_the_groups_hold_each_of_the_35_actions_once() {
    let bundles = Bundles::load().unwrap();
    let t = Texts::new(&bundles, Lang::En);
    let values: Vec<String> = action_groups(t, true)
        .into_iter()
        .flat_map(|group| group.options)
        .map(|choice| choice.value)
        .collect();
    assert_eq!(values.len(), 35);
    let distinct: BTreeSet<&str> = values.iter().map(String::as_str).collect();
    let expected: BTreeSet<&str> = AuditAction::ALL
        .iter()
        .map(|action| action.as_str())
        .collect();
    assert_eq!(distinct, expected);
}

#[test]
fn choices_the_recipe_scope_holds_17_actions_in_two_groups() {
    let bundles = Bundles::load().unwrap();
    let t = Texts::new(&bundles, Lang::En);
    let groups = action_groups(t, false);
    let labels: Vec<&str> = groups.iter().map(|group| group.label.as_str()).collect();
    assert_eq!(labels, ["Recipes", "Export"]);
    let count: usize = groups.iter().map(|group| group.options.len()).sum();
    assert_eq!(count, 17);
    assert_eq!(
        AuditAction::ALL
            .iter()
            .filter(|a| is_recipe_action(**a))
            .count(),
        17
    );
}

#[test]
fn choices_the_entity_select_starts_with_every_type_and_the_scope_cuts_it() {
    let bundles = Bundles::load().unwrap();
    for lang in [Lang::En, Lang::Pl] {
        let t = Texts::new(&bundles, lang);
        let all = entity_choices(t, true);
        assert_eq!(all.len(), 9);
        assert_eq!(all.first().map(|choice| choice.value.as_str()), Some(""));
        let recipes: Vec<String> = entity_choices(t, false)
            .into_iter()
            .map(|choice| choice.value)
            .collect();
        assert_eq!(recipes, ["", "recipe", "recipe_version", "parameter"]);
    }
}
