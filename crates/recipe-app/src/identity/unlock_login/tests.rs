//! Tests of the use case `UnlockLogin` (task 4.5, design D8 i, j, and q).

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::audit::{AuditAction, AuditTarget, RequestId, SecurityEvent, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{Login, Password, Role, TypedLogin};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::sync::Notify;
use tokio::time::Instant;
use tracing::Level;
use uuid::Uuid;

use super::UnlockLogin;
use crate::auth::{Operation, RequestMeta};
use crate::context::UseCase;
use crate::error::{AppError, Missing, StorageError};
use crate::identity::fixtures::{DRAIN_WAIT, IdentityFixture, LogCapture, actor_id, anna_id};
use crate::identity::{IdentityContext, Login as LoginUseCase, LoginInput};
use crate::memory::{Fault, MemorySink};
use crate::ports::SecurityEventSink;

/// The time that the caller waits before a test drops its future.
const CALLER_WAIT: Duration = Duration::from_millis(10);

/// The start of the clock of a fixture, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;

fn admin_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9))
}

fn failing_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))
}

fn anna() -> Login {
    Login::parse("anna").unwrap()
}

fn typed(text: &str) -> TypedLogin {
    TypedLogin::parse(text).unwrap()
}

fn at(offset: i64) -> Timestamp {
    let seconds = START_SECONDS.checked_add(offset).unwrap();
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    fixture
}

/// Seed `count` failed attempts of `login`, one second apart. The newest has the highest key.
fn seed_failures(fixture: &IdentityFixture, login: &str, count: i64) {
    for offset in 0..count {
        fixture
            .store
            .seed_attempt(&typed(login), failing_ip(), at(offset), false);
    }
}

fn attempts_of(fixture: &IdentityFixture, login: &str) -> usize {
    let login = typed(login);
    fixture
        .store
        .state()
        .attempts
        .values()
        .filter(|row| row.login == login)
        .count()
}

#[test]
fn the_operation_is_unlock_login() {
    assert_eq!(<UnlockLogin as UseCase>::OPERATION, Operation::UnlockLogin);
}

#[tokio::test]
async fn a_success_deletes_every_attempt_of_the_login_and_keeps_the_others() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 5);
    seed_failures(&fixture, "berta", 2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    assert_eq!(attempts_of(&fixture, "anna"), 0);
    assert_eq!(attempts_of(&fixture, "berta"), 2);
}

#[tokio::test]
async fn the_entry_login_unlocked_holds_the_deleted_attempts_and_an_empty_array() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::LoginUnlocked);
    assert_eq!(entry.target(), &AuditTarget::app_user(anna_id()));
    assert_eq!(entry.actor(), Some(actor_id()));
    let before = entry.before().unwrap();
    assert_eq!(before["login"], "anna");
    assert_eq!(before["login_attempt_count"], 2);
    let listed = before["login_attempts"].as_array().unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed.first().unwrap()["client_ip"], "10.0.0.5");
    assert_eq!(listed.first().unwrap()["succeeded"], false);
    let after = entry.after().unwrap();
    assert_eq!(after["login"], "anna");
    assert_eq!(after["login_attempt_count"], 0);
    assert!(after["login_attempts"].as_array().unwrap().is_empty());
    let shown = format!("{entries:?}");
    assert!(!shown.contains("password_hash"), "{shown}");
    assert!(!shown.contains("argon2id"), "{shown}");
}

#[tokio::test]
async fn the_attempted_at_of_each_listed_attempt_is_rfc_3339_text() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 3);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    let before = entries.first().unwrap().before().unwrap();
    let listed = before["login_attempts"].as_array().unwrap();
    assert_eq!(listed.len(), 3);
    for attempt in listed {
        let text = attempt["attempted_at"].as_str().unwrap();
        assert!(
            OffsetDateTime::parse(text, &Rfc3339).is_ok(),
            "{text:?} is no RFC 3339 text"
        );
    }
}

#[tokio::test]
async fn the_image_holds_the_newest_100_of_250_attempts_and_the_full_count() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 250);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    let before = entries.first().unwrap().before().unwrap();
    assert_eq!(before["login_attempt_count"], 250);
    let listed = before["login_attempts"].as_array().unwrap();
    assert_eq!(listed.len(), 100);
    assert_eq!(listed.first().unwrap()["id"], 250);
    assert_eq!(listed.last().unwrap()["id"], 151);
    assert_eq!(attempts_of(&fixture, "anna"), 0);
}

#[tokio::test]
async fn the_event_unlock_names_the_target_the_actor_and_the_address() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 1);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::Unlock);
    assert_eq!(event.user(), Some(anna_id()));
    assert_eq!(event.client_ip(), Some(admin_ip()));
    assert_eq!(event.request_id(), meta.request_id());
    assert_eq!(
        event.detail().unwrap()["unlocked_by"],
        actor_id().as_uuid().to_string()
    );
}

#[tokio::test]
async fn a_repeated_unlock_deletes_nothing_more_and_commits_a_count_of_0() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 3);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let unlock = UnlockLogin::new(fixture.context.clone());
    unlock
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    unlock
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 2);
    let second = entries.last().unwrap().before().unwrap();
    assert_eq!(second["login_attempt_count"], 0);
}

#[tokio::test]
async fn an_unknown_login_gives_e_web_005_and_writes_nothing() {
    let fixture = fixture();
    seed_failures(&fixture, "nobody", 2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let nobody = Login::parse("nobody").unwrap();
    let error = UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &nobody, admin_ip())
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::NotFound { entity: Missing::User(login) } if login == &nobody
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_eq!(attempts_of(&fixture, "nobody"), 2);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn the_permission_check_comes_first_for_an_operator_and_a_process_engineer() {
    for role in [Role::Operator, Role::ProcessEngineer] {
        for login in ["anna", "nobody"] {
            let fixture = fixture();
            seed_failures(&fixture, "anna", 3);
            let (auth, meta) = fixture.actor(role).await;
            let journal_before = fixture.store.journal().len();
            let error = UnlockLogin::new(fixture.context.clone())
                .execute(&auth, &meta, &Login::parse(login).unwrap(), admin_ip())
                .await
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    AppError::Forbidden {
                        operation: Operation::UnlockLogin
                    }
                ),
                "{error:?}"
            );
            assert_eq!(error.code(), ErrorCode::WEB_002);
            let events = fixture.sink.events();
            assert_eq!(events.len(), 1, "{role:?}");
            assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
            assert_eq!(fixture.store.journal().len(), journal_before, "{role:?}");
            assert_eq!(attempts_of(&fixture, "anna"), 3, "{role:?}");
        }
    }
}

#[tokio::test]
async fn a_failed_unlock_event_completes_the_unlock_and_writes_one_warn_line() {
    let (capture, _guard) = LogCapture::install();
    let fixture = fixture();
    seed_failures(&fixture, "anna", 4);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.sink.fail_with(Fault::Unavailable);
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    assert_eq!(attempts_of(&fixture, "anna"), 0);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries.first().unwrap().action(),
        AuditAction::LoginUnlocked
    );
    assert!(fixture.sink.events().is_empty());
    assert_one_lost_event_line(&capture, &meta);
}

#[tokio::test(start_paused = true)]
async fn unlock_writes_its_event_after_the_caller_drops_the_future() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 4);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let gated = GatedSink::new(fixture.sink.clone());
    let unlock = UnlockLogin::new(gated.install(&fixture));

    let dropped = tokio::time::timeout(
        CALLER_WAIT,
        unlock.execute(&auth, &meta, &anna(), admin_ip()),
    )
    .await;
    assert!(dropped.is_err(), "the caller must time out first");
    assert!(fixture.sink.events().is_empty());

    gated.release();
    assert_eq!(unlock.ctx.detached().drain(DRAIN_WAIT).await, 0);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Unlock);
    assert_eq!(attempts_of(&fixture, "anna"), 0);
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn unlock_writes_exactly_one_warn_line_when_the_event_fails_after_a_drop() {
    let (capture, _guard) = LogCapture::install();
    let fixture = fixture();
    seed_failures(&fixture, "anna", 4);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.sink.fail_with(Fault::Unavailable);
    let gated = GatedSink::new(fixture.sink.clone());
    let unlock = UnlockLogin::new(gated.install(&fixture));

    let dropped = tokio::time::timeout(
        CALLER_WAIT,
        unlock.execute(&auth, &meta, &anna(), admin_ip()),
    )
    .await;
    assert!(dropped.is_err(), "the caller must time out first");
    assert_eq!(capture.count(), 0, "the event write has not run yet");

    gated.release();
    assert_eq!(unlock.ctx.detached().drain(DRAIN_WAIT).await, 0);
    assert_one_lost_event_line(&capture, &meta);
    assert!(fixture.sink.events().is_empty());
    assert_eq!(attempts_of(&fixture, "anna"), 0);
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

/// Start `count` logins. Each one waits for an Argon2 permit that the test holds.
fn start_waiting_logins(
    fixture: &IdentityFixture,
    count: usize,
) -> Vec<tokio::task::JoinHandle<Result<(), AppError>>> {
    (0..count)
        .map(|index| {
            let login = LoginUseCase::new(fixture.context.clone());
            let meta = RequestMeta::new(
                RequestId::from(Uuid::from_u128(0x100 + index as u128)),
                None,
                None,
                None,
            );
            let input = LoginInput {
                login: typed("anna"),
                password: Password::parse("wrong-password-phrase").unwrap(),
                client_ip: failing_ip(),
                old_session: None,
            };
            tokio::spawn(async move { login.execute(&meta, input).await.map(drop) })
        })
        .collect()
}

#[tokio::test(start_paused = true)]
async fn unlock_gets_a_slot_while_logins_wait_for_a_permit() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 4);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let gate = &fixture.context.gate;
    let mut held = Vec::new();
    for _ in 0..gate.available_permits() {
        held.push(gate.acquire().await.unwrap());
    }
    let slots = fixture.context.settings.detached_tasks;
    let logins = start_waiting_logins(&fixture, slots.checked_add(2).unwrap());
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    assert!(logins.iter().all(|login| !login.is_finished()));

    let started = Instant::now();
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let waited = started.elapsed();
    let limit = fixture.context.settings.argon2_acquire_timeout;
    assert!(waited < limit, "the unlock waited {waited:?} for a slot");
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Unlock);
    assert_eq!(attempts_of(&fixture, "anna"), 0);

    drop(held);
    for login in logins {
        let _outcome = login.await.unwrap();
    }
    assert_eq!(fixture.context.detached().drain(DRAIN_WAIT).await, 0);
}

#[tokio::test]
async fn a_failed_commit_deletes_nothing_and_writes_no_event() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 4);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.store.fail_finish(Fault::Unavailable);
    let error = UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert_eq!(attempts_of(&fixture, "anna"), 4);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}

/// Assert that the capture holds the one `WARN` line of a lost `unlock` event.
fn assert_one_lost_event_line(capture: &LogCapture, meta: &RequestMeta) {
    let lines = capture.events();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = lines.first().unwrap();
    assert_eq!(line.level, Level::WARN);
    assert_eq!(line.field("event"), Some("\"unlock\""));
    assert!(line.field("error_code").is_some(), "{line:?}");
    assert!(line.field("chain").is_some(), "{line:?}");
    let request_id = line.field("request_id").unwrap();
    assert!(
        request_id.contains(&meta.request_id().to_string()),
        "{line:?}"
    );
}

// ---- a sink that waits for the test ----

/// A sink that holds every `record` call until the test releases it.
///
/// The type solves one problem: a test must drop the future of the caller while the use case
/// awaits the security event.
#[derive(Clone)]
struct GatedSink {
    inner: MemorySink,
    release: Arc<Notify>,
}

impl GatedSink {
    fn new(inner: MemorySink) -> Self {
        Self {
            inner,
            release: Arc::new(Notify::new()),
        }
    }

    /// Return a copy of the context of `fixture` whose sink is this one.
    fn install(&self, fixture: &IdentityFixture) -> IdentityContext {
        let mut context = fixture.context.clone();
        context.app.security = Arc::new(self.clone());
        context
    }

    fn release(&self) {
        self.release.notify_one();
    }
}

#[async_trait]
impl SecurityEventSink for GatedSink {
    async fn record(&self, event: SecurityEvent) -> Result<(), StorageError> {
        self.release.notified().await;
        self.inner.record(event).await
    }
}
