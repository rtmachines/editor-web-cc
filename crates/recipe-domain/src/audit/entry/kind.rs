//! The three state kinds of an audit entry (`domain-model.md` section 2.4, state rule).
//!
//! Each action belongs to exactly one kind. A constructor of `AuditEntry` takes the
//! enum of its kind, so an action cannot meet a state of another kind.

use crate::audit::AuditAction;

/// Action that stores a diff object of the changed fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiffAction {
    /// A parameter definition was updated.
    ParameterDefinitionUpdated,
    /// A parameter value was set.
    ParameterValueSet,
    /// The limits of a parameter were replaced.
    LimitsReplaced,
    /// A translation was inserted or updated.
    TranslationUpserted,
    /// A recipe was rebound to a new type revision.
    TypeRevisionRebound,
}

impl DiffAction {
    /// Every diff action.
    pub const ALL: [Self; 5] = [
        Self::ParameterDefinitionUpdated,
        Self::ParameterValueSet,
        Self::LimitsReplaced,
        Self::TranslationUpserted,
        Self::TypeRevisionRebound,
    ];
}

impl From<DiffAction> for AuditAction {
    fn from(value: DiffAction) -> Self {
        match value {
            DiffAction::ParameterDefinitionUpdated => Self::ParameterDefinitionUpdated,
            DiffAction::ParameterValueSet => Self::ParameterValueSet,
            DiffAction::LimitsReplaced => Self::LimitsReplaced,
            DiffAction::TranslationUpserted => Self::TranslationUpserted,
            DiffAction::TypeRevisionRebound => Self::TypeRevisionRebound,
        }
    }
}

/// Action that stores a specific state object instead of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateAction {
    /// A user logged in. The state goes in `after`.
    UserLoggedIn,
    /// A user logged out. The state goes in `before`.
    UserLoggedOut,
    /// A version was exported. The state goes in `after`.
    VersionExported,
}

impl StateAction {
    /// Every state action.
    pub const ALL: [Self; 3] = [
        Self::UserLoggedIn,
        Self::UserLoggedOut,
        Self::VersionExported,
    ];
}

impl From<StateAction> for AuditAction {
    fn from(value: StateAction) -> Self {
        match value {
            StateAction::UserLoggedIn => Self::UserLoggedIn,
            StateAction::UserLoggedOut => Self::UserLoggedOut,
            StateAction::VersionExported => Self::VersionExported,
        }
    }
}

/// Action that stores the full row of the changed entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RowAction {
    /// A machine was created.
    MachineCreated,
    /// A machine was renamed.
    MachineRenamed,
    /// A machine was archived.
    MachineArchived,
    /// A unit was added to a machine.
    UnitAdded,
    /// A unit was renamed.
    UnitRenamed,
    /// A unit was archived.
    UnitArchived,
    /// A user-defined data type was defined.
    UdtDefined,
    /// A derived type was created by the internal operation.
    DerivedTypeCreated,
    /// A recipe was created.
    RecipeCreated,
    /// A recipe was renamed.
    RecipeRenamed,
    /// A recipe was deleted.
    RecipeDeleted,
    /// A deleted recipe was restored.
    RecipeRestored,
    /// A version was created.
    VersionCreated,
    /// A draft was discarded.
    DraftDiscarded,
    /// A version was approved.
    VersionApproved,
    /// A version was retired.
    VersionRetired,
    /// A parameter was added.
    ParameterAdded,
    /// A translation was removed.
    TranslationRemoved,
    /// A parameter was removed.
    ParameterRemoved,
    /// A user was created.
    UserCreated,
    /// The role of a user was set.
    UserRoleSet,
    /// A user was deactivated.
    UserDeactivated,
    /// A user was activated.
    UserActivated,
    /// A password was reset by an administrator.
    PasswordReset,
    /// A user changed the own password.
    OwnPasswordChanged,
    /// A login throttle was lifted.
    LoginUnlocked,
    /// A startup setting was changed.
    SettingChanged,
}

impl RowAction {
    /// Every row action.
    pub const ALL: [Self; 27] = [
        Self::MachineCreated,
        Self::MachineRenamed,
        Self::MachineArchived,
        Self::UnitAdded,
        Self::UnitRenamed,
        Self::UnitArchived,
        Self::UdtDefined,
        Self::DerivedTypeCreated,
        Self::RecipeCreated,
        Self::RecipeRenamed,
        Self::RecipeDeleted,
        Self::RecipeRestored,
        Self::VersionCreated,
        Self::DraftDiscarded,
        Self::VersionApproved,
        Self::VersionRetired,
        Self::ParameterAdded,
        Self::TranslationRemoved,
        Self::ParameterRemoved,
        Self::UserCreated,
        Self::UserRoleSet,
        Self::UserDeactivated,
        Self::UserActivated,
        Self::PasswordReset,
        Self::OwnPasswordChanged,
        Self::LoginUnlocked,
        Self::SettingChanged,
    ];
}

impl From<RowAction> for AuditAction {
    fn from(value: RowAction) -> Self {
        match value {
            RowAction::MachineCreated => Self::MachineCreated,
            RowAction::MachineRenamed => Self::MachineRenamed,
            RowAction::MachineArchived => Self::MachineArchived,
            RowAction::UnitAdded => Self::UnitAdded,
            RowAction::UnitRenamed => Self::UnitRenamed,
            RowAction::UnitArchived => Self::UnitArchived,
            RowAction::UdtDefined => Self::UdtDefined,
            RowAction::DerivedTypeCreated => Self::DerivedTypeCreated,
            RowAction::RecipeCreated => Self::RecipeCreated,
            RowAction::RecipeRenamed => Self::RecipeRenamed,
            RowAction::RecipeDeleted => Self::RecipeDeleted,
            RowAction::RecipeRestored => Self::RecipeRestored,
            RowAction::VersionCreated => Self::VersionCreated,
            RowAction::DraftDiscarded => Self::DraftDiscarded,
            RowAction::VersionApproved => Self::VersionApproved,
            RowAction::VersionRetired => Self::VersionRetired,
            RowAction::ParameterAdded => Self::ParameterAdded,
            RowAction::TranslationRemoved => Self::TranslationRemoved,
            RowAction::ParameterRemoved => Self::ParameterRemoved,
            RowAction::UserCreated => Self::UserCreated,
            RowAction::UserRoleSet => Self::UserRoleSet,
            RowAction::UserDeactivated => Self::UserDeactivated,
            RowAction::UserActivated => Self::UserActivated,
            RowAction::PasswordReset => Self::PasswordReset,
            RowAction::OwnPasswordChanged => Self::OwnPasswordChanged,
            RowAction::LoginUnlocked => Self::LoginUnlocked,
            RowAction::SettingChanged => Self::SettingChanged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn each_diff_action_converts_to_its_audit_action() {
        let pairs = [
            (
                DiffAction::ParameterDefinitionUpdated,
                AuditAction::ParameterDefinitionUpdated,
            ),
            (
                DiffAction::ParameterValueSet,
                AuditAction::ParameterValueSet,
            ),
            (DiffAction::LimitsReplaced, AuditAction::LimitsReplaced),
            (
                DiffAction::TranslationUpserted,
                AuditAction::TranslationUpserted,
            ),
            (
                DiffAction::TypeRevisionRebound,
                AuditAction::TypeRevisionRebound,
            ),
        ];
        for (diff, expected) in pairs {
            assert_eq!(AuditAction::from(diff), expected);
        }
        assert_eq!(DiffAction::ALL.len(), 5);
    }

    #[test]
    fn each_state_action_converts_to_its_audit_action() {
        let pairs = [
            (StateAction::UserLoggedIn, AuditAction::UserLoggedIn),
            (StateAction::UserLoggedOut, AuditAction::UserLoggedOut),
            (StateAction::VersionExported, AuditAction::VersionExported),
        ];
        for (state, expected) in pairs {
            assert_eq!(AuditAction::from(state), expected);
        }
        assert_eq!(StateAction::ALL.len(), 3);
    }

    #[test]
    fn row_actions_are_the_twenty_seven_remaining_actions() {
        assert_eq!(RowAction::ALL.len(), 27);
    }

    #[test]
    fn every_audit_action_belongs_to_exactly_one_kind() {
        let row: Vec<AuditAction> = RowAction::ALL.into_iter().map(AuditAction::from).collect();
        let diff: Vec<AuditAction> = DiffAction::ALL.into_iter().map(AuditAction::from).collect();
        let state: Vec<AuditAction> = StateAction::ALL
            .into_iter()
            .map(AuditAction::from)
            .collect();
        for action in AuditAction::ALL {
            let count = [&row, &diff, &state]
                .iter()
                .filter(|kind| kind.contains(&action))
                .count();
            assert_eq!(count, 1, "{action}");
        }
        let union: BTreeSet<AuditAction> = row.iter().chain(&diff).chain(&state).copied().collect();
        assert_eq!(union.len(), row.len() + diff.len() + state.len());
        assert_eq!(union, AuditAction::ALL.into_iter().collect::<BTreeSet<_>>());
    }

    #[test]
    fn the_diff_kind_is_the_five_actions_of_the_state_rule() {
        let diff: BTreeSet<&str> = DiffAction::ALL
            .into_iter()
            .map(|action| AuditAction::from(action).as_str())
            .collect();
        let expected: BTreeSet<&str> = [
            "ParameterDefinitionUpdated",
            "ParameterValueSet",
            "LimitsReplaced",
            "TranslationUpserted",
            "TypeRevisionRebound",
        ]
        .into_iter()
        .collect();
        assert_eq!(diff, expected);
    }
}
