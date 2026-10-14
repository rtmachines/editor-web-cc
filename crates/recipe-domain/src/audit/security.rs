//! Security events of the audit context (`domain-model.md` 2.4, `data-model.md` 3.8).

use std::net::IpAddr;

use serde_json::{Value, json};

use super::request::{RequestId, UserAgent};
use crate::identity::{KeyRing, TypedLogin, UserId};

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
mod tests {
    use super::*;
    use crate::identity::{KeyRing, TypedLogin, UserId};
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

    fn context() -> (RequestId, IpAddr, Option<UserAgent>) {
        (
            RequestId::from(Uuid::from_u128(8)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)),
            Some(UserAgent::from("test-agent")),
        )
    }

    fn assert_common(event: &SecurityEvent, kind: SecurityEventKind) {
        let (request, ip, agent) = context();
        assert_eq!(event.kind(), kind);
        assert_eq!(event.request_id(), request);
        assert_eq!(event.client_ip(), Some(ip));
        assert_eq!(event.user_agent(), agent.as_ref());
        assert_eq!(event.count(), 1);
    }

    #[test]
    fn login_failed_of_an_existing_account_holds_the_user_and_no_hmac() {
        let user = UserId::from(Uuid::from_u128(7));
        let (request, ip, agent) = context();
        let event = SecurityEvent::login_failed(LoginSubject::User(user), request, ip, agent);
        assert_common(&event, SecurityEventKind::LoginFailed);
        assert_eq!(event.user(), Some(user));
        assert_eq!(event.login_hmac(), None);
        assert_eq!(event.detail(), None);
    }

    #[test]
    fn login_failed_of_an_unknown_account_holds_the_hmac_and_no_user() {
        let hmac = LoginHmac::from([9; 32]);
        let (request, ip, agent) = context();
        let event = SecurityEvent::login_failed(LoginSubject::Unknown(hmac), request, ip, agent);
        assert_common(&event, SecurityEventKind::LoginFailed);
        assert_eq!(event.user(), None);
        assert_eq!(event.login_hmac(), Some(&hmac));
    }

    #[test]
    fn login_rejected_inactive_holds_the_user_and_no_hmac() {
        let user = UserId::from(Uuid::from_u128(7));
        let (request, ip, agent) = context();
        let event = SecurityEvent::login_rejected_inactive(user, request, ip, agent);
        assert_common(&event, SecurityEventKind::LoginRejectedInactive);
        assert_eq!(event.user(), Some(user));
        assert_eq!(event.login_hmac(), None);
    }

    #[test]
    fn login_throttled_always_holds_the_hmac_and_no_user() {
        let hmac = LoginHmac::from([4; 32]);
        let (request, ip, agent) = context();
        let event = SecurityEvent::login_throttled(hmac, request, ip, agent);
        assert_common(&event, SecurityEventKind::LoginThrottled);
        assert_eq!(event.login_hmac(), Some(&hmac));
        assert_eq!(event.user(), None);
    }

    #[test]
    fn csrf_rejected_has_no_subject() {
        let (request, ip, agent) = context();
        let event = SecurityEvent::csrf_rejected(request, ip, agent);
        assert_common(&event, SecurityEventKind::CsrfRejected);
        assert_eq!(event.user(), None);
        assert_eq!(event.login_hmac(), None);
        assert_eq!(event.detail(), None);
    }

    #[test]
    fn unlock_holds_the_target_and_names_the_administrator_in_the_detail() {
        let target = UserId::from(Uuid::from_u128(7));
        let admin = UserId::from(Uuid::from_u128(12));
        let (request, ip, agent) = context();
        let event = SecurityEvent::unlock(target, admin, request, ip, agent);
        assert_common(&event, SecurityEventKind::Unlock);
        assert_eq!(event.user(), Some(target));
        assert_eq!(event.login_hmac(), None);
        assert_eq!(
            event.detail(),
            Some(&serde_json::json!({"unlocked_by": Uuid::from_u128(12).to_string()}))
        );
    }

    #[test]
    fn the_new_constructors_meet_the_subject_constraint_of_section_3_8() {
        let user = UserId::from(Uuid::from_u128(7));
        let hmac = LoginHmac::from([4; 32]);
        let (request, ip, agent) = context();
        let events = [
            SecurityEvent::login_failed(LoginSubject::User(user), request, ip, agent.clone()),
            SecurityEvent::login_failed(LoginSubject::Unknown(hmac), request, ip, agent.clone()),
            SecurityEvent::login_rejected_inactive(user, request, ip, agent.clone()),
            SecurityEvent::login_throttled(hmac, request, ip, agent.clone()),
            SecurityEvent::csrf_rejected(request, ip, agent.clone()),
            SecurityEvent::unlock(user, user, request, ip, agent),
        ];
        for event in &events {
            let has_subject = event.login_hmac().is_some() || event.user().is_some();
            assert!(has_subject || event.kind() == SecurityEventKind::CsrfRejected);
            assert!(event.client_ip().is_some());
        }
    }

    const KEY_ONE: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const KEY_TWO: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    #[test]
    fn login_hmac_is_the_signature_of_the_typed_login_under_the_first_key() {
        let ring = KeyRing::parse(&format!("{KEY_ONE}\n{KEY_TWO}")).unwrap();
        let typed = TypedLogin::parse("jkowalski").unwrap();
        let expected = LoginHmac::from(ring.sign(b"jkowalski"));
        assert_eq!(LoginHmac::compute(&ring, &typed), expected);
    }

    #[test]
    fn login_hmac_ignores_the_case_of_the_typed_login() {
        let ring = KeyRing::parse(KEY_ONE).unwrap();
        let upper = TypedLogin::parse("JKowalski").unwrap();
        let lower = TypedLogin::parse("jkowalski").unwrap();
        assert_eq!(
            LoginHmac::compute(&ring, &upper),
            LoginHmac::compute(&ring, &lower)
        );
    }

    #[test]
    fn login_hmac_differs_for_another_login_or_another_key() {
        let ring = KeyRing::parse(KEY_ONE).unwrap();
        let other_ring = KeyRing::parse(KEY_TWO).unwrap();
        let first = TypedLogin::parse("jkowalski").unwrap();
        let second = TypedLogin::parse("anowak").unwrap();
        assert_ne!(
            LoginHmac::compute(&ring, &first),
            LoginHmac::compute(&ring, &second)
        );
        assert_ne!(
            LoginHmac::compute(&ring, &first),
            LoginHmac::compute(&other_ring, &first)
        );
    }
}
