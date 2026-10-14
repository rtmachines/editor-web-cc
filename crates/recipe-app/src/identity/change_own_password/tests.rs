//! Tests of the use case `ChangeOwnPassword` (task 4.2, design D7, ADR-0035 b and m).

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{
    AuditAction, AuditTarget, LoginHmac, RequestId, SecurityEventKind, SessionRef,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{IdentityError, Password, PasswordReason, SessionId, TypedLogin};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{ChangeOwnPassword, ChangeOwnPasswordInput};
use crate::auth::{Operation, RequestMeta};
use crate::context::UseCase;
use crate::error::{AppError, TimeoutStage};
use crate::identity::authenticate;
use crate::identity::fixtures::{IdentityFixture, anna_id, fake_hash};
use crate::memory::{Fault, MemoryStore};
use crate::ports::UserRow;

const PASSWORD: &str = "correct-horse-battery";
const NEW_PASSWORD: &str = "brand-new-secret-phrase";
const START_SECONDS: i64 = 1_000_000;

fn ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))
}

fn at(offset: i64) -> Timestamp {
    let seconds = START_SECONDS.checked_add(offset).unwrap();
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn meta(session: &SessionId) -> RequestMeta {
    RequestMeta::new(
        RequestId::from(Uuid::from_u128(0x99)),
        Some(ip()),
        None,
        Some(SessionRef::of(session)),
    )
}

fn input(current: &str, new: &str) -> ChangeOwnPasswordInput {
    ChangeOwnPasswordInput {
        client_ip: ip(),
        current_password: Password::parse(current).unwrap(),
        new_password: Password::parse(new).unwrap(),
    }
}

fn fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna(PASSWORD, true);
    fixture
}

fn racing_fixture(change: fn(&mut UserRow)) -> IdentityFixture {
    let fixture = IdentityFixture::with_race(move |store| {
        store.update_state(|state, _now| change(state.users.get_mut(&anna_id()).unwrap()));
    });
    fixture.seed_anna(PASSWORD, false);
    fixture
}

fn reset(row: &mut UserRow) {
    row.password_hash = fake_hash("a-one-time-password");
    row.session_epoch = row.session_epoch.checked_add(1).unwrap();
}

fn seed_failures(store: &MemoryStore, seconds_ago: &[i64]) {
    let login = TypedLogin::parse("anna").unwrap();
    for ago in seconds_ago {
        store.seed_attempt(&login, ip(), at(ago.checked_neg().unwrap()), false);
    }
}

fn user_row(store: &MemoryStore) -> UserRow {
    store.state().users.get(&anna_id()).unwrap().clone()
}

#[test]
fn the_operation_is_change_own_password() {
    assert_eq!(
        <ChangeOwnPassword as UseCase>::OPERATION,
        Operation::ChangeOwnPassword
    );
}

#[tokio::test]
async fn a_success_replaces_the_hash_and_clears_the_flag_and_raises_the_epoch() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap();
    let row = user_row(&fixture.store);
    assert_eq!(row.password_hash, fake_hash(NEW_PASSWORD));
    assert!(!row.must_change_password);
    assert_eq!(row.session_epoch, 1);
    assert_eq!(fixture.hasher.verify_calls(), 1);
    assert_eq!(fixture.hasher.hash_calls(), 1);
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn a_success_deletes_every_session_and_creates_one_new_session() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let other = fixture.seed_anna_session(2);
    let auth = fixture.authenticated(&token).await;
    let issued = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap();
    let state = fixture.store.state();
    assert_eq!(state.sessions.len(), 1);
    assert!(!state.sessions.contains_key(&SessionId::of(&token)));
    assert!(!state.sessions.contains_key(&SessionId::of(&other)));
    let id = SessionId::of(issued.token());
    let stored = state.sessions.get(&id).unwrap();
    assert_eq!(stored.user_id, anna_id());
    assert_eq!(stored.epoch, 1);
    assert_eq!(stored.csrf_token.to_hex(), issued.csrf_token().to_hex());
    assert_ne!(issued.csrf_token().to_hex(), auth.csrf_token().to_hex());
    assert_eq!(issued.session_ref(), &SessionRef::of(&id));
    let renewed = authenticate(&fixture.context, issued.token())
        .await
        .unwrap()
        .unwrap();
    assert!(!renewed.must_change_password());
}

#[tokio::test]
async fn a_success_writes_own_password_changed_without_a_hash() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::OwnPasswordChanged);
    assert_eq!(entry.target(), &AuditTarget::app_user(anna_id()));
    assert_eq!(entry.actor(), Some(anna_id()));
    let before = entry.before().unwrap();
    let after = entry.after().unwrap();
    assert_eq!(before.get("session_epoch"), Some(&0.into()));
    assert_eq!(after.get("session_epoch"), Some(&1.into()));
    assert_eq!(before.get("must_change_password"), Some(&true.into()));
    assert_eq!(after.get("must_change_password"), Some(&false.into()));
    let shown = format!("{entries:?}");
    for secret in ["password_hash", "argon2id", PASSWORD, NEW_PASSWORD] {
        assert!(!shown.contains(secret), "{secret}");
    }
}

#[tokio::test]
async fn a_success_marks_the_attempt_of_t1_as_a_success() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap();
    let attempts = fixture.store.state().attempts;
    assert_eq!(attempts.len(), 1);
    assert!(attempts.values().all(|row| row.succeeded));
    assert!(attempts.values().all(|row| row.login.as_str() == "anna"));
}

#[tokio::test]
async fn a_new_password_outside_the_policy_gives_e_idn_006_with_no_attempt_and_no_hash() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let journal_before = fixture.store.journal().len();
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, "short-pw-11"),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::PasswordPolicy {
                reason: PasswordReason::TooShort
            })
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_006);
    assert!(fixture.store.state().attempts.is_empty());
    assert_eq!(fixture.hasher.verify_calls(), 0);
    assert_eq!(fixture.hasher.hash_calls(), 0);
    assert_eq!(
        fixture.store.journal().len(),
        journal_before,
        "no transaction"
    );
}

#[tokio::test(start_paused = true)]
async fn the_policy_error_comes_before_the_wait_for_a_permit() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(fixture.context.gate().acquire().await.unwrap());
    }
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, "short-pw-11"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::IDN_006);
    drop(held);
}

#[tokio::test(start_paused = true)]
async fn no_free_permit_gives_e_web_009_and_inserts_no_attempt() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(fixture.context.gate().acquire().await.unwrap());
    }
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
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
    assert!(fixture.store.state().attempts.is_empty());
    drop(held);
}

#[tokio::test]
async fn a_throttled_change_gives_e_idn_005_and_checks_no_password() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    seed_failures(&fixture.store, &[10, 9, 8, 1]);
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
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
    assert_eq!(fixture.hasher.verify_calls(), 0);
    assert_eq!(fixture.hasher.hash_calls(), 0);
    assert_eq!(fixture.context.gate().available_permits(), 4);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginThrottled);
    let typed = TypedLogin::parse("anna").unwrap();
    assert_eq!(
        event.login_hmac(),
        Some(&LoginHmac::compute(fixture.context.login_keys(), &typed))
    );
    assert_eq!(event.client_ip(), Some(ip()));
    assert_eq!(user_row(&fixture.store).password_hash, fake_hash(PASSWORD));
}

#[tokio::test]
async fn a_failed_throttle_event_write_returns_the_storage_error() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    seed_failures(&fixture.store, &[10, 9, 8, 1]);
    fixture.sink.fail_with(Fault::Unavailable);
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
}

#[tokio::test]
async fn a_wrong_current_password_gives_e_idn_008_and_one_failed_attempt() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input("wrong-current-pw", NEW_PASSWORD),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::InvalidCurrentPassword)
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_008);
    let state = fixture.store.state();
    assert_eq!(state.attempts.len(), 1);
    assert!(state.attempts.values().all(|row| !row.succeeded));
    assert_eq!(fixture.hasher.verify_calls(), 1);
    assert_eq!(fixture.hasher.hash_calls(), 0);
    assert_eq!(user_row(&fixture.store).password_hash, fake_hash(PASSWORD));
    assert!(state.sessions.contains_key(&SessionId::of(&token)));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn a_reset_between_the_check_and_the_commit_gives_e_web_004_and_keeps_the_reset_hash() {
    let fixture = racing_fixture(reset);
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unauthenticated), "{error:?}");
    assert_eq!(error.code(), ErrorCode::WEB_004);
    let row = user_row(&fixture.store);
    assert_eq!(row.password_hash, fake_hash("a-one-time-password"));
    assert_eq!(row.session_epoch, 1);
    let state = fixture.store.state();
    assert_eq!(state.sessions.len(), 1, "no new session");
    assert!(state.attempts.values().all(|attempt| !attempt.succeeded));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unlock_between_the_check_and_the_commit_does_not_stop_the_change() {
    let fixture = IdentityFixture::with_race(|store| {
        let login = TypedLogin::parse("anna").unwrap();
        store.update_state(|state, _now| state.attempts.retain(|_id, row| row.login != login));
    });
    fixture.seed_anna(PASSWORD, false);
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let outcome = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await;
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(
        user_row(&fixture.store).password_hash,
        fake_hash(NEW_PASSWORD)
    );
    assert!(fixture.store.state().attempts.is_empty());
}

#[tokio::test]
async fn a_new_epoch_alone_between_the_check_and_the_commit_gives_e_web_004() {
    let fixture =
        racing_fixture(|row| row.session_epoch = row.session_epoch.checked_add(1).unwrap());
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unauthenticated), "{error:?}");
    assert_eq!(user_row(&fixture.store).password_hash, fake_hash(PASSWORD));
}

#[tokio::test]
async fn a_deactivation_between_the_check_and_the_commit_gives_e_web_004() {
    let fixture = racing_fixture(|row| row.is_active = false);
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unauthenticated), "{error:?}");
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_storage_failure_at_t1_returns_the_error_and_frees_the_permit() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    fixture.store.fail_begin(Fault::Unavailable);
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert_eq!(fixture.context.gate().available_permits(), 4);
    assert_eq!(fixture.hasher.verify_calls(), 0);
}

#[tokio::test]
async fn a_failed_commit_keeps_the_old_password_and_the_old_sessions() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    fixture.store.fail_finish(Fault::Unavailable);
    let error = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    let row = user_row(&fixture.store);
    assert_eq!(row.password_hash, fake_hash(PASSWORD));
    assert_eq!(row.session_epoch, 0);
    let state = fixture.store.state();
    assert!(state.sessions.contains_key(&SessionId::of(&token)));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_issued_session_hides_its_secrets_in_debug() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let issued = ChangeOwnPassword::new(fixture.context.clone())
        .execute(
            &auth,
            &meta(auth.session_id()),
            input(PASSWORD, NEW_PASSWORD),
        )
        .await
        .unwrap();
    let shown = format!("{issued:?}");
    assert!(!shown.contains(&issued.token().to_hex()), "{shown}");
    assert!(!shown.contains(&issued.csrf_token().to_hex()), "{shown}");
    assert!(shown.contains("[redacted]"), "{shown}");
}
