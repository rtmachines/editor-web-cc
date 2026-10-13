//! Security events of the audit context (`domain-model.md` 2.4, `data-model.md` 3.8).

use std::net::IpAddr;

use serde_json::{Value, json};

use super::request::{RequestId, UserAgent};
use crate::identity::UserId;

/// The kind of a security event. Each kind has one SQL value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityEventKind {
    /// A login failed. The password is wrong or no account matches.
    LoginFailed,
    /// A login was refused because the account is inactive.
    LoginRejectedInactive,
    /// A login was refused by the login throttle.
    LoginThrottled,
    /// A request failed the CSRF check.
    CsrfRejected,
    /// A user lacked the permission of an operation.
    Forbidden,
    /// An administrator lifted a login throttle.
    Unlock,
    /// The nightly audit chain check found a broken hash chain.
    AuditChainBroken,
}

impl SecurityEventKind {
    /// Every kind, in the order of `domain-model.md` section 2.4.
    pub const ALL: [Self; 7] = [
        Self::LoginFailed,
        Self::LoginRejectedInactive,
        Self::LoginThrottled,
        Self::CsrfRejected,
        Self::Forbidden,
        Self::Unlock,
        Self::AuditChainBroken,
    ];

    /// Return the SQL value of the kind (`data-model.md` 3.8).
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::LoginFailed => "login_failed",
            Self::LoginRejectedInactive => "login_rejected_inactive",
            Self::LoginThrottled => "login_throttled",
            Self::CsrfRejected => "csrf_rejected",
            Self::Forbidden => "forbidden",
            Self::Unlock => "unlock",
            Self::AuditChainBroken => "audit_chain_broken",
        }
    }
}

/// HMAC-SHA256 of a login value. A security event never stores the plain login.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoginHmac([u8; 32]);

impl LoginHmac {
    /// Return the 32 bytes of the HMAC.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for LoginHmac {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

/// One security event. Other contexts write it and never read it back.
#[derive(Clone, Debug, PartialEq)]
pub struct SecurityEvent {
    kind: SecurityEventKind,
    login_hmac: Option<LoginHmac>,
    user: Option<UserId>,
    client_ip: Option<IpAddr>,
    user_agent: Option<UserAgent>,
    request_id: RequestId,
    detail: Option<Value>,
    count: u32,
}

impl SecurityEvent {
    /// Build a `Forbidden` event for an operation that the user may not run.
    #[must_use]
    pub fn forbidden(
        user: UserId,
        operation: &str,
        request_id: RequestId,
        client_ip: Option<IpAddr>,
        user_agent: Option<UserAgent>,
    ) -> Self {
        Self {
            kind: SecurityEventKind::Forbidden,
            login_hmac: None,
            user: Some(user),
            client_ip,
            user_agent,
            request_id,
            detail: Some(json!({ "operation": operation })),
            count: 1,
        }
    }

    /// Return the kind of the event.
    #[must_use]
    pub const fn kind(&self) -> SecurityEventKind {
        self.kind
    }

    /// Return the HMAC of the login value, when one exists.
    #[must_use]
    pub const fn login_hmac(&self) -> Option<&LoginHmac> {
        self.login_hmac.as_ref()
    }

    /// Return the user of the event, when an account matches.
    #[must_use]
    pub const fn user(&self) -> Option<UserId> {
        self.user
    }

    /// Return the client address, when the request has one.
    #[must_use]
    pub const fn client_ip(&self) -> Option<IpAddr> {
        self.client_ip
    }

    /// Return the kept User-Agent header, when the request has one.
    #[must_use]
    pub const fn user_agent(&self) -> Option<&UserAgent> {
        self.user_agent.as_ref()
    }

    /// Return the identifier of the request.
    #[must_use]
    pub const fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Return the detail object of the event, when one exists.
    #[must_use]
    pub const fn detail(&self) -> Option<&Value> {
        self.detail.as_ref()
    }

    /// Return the number of events that this row holds.
    #[must_use]
    pub const fn count(&self) -> u32 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::UserId;
    use std::net::{IpAddr, Ipv4Addr};
    use uuid::Uuid;

    #[test]
    fn kinds_have_the_sql_names_of_section_2_4() {
        let names: Vec<&str> = SecurityEventKind::ALL
            .iter()
            .map(|kind| kind.as_sql())
            .collect();
        assert_eq!(
            names,
            [
                "login_failed",
                "login_rejected_inactive",
                "login_throttled",
                "csrf_rejected",
                "forbidden",
                "unlock",
                "audit_chain_broken",
            ]
        );
    }

    #[test]
    fn forbidden_event_holds_user_request_and_operation() {
        let user = UserId::from(Uuid::from_u128(7));
        let request = RequestId::from(Uuid::from_u128(8));
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        let event = SecurityEvent::forbidden(
            user,
            "RenameMachine",
            request,
            Some(ip),
            Some(UserAgent::from("test-agent")),
        );
        assert_eq!(event.kind(), SecurityEventKind::Forbidden);
        assert_eq!(event.user(), Some(user));
        assert_eq!(event.request_id(), request);
        assert_eq!(event.client_ip(), Some(ip));
        assert_eq!(event.login_hmac(), None);
        assert_eq!(
            event.detail(),
            Some(&serde_json::json!({"operation": "RenameMachine"}))
        );
        assert_eq!(event.count(), 1);
    }

    #[test]
    fn login_hmac_keeps_its_bytes() {
        assert_eq!(LoginHmac::from([7; 32]).as_bytes(), &[7; 32]);
    }
}
