//! The record that a use case passes to the commit (`domain-model.md` section 2.4).
//!
//! The database writes `id`, `occurred_at`, `chain_seq`, `prev_hash`, and `entry_hash`.
//! This type therefore does not hold them.

mod kind;

use std::net::IpAddr;

use serde_json::Value;

use super::{AuditAction, AuditTarget, RequestId, SessionRef};
use crate::identity::UserId;
use crate::setting::SettingId;

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

    /// Build the entry of a startup setting change.
    ///
    /// The startup settings comparison writes it, not a user. The entry therefore has no
    /// actor, no client address, and no session. It keeps the full rows as it gets them.
    /// The comparison updates or inserts a row and never deletes one. A missing row has no
    /// old state, so `before` is `None` and the entry is a create.
    #[must_use]
    pub fn setting_changed(
        setting: SettingId,
        before: Option<Value>,
        after: Value,
        request_id: RequestId,
    ) -> Self {
        let change = match before {
            None => RowChange::Create { after },
            Some(before) => RowChange::Update { before, after },
        };
        let context = AuditContext {
            actor: None,
            request_id,
            client_ip: None,
            session_ref: None,
        };
        Self::row(
            RowAction::SettingChanged,
            AuditTarget::app_setting(setting),
            change,
            context,
        )
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
mod tests;
