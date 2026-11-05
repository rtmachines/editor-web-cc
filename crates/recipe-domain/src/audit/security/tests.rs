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
fn login_failed_of_a_technical_account_holds_the_user_and_the_reason() {
    let user = UserId::from(Uuid::from_u128(5));
    let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    let event = SecurityEvent::login_failed_technical_account(
        user,
        RequestId::from(Uuid::from_u128(9)),
        ip,
        None,
    );
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), Some(user));
    assert_eq!(event.login_hmac(), None);
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "technical_account"}))
    );
}

#[test]
fn a_basic_refusal_puts_the_reason_and_the_scheme_in_the_detail() {
    let user = UserId::from(Uuid::from_u128(5));
    let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    let request = RequestId::from(Uuid::from_u128(9));
    let event = SecurityEvent::login_failed(LoginSubject::User(user), request, ip, None);
    let marked = event
        .clone()
        .with_basic_refusal(BasicRefusal::PersonAccount);
    assert_eq!(
        marked.detail(),
        Some(&json!({"reason": "person_account", "scheme": "basic"}))
    );
    assert_eq!(marked.kind(), event.kind());
    assert_eq!(marked.user(), event.user());
    assert_eq!(
        event.with_basic_scheme().detail(),
        Some(&json!({"scheme": "basic"}))
    );
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
fn audit_chain_broken_has_no_client_address_and_no_subject() {
    let request = RequestId::from(Uuid::from_u128(21));
    let event = SecurityEvent::audit_chain_broken(request);
    assert_eq!(event.kind(), SecurityEventKind::AuditChainBroken);
    assert_eq!(event.request_id(), request);
    assert_eq!(event.client_ip(), None);
    assert_eq!(event.user_agent(), None);
    assert_eq!(event.user(), None);
    assert_eq!(event.login_hmac(), None);
    assert_eq!(event.detail(), None);
    assert_eq!(event.count(), 1);
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
