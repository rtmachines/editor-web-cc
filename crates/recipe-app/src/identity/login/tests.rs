//! Tests of the use case `Login` (task 4.1, design D6, `data-model.md` section 6).

use std::fmt::Debug;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, LazyLock};
use std::task::Poll;
use std::time::Duration as StdDuration;

use parking_lot::Mutex;
use recipe_domain::audit::{
    AuditAction, AuditTarget, LoginHmac, RequestId, SecurityEventKind, SessionRef, UserAgent,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{
    CredentialFailure, CsrfToken, IdentityError, Login as UserLogin, Password, Role, SessionId,
    TypedLogin, UserId,
};
use recipe_domain::timestamp::Timestamp;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use uuid::Uuid;

use super::{Login, LoginInput};
use crate::auth::{Operation, RequestMeta};
use crate::context::UseCase;
use crate::error::{AppError, TimeoutStage};
use crate::identity::fixtures::{IdentityFixture, fake_hash};
use crate::identity::no_session::require_no_session;
use crate::memory::{Fault, MemoryStore};
use crate::ports::{NewSession, NewUser};

const PASSWORD: &str = "correct-horse-battery";
const START_SECONDS: i64 = 1_000_000;
const DRAIN_WAIT: StdDuration = StdDuration::from_secs(5);

fn user_id() -> UserId {
    UserId::from(Uuid::from_u128(11))
}

fn ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))
}

fn at(offset: i64) -> Timestamp {
    let seconds = START_SECONDS.checked_add(offset).unwrap();
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn request_id() -> RequestId {
    RequestId::from(Uuid::from_u128(0x77))
}

fn meta() -> RequestMeta {
    RequestMeta::new(
        request_id(),
        Some(ip()),
        Some(UserAgent::from("test-agent")),
        None,
    )
}

fn input(login: &str, password: &str) -> LoginInput {
    LoginInput {
        login: TypedLogin::parse(login).unwrap(),
        password: Password::parse(password).unwrap(),
        client_ip: ip(),
        old_session: None,
    }
}

fn seed_user(store: &MemoryStore, role: Role, must_change_password: bool) {
    store.seed_user(&NewUser {
        id: user_id(),
        login: UserLogin::parse("anna").unwrap(),
        password_hash: fake_hash(PASSWORD),
        role,
        must_change_password,
    });
}

fn fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    seed_user(&fixture.store, Role::Operator, false);
    fixture
}

fn deactivate(store: &MemoryStore) {
    store.update_state(|state, _now| {
        state.users.get_mut(&user_id()).unwrap().is_active = false;
    });
}

/// A fixture whose user changes between the password check and T2.
fn racing_fixture(change: fn(&mut crate::ports::UserRow)) -> IdentityFixture {
    let fixture = IdentityFixture::with_race(move |store| {
        store.update_state(|state, _now| change(state.users.get_mut(&user_id()).unwrap()));
    });
    seed_user(&fixture.store, Role::Operator, false);
    fixture
}

fn reset(row: &mut crate::ports::UserRow) {
    row.password_hash = fake_hash("a-one-time-password");
    row.session_epoch = row.session_epoch.checked_add(1).unwrap();
}

fn seed_failures(store: &MemoryStore, seconds_ago: &[i64]) {
    let login = TypedLogin::parse("anna").unwrap();
    for ago in seconds_ago {
        store.seed_attempt(&login, ip(), at(ago.checked_neg().unwrap()), false);
    }
}

fn wrong_password() -> impl Fn(&AppError) -> bool {
    |error| {
        matches!(
            error,
            AppError::Identity(IdentityError::InvalidCredentials {
                cause: CredentialFailure::WrongPassword
            })
        )
    }
}

#[tokio::test]
async fn a_right_password_creates_a_session_for_the_token() {
    let fixture = fixture();
    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap();
    let id = SessionId::of(outcome.token());
    let state = fixture.store.state();
    let stored = state.sessions.get(&id).unwrap();
    assert_eq!(stored.user_id, user_id());
    assert_eq!(stored.epoch, 0);
    assert_eq!(stored.csrf_token.to_hex(), outcome.csrf_token().to_hex());
    assert_eq!(outcome.session_ref(), &SessionRef::of(&id));
    assert!(!outcome.must_change_password());
    assert_eq!(fixture.hasher.verify_calls(), 1);
    assert_eq!(fixture.hasher.hash_calls(), 0, "a login hashes nothing");
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn a_right_password_marks_the_attempt_as_a_success() {
    let fixture = fixture();
    Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap();
    let attempts = fixture.store.state().attempts;
    assert_eq!(attempts.len(), 1);
    assert!(attempts.values().all(|row| row.succeeded));
}

#[tokio::test]
async fn a_new_token_and_a_new_csrf_token_come_from_random_bytes() {
    let fixture = fixture();
    let login = Login::new(fixture.context.clone());
    let first = login
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap();
    let second = login
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap();
    assert_ne!(first.token().to_hex(), second.token().to_hex());
    assert_ne!(first.csrf_token().to_hex(), second.csrf_token().to_hex());
}

#[tokio::test]
async fn the_flag_must_change_password_reaches_the_outcome() {
    let fixture = IdentityFixture::new();
    seed_user(&fixture.store, Role::Operator, true);
    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap();
    assert!(outcome.must_change_password());
}

#[tokio::test]
async fn the_session_carries_the_epoch_of_the_locked_row() {
    let fixture = fixture();
    fixture.store.update_state(|state, _now| {
        state.users.get_mut(&user_id()).unwrap().session_epoch = 5;
    });
    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap();
    let state = fixture.store.state();
    let stored = state.sessions.get(&SessionId::of(outcome.token())).unwrap();
    assert_eq!(stored.epoch, 5);
}

#[tokio::test]
async fn the_entry_user_logged_in_holds_the_session_ref() {
    let fixture = fixture();
    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UserLoggedIn);
    assert_eq!(entry.target(), &AuditTarget::app_user(user_id()));
    assert_eq!(entry.actor(), Some(user_id()));
    assert_eq!(entry.before(), None);
    assert_eq!(
        entry.after(),
        Some(&json!({"session_ref": outcome.session_ref().as_str()}))
    );
    assert_eq!(entry.request_id(), request_id());
    assert_eq!(entry.client_ip(), Some(ip()));
}

#[tokio::test]
async fn the_old_session_of_the_request_is_deleted() {
    let fixture = fixture();
    let old = SessionId::of(&recipe_domain::identity::SessionToken::from_bytes([3; 16]));
    fixture.store.seed_session(&NewSession {
        id: old.clone(),
        user_id: user_id(),
        epoch: 0,
        csrf_token: CsrfToken::from_bytes([4; 32]),
        idle_timeout: std::time::Duration::from_secs(60),
    });
    let mut request = input("anna", PASSWORD);
    request.old_session = Some(old.clone());
    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), request)
        .await
        .unwrap();
    let state = fixture.store.state();
    assert!(!state.sessions.contains_key(&old));
    assert!(state.sessions.contains_key(&SessionId::of(outcome.token())));
    assert_eq!(state.sessions.len(), 1);
}

#[tokio::test]
async fn a_wrong_password_gives_e_idn_003_and_a_login_failed_event_of_the_user() {
    let fixture = fixture();
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", "not-the-password"))
        .await
        .unwrap_err();
    assert!(wrong_password()(&error), "{error:?}");
    assert_eq!(error.code(), ErrorCode::IDN_003);
    assert_eq!(fixture.hasher.verify_calls(), 1);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(event.login_hmac(), None);
    assert_eq!(event.client_ip(), Some(ip()));
    assert_eq!(event.request_id(), request_id());
    assert_eq!(event.user_agent(), Some(&UserAgent::from("test-agent")));
    let state = fixture.store.state();
    assert!(state.sessions.is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(state.attempts.len(), 1);
    assert!(state.attempts.values().all(|row| !row.succeeded));
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

/// A fixture with an active `ExportClient` account that has the login `anna`.
fn technical_fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    seed_user(&fixture.store, Role::ExportClient, false);
    fixture
}

#[tokio::test]
async fn a_technical_account_with_the_right_password_gets_the_answer_of_a_wrong_password() {
    let technical = technical_fixture();
    let technical_error = Login::new(technical.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    let person = fixture();
    let person_error = Login::new(person.context.clone())
        .execute(&meta(), input("anna", "not-the-password"))
        .await
        .unwrap_err();
    assert!(wrong_password()(&technical_error), "{technical_error:?}");
    assert_eq!(technical_error.code(), ErrorCode::IDN_003);
    assert_eq!(
        format!("{technical_error:?}"),
        format!("{person_error:?}"),
        "the refusal looks like a wrong password"
    );
    assert_eq!(
        technical.hasher.verify_calls(),
        person.hasher.verify_calls(),
        "the same hash work as a wrong password"
    );
    assert_eq!(technical.hasher.verify_calls(), 1);
    assert_eq!(technical.hasher.hash_calls(), 0);
    assert_eq!(technical.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn a_technical_account_gets_no_session_and_a_failed_attempt() {
    let fixture = technical_fixture();
    Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    let state = fixture.store.state();
    assert!(state.sessions.is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(state.attempts.len(), 1);
    assert!(state.attempts.values().all(|row| !row.succeeded));
}

#[tokio::test]
async fn a_technical_account_writes_login_failed_with_the_reason_technical_account() {
    let fixture = technical_fixture();
    Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(event.login_hmac(), None);
    assert_eq!(event.client_ip(), Some(ip()));
    assert_eq!(event.request_id(), request_id());
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "technical_account"}))
    );
    let shown = format!("{events:?}");
    assert!(!shown.contains(PASSWORD), "{shown}");
}

#[tokio::test]
async fn a_technical_account_with_a_wrong_password_writes_no_reason() {
    let fixture = technical_fixture();
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", "not-the-password"))
        .await
        .unwrap_err();
    assert!(wrong_password()(&error), "{error:?}");
    let events = fixture.sink.events();
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.detail(), None);
}

#[tokio::test]
async fn an_unknown_login_gets_one_dummy_check_and_an_event_with_the_hmac() {
    let fixture = fixture();
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("nobody", PASSWORD))
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::InvalidCredentials {
                cause: CredentialFailure::UnknownLogin
            })
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_003);
    assert_eq!(fixture.hasher.verify_calls(), 1);
    let typed = TypedLogin::parse("nobody").unwrap();
    let expected = LoginHmac::compute(fixture.context.login_keys(), &typed);
    let events = fixture.sink.events();
    let event = events.first().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), None);
    assert_eq!(event.login_hmac(), Some(&expected));
}

#[tokio::test]
async fn a_typed_login_outside_the_grammar_counts_as_unknown() {
    let fixture = fixture();
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("a b!", PASSWORD))
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::InvalidCredentials {
                cause: CredentialFailure::UnknownLogin
            })
        ),
        "{error:?}"
    );
    assert_eq!(fixture.hasher.verify_calls(), 1);
    let events = fixture.sink.events();
    assert!(
        events
            .first()
            .is_some_and(|event| event.login_hmac().is_some())
    );
}

#[tokio::test]
async fn an_inactive_user_gets_one_dummy_check_and_e_idn_004_even_with_the_right_password() {
    let fixture = fixture();
    deactivate(&fixture.store);
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(
        matches!(error, AppError::Identity(IdentityError::UserInactive)),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_004);
    assert_eq!(fixture.hasher.verify_calls(), 1);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginRejectedInactive);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(event.login_hmac(), None);
    assert!(fixture.store.state().sessions.is_empty());
}

#[tokio::test]
async fn four_failures_with_the_last_one_second_ago_give_e_idn_005_with_retry_after_4() {
    let fixture = fixture();
    seed_failures(&fixture.store, &[10, 9, 8, 1]);
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::LoginThrottled { retry_after: 4 })
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_005);
    assert_eq!(fixture.hasher.verify_calls(), 0, "no password is checked");
    assert_eq!(fixture.context.gate().available_permits(), 4);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginThrottled);
    assert_eq!(event.user(), None);
    let typed = TypedLogin::parse("anna").unwrap();
    assert_eq!(
        event.login_hmac(),
        Some(&LoginHmac::compute(fixture.context.login_keys(), &typed))
    );
    assert!(fixture.store.state().sessions.is_empty());
}

#[tokio::test]
async fn a_login_after_the_retry_after_time_checks_the_password() {
    let fixture = fixture();
    seed_failures(&fixture.store, &[10, 9, 8, 1]);
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    let retry_after = match error {
        AppError::Identity(IdentityError::LoginThrottled { retry_after }) => retry_after,
        _ => 0,
    };
    assert!(retry_after >= 1, "the login must be throttled");
    let seconds = i64::try_from(retry_after).unwrap();
    fixture.store.clock().advance(Duration::seconds(seconds));
    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await;
    assert!(outcome.is_ok(), "{outcome:?}");
}

#[tokio::test]
async fn the_attempt_passes_after_the_delay() {
    let fixture = fixture();
    seed_failures(&fixture.store, &[10, 9, 8, 1]);
    fixture.store.clock().advance(Duration::seconds(1));
    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await;
    assert!(outcome.is_ok(), "{outcome:?}");
}

#[tokio::test]
async fn a_used_address_budget_gives_e_idn_005_with_the_rest_of_the_window() {
    let fixture = fixture();
    for number in 0..100 {
        let login = TypedLogin::parse(&format!("user{number}")).unwrap();
        let ago: i64 = if number == 0 { 870 } else { 100 };
        fixture
            .store
            .seed_attempt(&login, ip(), at(ago.checked_neg().unwrap()), false);
    }
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::LoginThrottled { retry_after: 30 })
        ),
        "{error:?}"
    );
    assert_eq!(fixture.store.state().attempts.len(), 100, "no row is added");
}

#[tokio::test(start_paused = true)]
async fn no_free_permit_gives_e_web_009_before_any_transaction() {
    let fixture = fixture();
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(fixture.context.gate().acquire().await.unwrap());
    }
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Timeout {
                stage: TimeoutStage::Argon2Permit
            }
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.state().attempts.is_empty());
    drop(held);
}

#[tokio::test]
async fn a_storage_failure_at_t1_returns_the_error_and_frees_the_permit() {
    let fixture = fixture();
    fixture.store.fail_begin(Fault::Unavailable);
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert_eq!(fixture.context.gate().available_permits(), 4);
    assert_eq!(fixture.hasher.verify_calls(), 0);
}

#[tokio::test]
async fn a_reset_between_the_check_and_t2_gives_e_idn_003_and_no_session() {
    let fixture = racing_fixture(reset);
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(wrong_password()(&error), "{error:?}");
    assert_eq!(error.code(), ErrorCode::IDN_003);
    let state = fixture.store.state();
    assert!(state.sessions.is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert!(state.attempts.values().all(|row| !row.succeeded));
    let row = state.users.get(&user_id()).unwrap();
    assert_eq!(row.password_hash, fake_hash("a-one-time-password"));
    assert_one_event(&fixture, SecurityEventKind::LoginFailed);
}

/// Assert that the sink holds exactly one event of the user with the given kind.
fn assert_one_event(fixture: &IdentityFixture, kind: SecurityEventKind) {
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    let event = events.first().unwrap();
    assert_eq!(event.kind(), kind);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(event.login_hmac(), None);
    assert_eq!(event.request_id(), request_id());
}

#[tokio::test]
async fn an_unlock_between_t1_and_t2_does_not_stop_the_login() {
    let fixture = IdentityFixture::with_race(|store| {
        let login = TypedLogin::parse("anna").unwrap();
        store.update_state(|state, _now| state.attempts.retain(|_id, row| row.login != login));
    });
    seed_user(&fixture.store, Role::Operator, false);
    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await;
    assert!(outcome.is_ok(), "{outcome:?}");
    let state = fixture.store.state();
    assert!(state.attempts.is_empty(), "the unlock removed the row");
    assert_eq!(state.sessions.len(), 1);
}

#[tokio::test]
async fn a_new_epoch_between_the_check_and_t2_gives_e_idn_003_and_no_session() {
    let fixture =
        racing_fixture(|row| row.session_epoch = row.session_epoch.checked_add(1).unwrap());
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(wrong_password()(&error), "{error:?}");
    assert!(fixture.store.state().sessions.is_empty());
    assert_one_event(&fixture, SecurityEventKind::LoginFailed);
}

#[tokio::test]
async fn a_deactivation_between_the_check_and_t2_gives_e_idn_003_and_no_session() {
    let fixture = racing_fixture(|row| row.is_active = false);
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(wrong_password()(&error), "{error:?}");
    assert!(fixture.store.state().sessions.is_empty());
    assert_one_event(&fixture, SecurityEventKind::LoginRejectedInactive);
}

#[tokio::test]
async fn the_outcome_and_the_records_hold_no_secret() {
    let fixture = fixture();
    let login = Login::new(fixture.context.clone());
    let outcome = login
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap();
    let token = outcome.token().to_hex();
    let csrf = outcome.csrf_token().to_hex();
    let shown = format!("{outcome:?}");
    assert!(!shown.contains(&token), "{shown}");
    assert!(!shown.contains(&csrf), "{shown}");
    assert!(shown.contains("[redacted]"), "{shown}");
    let entries = format!("{:?}", fixture.store.audit_entries());
    let id = SessionId::of(outcome.token());
    for secret in [
        token.as_str(),
        csrf.as_str(),
        id.as_str(),
        PASSWORD,
        "argon2id",
    ] {
        assert!(!entries.contains(secret), "{secret}");
    }
    let failure = login
        .execute(&meta(), input("anna", "not-the-password"))
        .await
        .unwrap_err();
    let shown = format!("{failure:?} {failure}");
    assert!(!shown.contains("not-the-password"), "{shown}");
    assert!(!shown.contains("anna"), "{shown}");
    let events = format!("{:?}", fixture.sink.events());
    assert!(!events.contains("not-the-password"), "{events}");
}

#[test]
fn the_operation_login_needs_no_session() {
    assert_eq!(<Login as UseCase>::OPERATION, Operation::Login);
    assert!(require_no_session(Operation::Login).is_ok());
}

#[test]
fn an_operation_with_a_session_requirement_fails_closed() {
    let error = require_no_session(Operation::Logout).unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Forbidden {
                operation: Operation::Logout
            }
        ),
        "{error:?}"
    );
}

/// Poll `future` once and drop it, as a timeout or a closed connection drops a request.
///
/// `Login` takes its permit and its slot, starts its detached task, and waits for the result. The
/// first poll ends at that wait, so the drop leaves the task as the only part that runs on.
async fn poll_once_then_drop<F: Future>(future: F) {
    let mut future = std::pin::pin!(future);
    std::future::poll_fn(|cx| {
        let _pending = future.as_mut().poll(cx);
        Poll::Ready(())
    })
    .await;
}

#[tokio::test]
async fn login_writes_login_failed_after_the_caller_drops_the_future() {
    let fixture = fixture();
    let login = Login::new(fixture.context.clone());

    poll_once_then_drop(login.execute(&meta(), input("anna", "not-the-password"))).await;
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);

    assert_one_event(&fixture, SecurityEventKind::LoginFailed);
    let state = fixture.store.state();
    assert_eq!(state.attempts.len(), 1);
    assert!(state.sessions.is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn login_writes_login_throttled_after_the_caller_drops_the_future() {
    let fixture = fixture();
    seed_failures(&fixture.store, &[10, 9, 8, 1]);
    let login = Login::new(fixture.context.clone());

    poll_once_then_drop(login.execute(&meta(), input("anna", PASSWORD))).await;
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);

    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(
        events.first().unwrap().kind(),
        SecurityEventKind::LoginThrottled
    );
    assert_eq!(fixture.hasher.verify_calls(), 0, "no password is checked");
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn login_creates_no_session_after_the_caller_drops_the_future() {
    let fixture = fixture();
    let login = Login::new(fixture.context.clone());

    poll_once_then_drop(login.execute(&meta(), input("anna", PASSWORD))).await;
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);

    let state = fixture.store.state();
    assert!(
        state.sessions.is_empty(),
        "T2 runs only for a waiting caller"
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(state.attempts.len(), 1);
    assert!(state.attempts.values().all(|row| !row.succeeded));
    assert!(fixture.sink.events().is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn login_refuses_with_one_warn_line_when_the_event_fails() {
    type Check = fn(&AppError) -> bool;
    let wrong: Check = |error| wrong_password()(error);
    let unknown: Check = |error| {
        matches!(
            error,
            AppError::Identity(IdentityError::InvalidCredentials {
                cause: CredentialFailure::UnknownLogin
            })
        )
    };
    let inactive: Check = |error| matches!(error, AppError::Identity(IdentityError::UserInactive));
    let throttled: Check = |error| {
        matches!(
            error,
            AppError::Identity(IdentityError::LoginThrottled { retry_after: 4 })
        )
    };
    let throttled_fixture = fixture();
    seed_failures(&throttled_fixture.store, &[10, 9, 8, 1]);
    let inactive_fixture = fixture();
    deactivate(&inactive_fixture.store);
    let cases = [
        (
            "wrong password",
            fixture(),
            input("anna", "wrong-password"),
            "login_failed",
            wrong,
        ),
        (
            "unknown login",
            fixture(),
            input("nobody", PASSWORD),
            "login_failed",
            unknown,
        ),
        (
            "inactive user",
            inactive_fixture,
            input("anna", PASSWORD),
            "login_rejected_inactive",
            inactive,
        ),
        (
            "throttled attempt",
            throttled_fixture,
            input("anna", PASSWORD),
            "login_throttled",
            throttled,
        ),
        (
            "user changed before T2",
            racing_fixture(reset),
            input("anna", PASSWORD),
            "login_failed",
            wrong,
        ),
    ];
    for (name, fixture, request, kind, answers) in cases {
        let (capture, _guard) = LogCapture::install();
        fixture.sink.fail_with(Fault::Unavailable);

        let error = Login::new(fixture.context.clone())
            .execute(&meta(), request)
            .await
            .unwrap_err();

        assert!(answers(&error), "{name}: {error:?}");
        let lines = capture.events();
        assert_eq!(lines.len(), 1, "{name}: {lines:?}");
        let line = lines.first().unwrap();
        assert_eq!(line.level, Level::WARN, "{name}");
        assert_eq!(
            line.field("event"),
            Some(format!("{kind:?}").as_str()),
            "{name}"
        );
        assert!(line.field("request_id").is_some(), "{name}: {line:?}");
        assert!(fixture.sink.events().is_empty(), "{name}");
        assert_eq!(fixture.context.gate().available_permits(), 4, "{name}");
    }
}

#[tokio::test]
async fn a_recorded_event_writes_no_warn_line() {
    let (capture, _guard) = LogCapture::install();
    let fixture = fixture();

    Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", "not-the-password"))
        .await
        .unwrap_err();

    assert_one_event(&fixture, SecurityEventKind::LoginFailed);
    assert_eq!(capture.events().len(), 0);
}

#[tokio::test]
async fn a_storage_failure_at_t1_in_the_task_returns_the_error_and_frees_the_slot() {
    let fixture = fixture();
    fixture.store.fail_begin(Fault::Unavailable);
    let error = Login::new(fixture.context.clone())
        .execute(&meta(), input("anna", PASSWORD))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
    assert!(fixture.context.detached().acquire().await.is_ok());
}

// ---- log capture ----

/// One event that the code wrote.
#[derive(Clone, Debug)]
struct Logged {
    level: Level,
    fields: Vec<(String, String)>,
}

impl Logged {
    fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// A subscriber that keeps every event of its thread.
#[derive(Clone, Default)]
struct LogCapture {
    events: Arc<Mutex<Vec<Logged>>>,
}

impl LogCapture {
    fn install() -> (Self, DefaultGuard) {
        LazyLock::force(&KEEPALIVE);
        let capture = Self::default();
        let guard = tracing::subscriber::set_default(capture.clone());
        (capture, guard)
    }

    fn events(&self) -> Vec<Logged> {
        self.events.lock().clone()
    }
}

#[derive(Default)]
struct FieldCopy {
    fields: Vec<(String, String)>,
}

impl Visit for FieldCopy {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.fields
            .push((field.name().to_owned(), format!("{value:?}")));
    }
}

impl Subscriber for LogCapture {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut copy = FieldCopy::default();
        event.record(&mut copy);
        self.events.lock().push(Logged {
            level: *event.metadata().level(),
            fields: copy.fields,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
