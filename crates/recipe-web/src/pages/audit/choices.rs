//! The choices of the selects of the audit filter form (ADR-0051 (d)).
//!
//! The module solves one problem: a role with only `AuditViewRecipes` must see only the entity
//! types and the actions of the recipe scope (A20), and the select groups the 35 actions by
//! context. The select uses the rule `recipe_app::audit::is_recipe_action`, so the select and the
//! scope of the query cannot differ. A test below compares the groups with it.

use recipe_app::audit::is_recipe_action;
use recipe_app::ports::AuditEntityType;
use recipe_domain::audit::AuditAction;

use super::rows::{action_word, entity_word};
use crate::pages::recipes::Choice;
use crate::pages::texts::Texts;

/// A group of actions: the key of its label and its actions.
type Group = (&'static str, &'static [AuditAction]);

/// The 35 actions grouped by context, in the order of the select (design D8).
const GROUPS: [Group; 5] = [
    (
        "audit-group-plant",
        &[
            AuditAction::MachineCreated,
            AuditAction::MachineRenamed,
            AuditAction::MachineArchived,
            AuditAction::UnitAdded,
            AuditAction::UnitRenamed,
            AuditAction::UnitArchived,
            AuditAction::UdtDefined,
            AuditAction::DerivedTypeCreated,
        ],
    ),
    (
        "audit-group-recipe",
        &[
            AuditAction::RecipeCreated,
            AuditAction::RecipeRenamed,
            AuditAction::RecipeDeleted,
            AuditAction::RecipeRestored,
            AuditAction::VersionCreated,
            AuditAction::DraftDiscarded,
            AuditAction::VersionApproved,
            AuditAction::VersionRetired,
            AuditAction::ParameterAdded,
            AuditAction::ParameterDefinitionUpdated,
            AuditAction::ParameterValueSet,
            AuditAction::LimitsReplaced,
            AuditAction::TranslationUpserted,
            AuditAction::TranslationRemoved,
            AuditAction::ParameterRemoved,
            AuditAction::TypeRevisionRebound,
        ],
    ),
    (
        "audit-group-identity",
        &[
            AuditAction::UserLoggedIn,
            AuditAction::UserLoggedOut,
            AuditAction::UserCreated,
            AuditAction::UserRoleSet,
            AuditAction::UserDeactivated,
            AuditAction::UserActivated,
            AuditAction::PasswordReset,
            AuditAction::OwnPasswordChanged,
            AuditAction::LoginUnlocked,
        ],
    ),
    ("audit-group-export", &[AuditAction::VersionExported]),
    ("audit-group-settings", &[AuditAction::SettingChanged]),
];

/// One `optgroup` of the action select.
pub(crate) struct ActionGroup {
    /// The label of the group.
    pub(crate) label: String,
    /// The actions of the group.
    pub(crate) options: Vec<Choice>,
}

/// Return the choices of the entity type select: every type, then the types of the scope.
pub(crate) fn entity_choices(t: Texts<'_>, all: bool) -> Vec<Choice> {
    std::iter::once(Choice::new("", t.get("audit-filter-any-type")))
        .chain(
            AuditEntityType::ALL
                .into_iter()
                .filter(|entity_type| all || entity_type.is_recipe_scope())
                .map(|entity_type| Choice::new(entity_type.as_sql(), entity_word(t, entity_type))),
        )
        .collect()
}

/// Return the groups of the action select for the scope. A group without an action is left out.
pub(crate) fn action_groups(t: Texts<'_>, all: bool) -> Vec<ActionGroup> {
    GROUPS
        .iter()
        .map(|(key, actions)| ActionGroup {
            label: t.get(key),
            options: actions
                .iter()
                .filter(|action| all || is_recipe_action(**action))
                .map(|action| Choice::new(action.as_str(), action_word(t, *action)))
                .collect(),
        })
        .filter(|group| !group.options.is_empty())
        .collect()
}

#[cfg(test)]
mod tests;
