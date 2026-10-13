//! The record that a use case passes to the commit (`domain-model.md` section 2.4).
//!
//! The database writes `id`, `occurred_at`, `chain_seq`, `prev_hash`, and `entry_hash`.
//! This type therefore does not hold them.

mod kind;

use std::net::IpAddr;

use serde_json::Value;

use super::{AuditAction, AuditTarget, RequestId, SessionRef};
use crate::identity::UserId;

pub use kind::{DiffAction, RowAction, StateAction};

/// Change of one row. Each variant keeps at least one state, so a change without state cannot exist.
///
/// This type enforces the constraint `audit_log_has_state`.
#[derive(Clone, Debug, PartialEq)]
pub enum RowChange {
    /// A create stores the new row in `after`.
    Create {
        /// The full row after the change.
        after: Value,
    },
    /// A delete stores the old row in `before`.
    Delete {
        /// The full row before the change.
        before: Value,
    },
    /// An update stores the old row in `before` and the new row in `after`.
    Update {
        /// The full row before the change.
        before: Value,
        /// The full row after the change.
        after: Value,
    },
}

/// Who did the change, and in which request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditContext {
    /// The acting user. It is `None` only for `create-admin`, the startup settings comparison, and a data migration.
    pub actor: Option<UserId>,
    /// The request that caused the change.
    pub request_id: RequestId,
    /// The address of the client, when the request has one.
    pub client_ip: Option<IpAddr>,
    /// The prefix of the session id hash, when the request has a session.
    pub session_ref: Option<SessionRef>,
}

/// One audit record of a committed change.
///
/// It solves the problem of an entry that breaks the state rule or the pairing of
/// action and state. Three constructors, one for each state kind, take the enum of
/// their kind. Every entry holds at least one state.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditEntry {
    action: AuditAction,
    target: AuditTarget,
    before: Option<Value>,
    after: Option<Value>,
    context: AuditContext,
}

impl AuditEntry {
    fn build(
        action: AuditAction,
        target: AuditTarget,
        before: Option<Value>,
        after: Option<Value>,
        context: AuditContext,
    ) -> Self {
        Self {
            action,
            target,
            before,
            after,
            context,
        }
    }

    /// Build the entry of an action that stores the full row of the changed entity.
    #[must_use]
    pub fn row(
        action: RowAction,
        target: AuditTarget,
        change: RowChange,
        context: AuditContext,
    ) -> Self {
        let (before, after) = match change {
            RowChange::Create { after } => (None, Some(after)),
            RowChange::Delete { before } => (Some(before), None),
            RowChange::Update { before, after } => (Some(before), Some(after)),
        };
        Self::build(action.into(), target, before, after, context)
    }

    /// Build the entry of an action that stores a diff object of the changed fields.
    ///
    /// Both diffs are objects. A diff with no old field is an empty object.
    #[must_use]
    pub fn diff(
        action: DiffAction,
        target: AuditTarget,
        before: Value,
        after: Value,
        context: AuditContext,
    ) -> Self {
        Self::build(action.into(), target, Some(before), Some(after), context)
    }

    /// Build the entry of an action that stores a specific state object.
    ///
    /// `UserLoggedOut` puts the state in `before`. The other two put it in `after`.
    #[must_use]
    pub fn state(
        action: StateAction,
        target: AuditTarget,
        state: Value,
        context: AuditContext,
    ) -> Self {
        let (before, after) = match action {
            StateAction::UserLoggedOut => (Some(state), None),
            StateAction::UserLoggedIn | StateAction::VersionExported => (None, Some(state)),
        };
        Self::build(action.into(), target, before, after, context)
    }

    /// Return the action.
    #[must_use]
    pub const fn action(&self) -> AuditAction {
        self.action
    }

    /// Return the changed row and the root of its aggregate.
    #[must_use]
    pub const fn target(&self) -> &AuditTarget {
        &self.target
    }

    /// Return the state before the change.
    #[must_use]
    pub const fn before(&self) -> Option<&Value> {
        self.before.as_ref()
    }

    /// Return the state after the change.
    #[must_use]
    pub const fn after(&self) -> Option<&Value> {
        self.after.as_ref()
    }

    /// Return the acting user.
    #[must_use]
    pub const fn actor(&self) -> Option<UserId> {
        self.context.actor
    }

    /// Return the request identifier.
    #[must_use]
    pub const fn request_id(&self) -> RequestId {
        self.context.request_id
    }

    /// Return the client address.
    #[must_use]
    pub const fn client_ip(&self) -> Option<IpAddr> {
        self.context.client_ip
    }

    /// Return the session reference.
    #[must_use]
    pub const fn session_ref(&self) -> Option<&SessionRef> {
        self.context.session_ref.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plant::MachineId;
    use crate::recipe::{ParameterId, RecipeId, RecipeVersionId};
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
            let entry =
                AuditEntry::row(RowAction::RecipeRenamed, recipe_target(), change, context());
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

    #[test]
    fn an_entry_without_actor_address_and_session_is_possible() {
        let bare = AuditContext {
            actor: None,
            request_id: RequestId::from(id(12)),
            client_ip: None,
            session_ref: None,
        };
        let entry = AuditEntry::row(
            RowAction::SettingChanged,
            machine_target(),
            RowChange::Update {
                before: json!({}),
                after: json!({}),
            },
            bare,
        );
        assert_eq!(entry.actor(), None);
        assert_eq!(entry.client_ip(), None);
        assert_eq!(entry.session_ref(), None);
    }
}
