use super::*;
use crate::plant::MachineId;
use crate::recipe::{ParameterId, RecipeId, RecipeVersionId};
use crate::setting::SettingId;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn context() -> AuditContext {
    AuditContext {
        actor: Some(UserId::from(id(10))),
        request_id: RequestId::from(id(11)),
        client_ip: Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7))),
        session_ref: Some(SessionRef::parse("0123456789abcdef").unwrap()),
    }
}

fn machine_target() -> AuditTarget {
    AuditTarget::machine(MachineId::from(id(1)))
}

fn recipe_target() -> AuditTarget {
    AuditTarget::recipe(RecipeId::from(id(4)))
}

#[test]
fn a_create_keeps_only_the_after_state() {
    let entry = AuditEntry::row(
        RowAction::MachineCreated,
        machine_target(),
        RowChange::Create {
            after: json!({"name": "M1"}),
        },
        context(),
    );
    assert_eq!(entry.action(), AuditAction::MachineCreated);
    assert_eq!(entry.before(), None);
    assert_eq!(entry.after(), Some(&json!({"name": "M1"})));
}

#[test]
fn a_delete_keeps_only_the_before_state() {
    let entry = AuditEntry::row(
        RowAction::ParameterRemoved,
        AuditTarget::parameter(RecipeId::from(id(4)), ParameterId::from(id(6))),
        RowChange::Delete {
            before: json!({"key": "p"}),
        },
        context(),
    );
    assert_eq!(entry.action(), AuditAction::ParameterRemoved);
    assert_eq!(entry.before(), Some(&json!({"key": "p"})));
    assert_eq!(entry.after(), None);
}

#[test]
fn an_update_keeps_both_states() {
    let entry = AuditEntry::row(
        RowAction::RecipeRenamed,
        recipe_target(),
        RowChange::Update {
            before: json!({"name": "a"}),
            after: json!({"name": "b"}),
        },
        context(),
    );
    assert_eq!(entry.before(), Some(&json!({"name": "a"})));
    assert_eq!(entry.after(), Some(&json!({"name": "b"})));
}

#[test]
fn a_row_entry_always_has_a_state() {
    let changes = [
        RowChange::Create { after: json!({}) },
        RowChange::Delete { before: json!({}) },
        RowChange::Update {
            before: json!({}),
            after: json!({}),
        },
    ];
    for change in changes {
        let entry = AuditEntry::row(RowAction::RecipeRenamed, recipe_target(), change, context());
        assert!(entry.before().is_some() || entry.after().is_some());
    }
}

#[test]
fn a_diff_entry_keeps_both_diffs_and_the_converted_action() {
    let entry = AuditEntry::diff(
        DiffAction::ParameterValueSet,
        AuditTarget::parameter(RecipeId::from(id(4)), ParameterId::from(id(6))),
        json!({"value": "1"}),
        json!({"value": "2"}),
        context(),
    );
    assert_eq!(entry.action(), AuditAction::ParameterValueSet);
    assert_eq!(entry.before(), Some(&json!({"value": "1"})));
    assert_eq!(entry.after(), Some(&json!({"value": "2"})));
}

#[test]
fn a_login_stores_its_state_in_after() {
    let entry = AuditEntry::state(
        StateAction::UserLoggedIn,
        AuditTarget::app_user(UserId::from(id(10))),
        json!({"session_ref": "0123456789abcdef"}),
        context(),
    );
    assert_eq!(entry.action(), AuditAction::UserLoggedIn);
    assert_eq!(entry.before(), None);
    assert_eq!(
        entry.after(),
        Some(&json!({"session_ref": "0123456789abcdef"}))
    );
}

#[test]
fn a_logout_stores_its_state_in_before() {
    let entry = AuditEntry::state(
        StateAction::UserLoggedOut,
        AuditTarget::app_user(UserId::from(id(10))),
        json!({"session_ref": "0123456789abcdef"}),
        context(),
    );
    assert_eq!(entry.action(), AuditAction::UserLoggedOut);
    assert_eq!(
        entry.before(),
        Some(&json!({"session_ref": "0123456789abcdef"}))
    );
    assert_eq!(entry.after(), None);
}

#[test]
fn an_export_stores_its_state_in_after() {
    let entry = AuditEntry::state(
        StateAction::VersionExported,
        AuditTarget::recipe_version(RecipeId::from(id(4)), RecipeVersionId::from(id(5))),
        json!({"schema_version": 1}),
        context(),
    );
    assert_eq!(entry.action(), AuditAction::VersionExported);
    assert_eq!(entry.before(), None);
    assert_eq!(entry.after(), Some(&json!({"schema_version": 1})));
}

#[test]
fn the_entry_keeps_target_and_context() {
    let entry = AuditEntry::row(
        RowAction::MachineArchived,
        machine_target(),
        RowChange::Update {
            before: json!({}),
            after: json!({}),
        },
        context(),
    );
    assert_eq!(entry.target(), &machine_target());
    assert_eq!(entry.actor(), Some(UserId::from(id(10))));
    assert_eq!(entry.request_id(), RequestId::from(id(11)));
    assert_eq!(
        entry.client_ip(),
        Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7)))
    );
    assert_eq!(
        entry.session_ref().map(SessionRef::as_str),
        Some("0123456789abcdef")
    );
}

fn setting_row(value: &str) -> Value {
    json!({"key": "approval.separation_of_duties", "value": value, "updated_by": null})
}

#[test]
fn a_setting_change_keeps_the_full_row_before_and_after_and_has_no_actor() {
    let setting = SettingId::from(id(8));
    let entry = AuditEntry::setting_changed(
        setting,
        Some(setting_row("true")),
        setting_row("false"),
        RequestId::from(id(12)),
    );
    assert_eq!(entry.action(), AuditAction::SettingChanged);
    assert_eq!(entry.target(), &AuditTarget::app_setting(setting));
    assert_eq!(entry.before(), Some(&setting_row("true")));
    assert_eq!(entry.after(), Some(&setting_row("false")));
    assert_eq!(entry.actor(), None);
    assert_eq!(entry.request_id(), RequestId::from(id(12)));
    assert_eq!(entry.client_ip(), None);
    assert_eq!(entry.session_ref(), None);
}

#[test]
fn a_setting_change_of_a_missing_row_keeps_only_the_after_state() {
    let entry = AuditEntry::setting_changed(
        SettingId::from(id(8)),
        None,
        setting_row("true"),
        RequestId::from(id(12)),
    );
    assert_eq!(entry.before(), None);
    assert_eq!(entry.after(), Some(&setting_row("true")));
    assert_eq!(entry.actor(), None);
}

#[test]
fn a_setting_change_copies_a_fingerprint_row_without_change() {
    let fingerprint = format!("sha256:{}", "ab".repeat(32));
    let row = json!({"key": "alerts.webhook_url", "value": fingerprint});
    let entry = AuditEntry::setting_changed(
        SettingId::from(id(9)),
        Some(json!({"key": "alerts.webhook_url", "value": ""})),
        row.clone(),
        RequestId::from(id(12)),
    );
    assert_eq!(entry.after(), Some(&row));
}

#[test]
fn an_entry_without_actor_address_and_session_is_possible() {
    let bare = AuditContext {
        actor: None,
        request_id: RequestId::from(id(12)),
        client_ip: None,
        session_ref: None,
    };
    let entry = AuditEntry::row(
        RowAction::UserCreated,
        AuditTarget::app_user(UserId::from(id(10))),
        RowChange::Create { after: json!({}) },
        bare,
    );
    assert_eq!(entry.actor(), None);
    assert_eq!(entry.client_ip(), None);
    assert_eq!(entry.session_ref(), None);
}
