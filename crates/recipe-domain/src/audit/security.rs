//! Security events of the audit context (`domain-model.md` 2.4, `data-model.md` 3.8).

use std::net::IpAddr;

use serde_json::{Value, json};

use super::request::{RequestId, UserAgent};
use crate::identity::{KeyRing, TypedLogin, UserId};

/// The value of `detail.reason` for a login of a technical account (ADR-0109 (c)).
const TECHNICAL_ACCOUNT_REASON: &str = "technical_account";

/// The value of `detail.scheme` for an event of the Basic check (ADR-0109 (m)).
const BASIC_SCHEME: &str = "basic";

/// Why a Basic check of the export routes refused an account.
///
/// The client always gets the answer of `E-IDN-003` (A42). The type solves one problem: only
/// the `security_event` row tells the operator the exact reason (ADR-0109 (m)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasicRefusal {
    /// No account has the typed login, or the typed login is outside the grammar of a login.
    UnknownLogin,
    /// The account is inactive.
    Inactive,
    /// The password does not match the hash of the account.
    WrongPassword,
    /// The password matches, but the account belongs to a person.
    PersonAccount,
}

impl BasicRefusal {
    /// Return the value of `detail.reason`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownLogin => "unknown_login",
            Self::Inactive => "inactive",
            Self::WrongPassword => "wrong_password",
            Self::PersonAccount => "person_account",
        }
    }
}

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
    /// The job `audit_chain_check` found a broken hash chain.
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
    /// Compute the HMAC of a typed login under the newest key of the ring.
    #[must_use]
    pub fn compute(ring: &KeyRing, login: &TypedLogin) -> Self {
        Self(ring.sign(login.as_str().as_bytes()))
    }

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

/// The subject of a `LoginFailed` event.
///
/// The enum makes a call pass either the user or the login HMAC, never both and never none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginSubject {
    /// An account matches the typed login.
    User(UserId),
    /// No account matches. The event holds the HMAC of the typed login.
    Unknown(LoginHmac),
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

    /// Build a `LoginFailed` event. The subject is the user or the HMAC of the typed login.
    #[must_use]
    pub fn login_failed(
        subject: LoginSubject,
        request_id: RequestId,
        client_ip: IpAddr,
        user_agent: Option<UserAgent>,
    ) -> Self {
        let (user, login_hmac) = match subject {
            LoginSubject::User(user) => (Some(user), None),
            LoginSubject::Unknown(hmac) => (None, Some(hmac)),
        };
        Self {
            user,
            login_hmac,
            ..Self::request_event(
                SecurityEventKind::LoginFailed,
                request_id,
                client_ip,
                user_agent,
            )
        }
    }

    /// Build a `LoginFailed` event for a technical account that used the login form.
    ///
    /// The event holds the user and the reason `technical_account` in `detail`. It solves one
    /// problem: the answer to the client is the answer of a wrong password (A42), so only this
    /// event tells the operator the exact reason (ADR-0109 (c)).
    #[must_use]
    pub fn login_failed_technical_account(
        user: UserId,
        request_id: RequestId,
        client_ip: IpAddr,
        user_agent: Option<UserAgent>,
    ) -> Self {
        Self {
            detail: Some(json!({ "reason": TECHNICAL_ACCOUNT_REASON })),
            ..Self::login_failed(LoginSubject::User(user), request_id, client_ip, user_agent)
        }
    }

    /// Mark the event as a refusal of the Basic check. `detail` gets the reason and the scheme.
    ///
    /// The method replaces `detail`. The events of `Login` hold no `detail` of this kind.
    #[must_use]
    pub fn with_basic_refusal(self, reason: BasicRefusal) -> Self {
        Self {
            detail: Some(json!({ "reason": reason.as_str(), "scheme": BASIC_SCHEME })),
            ..self
        }
    }

    /// Mark the event as an event of the Basic check without a reason. `detail` gets the scheme.
    ///
    /// The throttle event uses it, because its kind already names the reason.
    #[must_use]
    pub fn with_basic_scheme(self) -> Self {
        Self {
            detail: Some(json!({ "scheme": BASIC_SCHEME })),
            ..self
        }
    }

    /// Build a `LoginRejectedInactive` event for the inactive user.
    #[must_use]
    pub fn login_rejected_inactive(
        user: UserId,
        request_id: RequestId,
        client_ip: IpAddr,
        user_agent: Option<UserAgent>,
    ) -> Self {
        Self {
            user: Some(user),
            ..Self::request_event(
                SecurityEventKind::LoginRejectedInactive,
                request_id,
                client_ip,
                user_agent,
            )
        }
    }

    /// Build a `LoginThrottled` event. It always holds the HMAC and never the user.
    #[must_use]
    pub fn login_throttled(
        login_hmac: LoginHmac,
        request_id: RequestId,
        client_ip: IpAddr,
        user_agent: Option<UserAgent>,
    ) -> Self {
        Self {
            login_hmac: Some(login_hmac),
            ..Self::request_event(
                SecurityEventKind::LoginThrottled,
                request_id,
                client_ip,
                user_agent,
            )
        }
    }

    /// Build a `CsrfRejected` event. It has no subject.
    #[must_use]
    pub fn csrf_rejected(
        request_id: RequestId,
        client_ip: IpAddr,
        user_agent: Option<UserAgent>,
    ) -> Self {
        Self::request_event(
            SecurityEventKind::CsrfRejected,
            request_id,
            client_ip,
            user_agent,
        )
    }

    /// Build an `Unlock` event. The subject is the user whose login was unlocked.
    #[must_use]
    pub fn unlock(
        target: UserId,
        unlocked_by: UserId,
        request_id: RequestId,
        client_ip: IpAddr,
        user_agent: Option<UserAgent>,
    ) -> Self {
        Self {
            user: Some(target),
            detail: Some(json!({ "unlocked_by": unlocked_by.as_uuid().to_string() })),
            ..Self::request_event(SecurityEventKind::Unlock, request_id, client_ip, user_agent)
        }
    }

    /// Build an `AuditChainBroken` event of the job `audit_chain_check`.
    ///
    /// The job has no request, so the event has no client address, no subject, and no
    /// User-Agent. The request id is the id of the job run (`data-model.md` 3.8).
    #[must_use]
    pub fn audit_chain_broken(request_id: RequestId) -> Self {
        Self {
            kind: SecurityEventKind::AuditChainBroken,
            login_hmac: None,
            user: None,
            client_ip: None,
            user_agent: None,
            request_id,
            detail: None,
            count: 1,
        }
    }

    /// Build an event of a request with a client address, no subject, and no detail.
    fn request_event(
        kind: SecurityEventKind,
        request_id: RequestId,
        client_ip: IpAddr,
        user_agent: Option<UserAgent>,
    ) -> Self {
        Self {
            kind,
            login_hmac: None,
            user: None,
            client_ip: Some(client_ip),
            user_agent,
            request_id,
            detail: None,
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
mod tests;
