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
