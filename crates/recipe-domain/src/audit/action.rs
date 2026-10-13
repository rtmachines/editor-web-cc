use std::fmt;

/// The action of an audit entry. One name per command result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditAction {
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
    /// A parameter definition was updated.
    ParameterDefinitionUpdated,
    /// A parameter value was set.
    ParameterValueSet,
    /// The limits of a parameter were replaced.
    LimitsReplaced,
    /// A translation was inserted or updated.
    TranslationUpserted,
    /// A translation was removed.
    TranslationRemoved,
    /// A parameter was removed.
    ParameterRemoved,
    /// A recipe was rebound to a new type revision.
    TypeRevisionRebound,
    /// A user logged in.
    UserLoggedIn,
    /// A user logged out.
    UserLoggedOut,
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
    /// A version was exported.
    VersionExported,
    /// A startup setting was changed.
    SettingChanged,
}

impl AuditAction {
    /// Every audit action, in the order of `data-model.md` section 10.
    pub const ALL: [Self; 35] = [
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
        Self::ParameterDefinitionUpdated,
        Self::ParameterValueSet,
        Self::LimitsReplaced,
        Self::TranslationUpserted,
        Self::TranslationRemoved,
        Self::ParameterRemoved,
        Self::TypeRevisionRebound,
        Self::UserLoggedIn,
        Self::UserLoggedOut,
        Self::UserCreated,
        Self::UserRoleSet,
        Self::UserDeactivated,
        Self::UserActivated,
        Self::PasswordReset,
        Self::OwnPasswordChanged,
        Self::LoginUnlocked,
        Self::VersionExported,
        Self::SettingChanged,
    ];

    /// Return the name that the `action` column stores.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MachineCreated => "MachineCreated",
            Self::MachineRenamed => "MachineRenamed",
            Self::MachineArchived => "MachineArchived",
            Self::UnitAdded => "UnitAdded",
            Self::UnitRenamed => "UnitRenamed",
            Self::UnitArchived => "UnitArchived",
            Self::UdtDefined => "UdtDefined",
            Self::DerivedTypeCreated => "DerivedTypeCreated",
            Self::RecipeCreated => "RecipeCreated",
            Self::RecipeRenamed => "RecipeRenamed",
            Self::RecipeDeleted => "RecipeDeleted",
            Self::RecipeRestored => "RecipeRestored",
            Self::VersionCreated => "VersionCreated",
            Self::DraftDiscarded => "DraftDiscarded",
            Self::VersionApproved => "VersionApproved",
            Self::VersionRetired => "VersionRetired",
            Self::ParameterAdded => "ParameterAdded",
            Self::ParameterDefinitionUpdated => "ParameterDefinitionUpdated",
            Self::ParameterValueSet => "ParameterValueSet",
            Self::LimitsReplaced => "LimitsReplaced",
            Self::TranslationUpserted => "TranslationUpserted",
            Self::TranslationRemoved => "TranslationRemoved",
            Self::ParameterRemoved => "ParameterRemoved",
            Self::TypeRevisionRebound => "TypeRevisionRebound",
            Self::UserLoggedIn => "UserLoggedIn",
            Self::UserLoggedOut => "UserLoggedOut",
            Self::UserCreated => "UserCreated",
            Self::UserRoleSet => "UserRoleSet",
            Self::UserDeactivated => "UserDeactivated",
            Self::UserActivated => "UserActivated",
            Self::PasswordReset => "PasswordReset",
            Self::OwnPasswordChanged => "OwnPasswordChanged",
            Self::LoginUnlocked => "LoginUnlocked",
            Self::VersionExported => "VersionExported",
            Self::SettingChanged => "SettingChanged",
        }
    }
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The 35 names of `data-model.md` section 10.
    const EXPECTED: [&str; 35] = [
        "MachineCreated",
        "MachineRenamed",
        "MachineArchived",
        "UnitAdded",
        "UnitRenamed",
        "UnitArchived",
        "UdtDefined",
        "DerivedTypeCreated",
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
        "UserLoggedIn",
        "UserLoggedOut",
        "UserCreated",
        "UserRoleSet",
        "UserDeactivated",
        "UserActivated",
        "PasswordReset",
        "OwnPasswordChanged",
        "LoginUnlocked",
        "VersionExported",
        "SettingChanged",
    ];

    #[test]
    fn all_holds_35_distinct_names() {
        let names: BTreeSet<&str> = AuditAction::ALL
            .iter()
            .map(|action| action.as_str())
            .collect();
        assert_eq!(AuditAction::ALL.len(), 35);
        assert_eq!(names.len(), 35);
    }

    #[test]
    fn names_equal_the_section_10_table() {
        let names: BTreeSet<&str> = AuditAction::ALL
            .iter()
            .map(|action| action.as_str())
            .collect();
        let expected: BTreeSet<&str> = EXPECTED.into_iter().collect();
        assert_eq!(names, expected);
    }

    #[test]
    fn names_satisfy_the_audit_log_action_format() {
        for action in AuditAction::ALL {
            let text = action.as_str();
            let starts_upper = text.chars().next().is_some_and(|c| c.is_ascii_uppercase());
            assert!(starts_upper, "{text}");
            assert!(text.chars().all(|c| c.is_ascii_alphabetic()), "{text}");
            assert!((3..=64).contains(&text.len()), "{text}");
        }
    }

    #[test]
    fn display_writes_the_name() {
        for action in AuditAction::ALL {
            assert_eq!(action.to_string(), action.as_str());
        }
    }
}
