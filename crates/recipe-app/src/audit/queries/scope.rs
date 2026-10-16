//! The scope of A20 for the audit queries (ADR-0051 (c)).

use recipe_domain::audit::AuditAction;
use recipe_domain::identity::{Permission, Role};

use crate::ports::{AuditFilter, AuditScope};

/// Return the scope of `role`: every entry with `AuditView`, else only the recipe entries.
///
/// The caller checks the permission of the query first, so a role here holds `AuditView` or
/// `AuditViewRecipes`.
pub(super) fn audit_scope(role: Role) -> AuditScope {
    if role.has(Permission::AuditView) {
        AuditScope::All
    } else {
        AuditScope::Recipes
    }
}

/// Return whether the entries of `action` have `aggregate_type = 'recipe'` (`data-model.md` 10).
///
/// The actions of Recipe Management and `VersionExported` write their entries on the recipe.
/// The function solves one problem: the scope of a role and the select of the audit filter form
/// need one rule, so the web layer calls it and does not repeat it.
pub const fn is_recipe_action(action: AuditAction) -> bool {
    matches!(
        action,
        AuditAction::RecipeCreated
            | AuditAction::RecipeRenamed
            | AuditAction::RecipeDeleted
            | AuditAction::RecipeRestored
            | AuditAction::VersionCreated
            | AuditAction::DraftDiscarded
            | AuditAction::VersionApproved
            | AuditAction::VersionRetired
            | AuditAction::ParameterAdded
            | AuditAction::ParameterDefinitionUpdated
            | AuditAction::ParameterValueSet
            | AuditAction::LimitsReplaced
            | AuditAction::TranslationUpserted
            | AuditAction::TranslationRemoved
            | AuditAction::ParameterRemoved
            | AuditAction::TypeRevisionRebound
            | AuditAction::VersionExported
    )
}

/// Return whether a filter value of `filter` can match an entry inside `scope`.
///
/// A filter value of `action` or `entity_type` outside the scope gives the empty list without a
/// read (ADR-0051 (c)).
pub(super) fn admits_filter(scope: AuditScope, filter: &AuditFilter) -> bool {
    match scope {
        AuditScope::All => true,
        AuditScope::Recipes => {
            filter.action.is_none_or(is_recipe_action)
                && filter
                    .entity
                    .is_none_or(|entity| entity.entity_type.is_recipe_scope())
        }
    }
}
