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
