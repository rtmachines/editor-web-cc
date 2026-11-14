//! Tests of the use case `ChangeOwnPassword` (task 4.2, design D7, ADR-0035 b and m).

use std::fmt::Debug;
use std::future::Future as _;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, LazyLock};
use std::task::Poll;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::{
    AuditAction, AuditTarget, LoginHmac, RequestId, SecurityEvent, SecurityEventKind, SessionRef,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{IdentityError, NewPassword, Password, SessionId, TypedLogin};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use uuid::Uuid;

use super::{ChangeOwnPassword, ChangeOwnPasswordInput};
use crate::auth::{Operation, RequestMeta};
use crate::context::UseCase;
use crate::error::{AppError, StorageError, TimeoutStage};
use crate::identity::fixtures::{IdentityFixture, anna_id, fake_hash};
use crate::identity::{Authenticated, IdentityContext, authenticate};
use crate::memory::{Fault, MemorySink, MemoryStore};
use crate::ports::{SecurityEventSink, UserRow};

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
        new_password: NewPassword::parse(new).unwrap(),
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
async fn a_failed_throttle_event_write_gives_the_throttle_answer_and_one_warn_line() {
    let (capture, _guard) = LogCapture::install();
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
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::LoginThrottled { retry_after: 4 })
        ),
        "{error:?}"
    );
    assert!(fixture.sink.events().is_empty());
    let lines = capture.events();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = lines.first().unwrap();
    assert_eq!(line.level, Level::WARN);
    assert_eq!(line.field("event"), Some("\"login_throttled\""));
    assert!(line.field("request_id").is_some(), "{line:?}");
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn throttled_change_writes_its_event_after_the_caller_drops_the_future() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    seed_failures(&fixture.store, &[10, 9, 8, 1]);
    let context = yielding_context(&fixture);

    start_and_drop(&context, &auth, PASSWORD).await;

    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginThrottled);
    assert_eq!(event.client_ip(), Some(ip()));
    assert_eq!(context.gate().available_permits(), 4);
    assert_eq!(fixture.hasher.verify_calls(), 0);
}

#[tokio::test]
async fn change_keeps_the_old_password_after_the_caller_drops_the_future() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;

    start_and_drop(&fixture.context, &auth, PASSWORD).await;

    let row = user_row(&fixture.store);
    assert_eq!(row.password_hash, fake_hash(PASSWORD));
    assert_eq!(row.session_epoch, 0);
    assert!(row.must_change_password);
    let state = fixture.store.state();
    assert!(state.sessions.contains_key(&SessionId::of(&token)));
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(state.attempts.len(), 1, "T1 committed its attempt");
    assert!(state.attempts.values().all(|attempt| !attempt.succeeded));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.hasher.hash_calls(), 0, "no hash for a gone client");
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test(start_paused = true)]
async fn no_free_detached_slot_gives_e_web_009_and_inserts_no_attempt() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let mut held = Vec::new();
    for _ in 0..fixture.context.settings.detached_tasks {
        held.push(fixture.context.detached().acquire().await.unwrap());
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
                stage: TimeoutStage::DetachedSlot
            }
        ),
        "{error:?}"
    );
    assert!(fixture.store.state().attempts.is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
    drop(held);
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

// ---- helpers for a dropped request ----

/// A sink that pauses once before it writes, so a request that waits for it can be dropped.
struct YieldingSink {
    inner: MemorySink,
}

#[async_trait]
impl SecurityEventSink for YieldingSink {
    async fn record(&self, event: SecurityEvent) -> Result<(), StorageError> {
        tokio::task::yield_now().await;
        self.inner.record(event).await
    }
}

/// Return the context of `fixture` with a sink that pauses before each write.
fn yielding_context(fixture: &IdentityFixture) -> IdentityContext {
    let mut context = fixture.context.clone();
    context.app.security = Arc::new(YieldingSink {
        inner: fixture.sink.clone(),
    });
    context
}

/// Start the change, drop its future at the first pause, and wait for the detached task.
async fn start_and_drop(context: &IdentityContext, auth: &Authenticated, current: &str) {
    let use_case = ChangeOwnPassword::new(context.clone());
    let request_meta = meta(auth.session_id());
    let mut request = Box::pin(use_case.execute(auth, &request_meta, input(current, NEW_PASSWORD)));
    let polled = std::future::poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx))).await;
    assert!(
        polled.is_pending(),
        "the request must still wait: {polled:?}"
    );
    drop(request);
    assert_eq!(context.detached().drain(Duration::from_secs(5)).await, 0);
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
