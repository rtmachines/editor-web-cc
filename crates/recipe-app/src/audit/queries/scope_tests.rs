//! Tests of the recipe scope of the audit queries (ADR-0051 (c), `data-model.md` section 10).

use recipe_domain::audit::AuditAction;

use super::scope::admits_filter;
use crate::audit::is_recipe_action;
use crate::ports::{AuditFilter, AuditScope};

/// The actions with `aggregate_type = 'recipe'`: Recipe Management and `VersionExported`.
///
/// The list comes from the table "Audit actions" of `data-model.md` section 10. It does not
/// come from the code under test.
const RECIPE_ACTIONS: [&str; 17] = [
    "RecipeCreated",
    "RecipeRenamed",
    "RecipeDeleted",
    "RecipeRestored",
    "VersionCreated",
    "DraftDiscarded",
    "VersionApproved",
    "VersionRetired",
    "ParameterAdded",
    "ParameterDefinitionUpdated",
    "ParameterValueSet",
    "LimitsReplaced",
    "TranslationUpserted",
    "TranslationRemoved",
    "ParameterRemoved",
    "TypeRevisionRebound",
    "VersionExported",
];

/// Return the filter that holds only `action`.
fn only(action: AuditAction) -> AuditFilter {
    AuditFilter {
        action: Some(action),
        ..AuditFilter::default()
    }
}

#[test]
fn the_recipe_scope_admits_exactly_the_actions_of_the_recipe_aggregate() {
    for action in AuditAction::ALL {
        let expected = RECIPE_ACTIONS.contains(&action.as_str());
        assert_eq!(
            admits_filter(AuditScope::Recipes, &only(action)),
            expected,
            "{action}"
        );
    }
}

#[test]
fn the_public_rule_names_exactly_the_actions_of_the_recipe_aggregate() {
    for action in AuditAction::ALL {
        let expected = RECIPE_ACTIONS.contains(&action.as_str());
        assert_eq!(is_recipe_action(action), expected, "{action}");
    }
}

#[test]
fn the_full_scope_admits_every_action() {
    for action in AuditAction::ALL {
        assert!(admits_filter(AuditScope::All, &only(action)), "{action}");
    }
}
