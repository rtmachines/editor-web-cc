//! Tests of `authenticate_basic`: steps 2 to 7 of design D2 (tasks 3.4 and 3.5).

use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use parking_lot::Mutex;
use recipe_domain::audit::{LoginHmac, RequestId, SecurityEvent, SecurityEventKind, UserAgent};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{
    CredentialFailure, IdentityError, KeyRing, Login as UserLogin, NewPassword, Password,
    PasswordHash, Role, TypedLogin, UserId,
};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{BasicCredentials, BasicPrincipal, authenticate_basic};
use crate::auth::RequestMeta;
use crate::blocking::AppSettings;
use crate::context::AppContext;
use crate::error::AppError;
use crate::identity::fixtures::fake_hash;
use crate::identity::{
    Argon2Params, BlockingHasher, HashFailure, HashGate, IdentityContext, IdentitySettings,
    PasswordHasher,
};
use crate::memory::{MemoryClock, MemorySink, MemoryStore};
use crate::ports::NewUser;

const PASSWORD: &str = "correct-horse-battery";
const WRONG_PASSWORD: &str = "not-the-password";
const START_SECONDS: i64 = 1_000_000;
const LOGIN_KEY: &str = "0101010101010101010101010101010101010101010101010101010101010101";
const PERSON_ROLES: [Role; 3] = [Role::Administrator, Role::ProcessEngineer, Role::Operator];

fn user_id() -> UserId {
    UserId::from(Uuid::from_u128(11))
}

fn ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))
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

fn credentials(login: &str, password: &str) -> BasicCredentials {
    BasicCredentials {
        login: TypedLogin::parse(login).unwrap(),
        password: Password::parse(password).unwrap(),
    }
}

/// One call to `verify`: the parameters of the verifier and whether the dummy hash was used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Call {
    params: Argon2Params,
    dummy: bool,
}

/// A change that a test makes to the store while the hash check runs.
type DuringCheck = Box<dyn FnOnce() + Send>;

/// A verifier that records each call. It passes only the hash of `fake_hash`.
///
/// A test can set one change that runs inside the next call, so that a reset or a deactivation
/// happens between T1 and T2.
struct RecordingHasher {
    params: Argon2Params,
    calls: Mutex<Vec<Call>>,
    during_check: Mutex<Option<DuringCheck>>,
}

impl fmt::Debug for RecordingHasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecordingHasher")
            .field("params", &self.params)
            .field("calls", &self.calls)
            .finish_non_exhaustive()
    }
}

impl RecordingHasher {
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().clone()
    }

    fn change_during_check(&self, change: impl FnOnce() + Send + 'static) {
        *self.during_check.lock() = Some(Box::new(change));
    }
}

impl PasswordHasher for RecordingHasher {
    fn hash(&self, password: &NewPassword) -> Result<PasswordHash, HashFailure> {
        Ok(fake_hash(password.as_str()))
    }

    fn verify(&self, password: &Password, hash: Option<&PasswordHash>) -> bool {
        self.calls.lock().push(Call {
            params: self.params,
            dummy: hash.is_none(),
        });
        let change = self.during_check.lock().take();
        if let Some(change) = change {
            change();
        }
        hash.is_some_and(|stored| stored.as_str() == fake_hash(password.as_str()).as_str())
    }
}

struct Fixture {
    store: MemoryStore,
    sink: MemorySink,
    hasher: Arc<RecordingHasher>,
    context: IdentityContext,
}

impl Fixture {
    /// Build a context over a memory store with a recording verifier and 4 Argon2 permits.
    fn new() -> Self {
        let start = OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap();
        let clock = MemoryClock::new(start);
        let store = MemoryStore::with_clock(clock.clone());
        let sink = store.sink();
        let settings = IdentitySettings::default();
        let app = AppContext::new(
            Arc::new(store.clone()),
            Arc::new(sink.clone()),
            Arc::new(clock),
            AppSettings::default(),
        );
        let hasher = Arc::new(RecordingHasher {
            params: Argon2Params::default(),
            calls: Mutex::new(Vec::new()),
            during_check: Mutex::new(None),
        });
        let gate = HashGate::new(4, settings.argon2_acquire_timeout, hasher.clone());
        let context = IdentityContext::new(
            app,
            Arc::new(store.clone()),
            Arc::new(store.clone()),
            gate,
            KeyRing::parse(LOGIN_KEY).unwrap(),
            settings,
        );
        Self {
            store,
            sink,
            hasher,
            context,
        }
    }

    /// Build a fixture with the account `anna` of `role`.
    fn with_anna(role: Role) -> Self {
        let fixture = Self::new();
        fixture.store.seed_user(&NewUser {
            id: user_id(),
            login: UserLogin::parse("anna").unwrap(),
            password_hash: fake_hash(PASSWORD),
            role,
            must_change_password: false,
        });
        fixture
    }

    fn deactivate(&self) {
        deactivate_user(&self.store);
    }

    async fn check(&self, login: &str, password: &str) -> Result<BasicPrincipal, AppError> {
        authenticate_basic(&self.context, credentials(login, password), &meta()).await
    }

    /// Run a check that must fail. Assert the facts that every refusal shares. Return the error
    /// and the one security event.
    async fn refused(&self, login: &str, password: &str, dummy: bool) -> (AppError, SecurityEvent) {
        let error = self.check(login, password).await.unwrap_err();
        assert_eq!(
            self.hasher.calls(),
            vec![Call {
                params: Argon2Params::default(),
                dummy
            }],
            "one hash check with the same parameters"
        );
        let events = self.sink.events();
        assert_eq!(events.len(), 1, "{events:?}");
        let event = events.into_iter().next().unwrap();
        assert_eq!(event.client_ip(), Some(ip()));
        assert_eq!(event.request_id(), request_id());
        assert_eq!(event.user_agent(), Some(&UserAgent::from("test-agent")));
        let state = self.store.state();
        assert!(state.sessions.is_empty());
        assert_eq!(state.attempts.len(), 1);
        assert!(state.attempts.values().all(|row| !row.succeeded));
        assert!(self.store.audit_entries().is_empty());
        assert_eq!(self.context.gate().available_permits(), 4);
        (error, event)
    }

    fn hmac_of(&self, login: &str) -> LoginHmac {
        let typed = TypedLogin::parse(login).unwrap();
        LoginHmac::compute(self.context.login_keys(), &typed)
    }
}

fn deactivate_user(store: &MemoryStore) {
    store.update_state(|state, _now| {
        state.users.get_mut(&user_id()).unwrap().is_active = false;
    });
}

fn reset_password(store: &MemoryStore) {
    store.update_state(|state, _now| {
        state.users.get_mut(&user_id()).unwrap().password_hash = fake_hash("a-new-password-1");
    });
}

/// Assert that `error` is the client answer of every refusal: `E-IDN-003`.
fn assert_invalid_credentials(error: &AppError) {
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::InvalidCredentials { .. })
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_003);
}

#[tokio::test]
async fn an_unknown_login_gets_the_dummy_hash_and_e_idn_003() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let (error, event) = fixture.refused("nobody", PASSWORD, true).await;
    assert_invalid_credentials(&error);
    assert!(matches!(
        error,
        AppError::Identity(IdentityError::InvalidCredentials {
            cause: CredentialFailure::UnknownLogin
        })
    ));
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), None);
    assert_eq!(event.login_hmac(), Some(&fixture.hmac_of("nobody")));
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "unknown_login", "scheme": "basic"}))
    );
}

#[tokio::test]
async fn a_login_outside_the_grammar_counts_as_unknown() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let (error, event) = fixture.refused("a b!", PASSWORD, true).await;
    assert_invalid_credentials(&error);
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), None);
    assert_eq!(event.login_hmac(), Some(&fixture.hmac_of("a b!")));
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "unknown_login", "scheme": "basic"}))
    );
}

#[tokio::test]
async fn an_inactive_account_gets_the_dummy_hash_and_e_idn_003_with_the_right_password() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    fixture.deactivate();
    let (error, event) = fixture.refused("anna", PASSWORD, true).await;
    assert_invalid_credentials(&error);
    assert_eq!(event.kind(), SecurityEventKind::LoginRejectedInactive);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(event.login_hmac(), None);
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "inactive", "scheme": "basic"}))
    );
}

#[tokio::test]
async fn a_wrong_password_checks_the_stored_hash_and_gives_e_idn_003() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let (error, event) = fixture.refused("anna", WRONG_PASSWORD, false).await;
    assert_invalid_credentials(&error);
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(event.login_hmac(), None);
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "wrong_password", "scheme": "basic"}))
    );
}

#[tokio::test]
async fn a_person_account_with_the_right_password_is_refused_after_the_hash_check() {
    for role in PERSON_ROLES {
        let fixture = Fixture::with_anna(role);
        let (error, event) = fixture.refused("anna", PASSWORD, false).await;
        assert_invalid_credentials(&error);
        assert_eq!(event.kind(), SecurityEventKind::LoginFailed, "{role:?}");
        assert_eq!(event.user(), Some(user_id()), "{role:?}");
        assert_eq!(event.login_hmac(), None, "{role:?}");
        assert_eq!(
            event.detail(),
            Some(&json!({"reason": "person_account", "scheme": "basic"})),
            "{role:?}"
        );
    }
}

#[tokio::test]
async fn a_person_account_with_a_wrong_password_gives_the_reason_wrong_password() {
    for role in PERSON_ROLES {
        let fixture = Fixture::with_anna(role);
        let (error, event) = fixture.refused("anna", WRONG_PASSWORD, false).await;
        assert_invalid_credentials(&error);
        assert_eq!(
            event.detail(),
            Some(&json!({"reason": "wrong_password", "scheme": "basic"})),
            "{role:?}"
        );
    }
}

#[tokio::test]
async fn every_refusal_gives_the_same_answer_to_the_client() {
    let mut cases = vec![
        (Role::ExportClient, "nobody", PASSWORD, false),
        (Role::ExportClient, "a b!", PASSWORD, false),
        (Role::ExportClient, "anna", PASSWORD, true),
        (Role::ExportClient, "anna", WRONG_PASSWORD, false),
    ];
    cases.extend(PERSON_ROLES.map(|role| (role, "anna", PASSWORD, false)));
    let mut answers = Vec::new();
    for (role, login, password, inactive) in cases {
        let fixture = Fixture::with_anna(role);
        if inactive {
            fixture.deactivate();
        }
        let error = fixture.check(login, password).await.unwrap_err();
        answers.push((error.code(), error.to_string()));
        assert_eq!(fixture.hasher.calls().len(), 1, "{role:?} {login}");
    }
    let first = answers.first().unwrap();
    assert!(answers.iter().all(|answer| answer == first), "{answers:?}");
}

#[tokio::test]
async fn a_throttled_check_gives_e_idn_005_and_runs_no_hash() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let login = TypedLogin::parse("anna").unwrap();
    for ago in [10_i64, 9, 8, 1] {
        let seconds = START_SECONDS.checked_sub(ago).unwrap();
        let at = OffsetDateTime::from_unix_timestamp(seconds).unwrap();
        fixture.store.seed_attempt(&login, ip(), at.into(), false);
    }
    let error = fixture.check("anna", PASSWORD).await.unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Identity(IdentityError::LoginThrottled { retry_after: 4 })
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_005);
    assert!(fixture.hasher.calls().is_empty(), "no password is checked");
    assert_eq!(fixture.context.gate().available_permits(), 4);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginThrottled);
    assert_eq!(event.user(), None);
    assert_eq!(event.login_hmac(), Some(&fixture.hmac_of("anna")));
    assert_eq!(event.detail(), Some(&json!({"scheme": "basic"})));
    assert!(fixture.store.state().sessions.is_empty());
}

#[tokio::test]
async fn an_active_export_client_with_the_right_password_gets_a_principal() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let principal = fixture.check("anna", PASSWORD).await.unwrap();
    assert_eq!(principal.user.user(), user_id());
    assert_eq!(principal.user.role(), Role::ExportClient);
    assert_eq!(principal.user.login().as_str(), "anna");
    assert_eq!(fixture.hasher.calls().len(), 1);
    assert!(fixture.sink.events().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.store.state().sessions.is_empty());
}

#[tokio::test]
async fn a_success_marks_the_attempt_as_succeeded() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    fixture.check("anna", PASSWORD).await.unwrap();
    let state = fixture.store.state();
    assert_eq!(state.attempts.len(), 1);
    assert!(state.attempts.values().all(|row| row.succeeded));
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn a_success_still_succeeds_when_unlock_login_removed_the_attempt() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let store = fixture.store.clone();
    fixture.hasher.change_during_check(move || {
        store.update_state(|state, _now| state.attempts.clear());
    });
    let principal = fixture.check("anna", PASSWORD).await.unwrap();
    assert_eq!(principal.user.user(), user_id());
    assert!(fixture.store.state().attempts.is_empty());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn a_reset_during_the_hash_check_gives_e_idn_003() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let store = fixture.store.clone();
    fixture
        .hasher
        .change_during_check(move || reset_password(&store));
    let error = fixture.check("anna", PASSWORD).await.unwrap_err();
    assert_invalid_credentials(&error);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "wrong_password", "scheme": "basic"}))
    );
    assert_no_trace_of_success(&fixture);
}

#[tokio::test]
async fn a_deactivation_during_the_hash_check_gives_e_idn_003() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let store = fixture.store.clone();
    fixture
        .hasher
        .change_during_check(move || deactivate_user(&store));
    let error = fixture.check("anna", PASSWORD).await.unwrap_err();
    assert_invalid_credentials(&error);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginRejectedInactive);
    assert_eq!(event.user(), Some(user_id()));
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "inactive", "scheme": "basic"}))
    );
    assert_no_trace_of_success(&fixture);
}

#[tokio::test]
async fn a_role_change_during_the_hash_check_gives_e_idn_003() {
    let fixture = Fixture::with_anna(Role::ExportClient);
    let store = fixture.store.clone();
    fixture.hasher.change_during_check(move || {
        store.update_state(|state, _now| {
            state.users.get_mut(&user_id()).unwrap().role = Role::Operator;
        });
    });
    let error = fixture.check("anna", PASSWORD).await.unwrap_err();
    assert_invalid_credentials(&error);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(
        events.first().unwrap().detail(),
        Some(&json!({"reason": "person_account", "scheme": "basic"}))
    );
    assert_no_trace_of_success(&fixture);
}

/// Assert that a refused T2 left a failed attempt, no session, no audit entry, and free permits.
fn assert_no_trace_of_success(fixture: &Fixture) {
    let state = fixture.store.state();
    assert_eq!(state.attempts.len(), 1);
    assert!(state.attempts.values().all(|row| !row.succeeded));
    assert!(state.sessions.is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[test]
fn debug_of_the_credentials_redacts_the_password() {
    let text = format!("{:?}", credentials("anna", PASSWORD));
    assert!(!text.contains(PASSWORD), "{text}");
    assert!(text.contains("BasicCredentials"), "{text}");
}

/// Build a context over a memory store with the account `anna` and a hasher that blocks.
fn blocking_context(hasher: &Arc<BlockingHasher>) -> IdentityContext {
    blocking_parts(hasher).0
}

/// Build the context of [`blocking_context`] and the sink that records its security events.
fn blocking_parts(hasher: &Arc<BlockingHasher>) -> (IdentityContext, MemorySink) {
    let clock = MemoryClock::new(OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap());
    let store = MemoryStore::with_clock(clock.clone());
    store.seed_user(&NewUser {
        id: user_id(),
        login: UserLogin::parse("anna").unwrap(),
        password_hash: fake_hash(PASSWORD),
        role: Role::ExportClient,
        must_change_password: false,
    });
    let settings = IdentitySettings::default();
    let app = AppContext::new(
        Arc::new(store.clone()),
        Arc::new(store.sink()),
        Arc::new(clock),
        AppSettings::default(),
    );
    let gate = HashGate::new(4, settings.argon2_acquire_timeout, hasher.clone());
    let sink = store.sink();
    let context = IdentityContext::new(
        app,
        Arc::new(store.clone()),
        Arc::new(store),
        gate,
        KeyRing::parse(LOGIN_KEY).unwrap(),
        settings,
    );
    (context, sink)
}

/// Wait until the sink holds `expected` events, or until ten real seconds pass.
async fn wait_for_events(sink: &MemorySink, expected: usize) -> bool {
    let deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(10))
        .unwrap();
    while sink.events().len() != expected {
        if std::time::Instant::now() > deadline {
            return false;
        }
        tokio::task::yield_now().await;
        std::thread::yield_now();
    }
    true
}

/// Wait until the Argon2 gate has `expected` free permits, or until ten real seconds pass.
async fn wait_for_hash_permits(context: &IdentityContext, expected: usize) -> bool {
    let deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(10))
        .unwrap();
    while context.gate().available_permits() != expected {
        if std::time::Instant::now() > deadline {
            return false;
        }
        tokio::task::yield_now().await;
        std::thread::yield_now();
    }
    true
}

/// Open the hasher when a test ends, so that a failed assertion never leaves a blocked thread.
struct ReleaseOnDrop(Arc<BlockingHasher>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

#[tokio::test(start_paused = true)]
async fn a_dropped_check_keeps_its_basic_permit_until_the_hash_ends() {
    let hasher = Arc::new(BlockingHasher::new());
    let _release = ReleaseOnDrop(Arc::clone(&hasher));
    let context = blocking_context(&hasher);
    let task = {
        let context = context.clone();
        tokio::spawn(async move {
            authenticate_basic(&context, credentials("anna", PASSWORD), &meta())
                .await
                .map(|_principal| ())
        })
    };
    assert!(hasher.wait_started(1).await, "the hash did not start");

    // The client disconnects: the caller drops the future while the hash still runs.
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(hasher.running(), 1);

    // A running blocking task stops the auto-advance of paused time, so the test advances it.
    let waiting = {
        let context = context.clone();
        tokio::spawn(async move { context.acquire_basic().await.err() })
    };
    tokio::task::yield_now().await;
    let past_limit = context
        .settings()
        .argon2_acquire_timeout
        .checked_add(std::time::Duration::from_millis(1))
        .unwrap();
    tokio::time::advance(past_limit).await;
    let second = waiting.await.unwrap();
    let error = second.expect("a second Basic check got a permit while the first hash runs");
    assert_eq!(error.code(), ErrorCode::WEB_009);

    hasher.release();
    assert!(
        wait_for_hash_permits(&context, 4).await,
        "permits stayed held"
    );
    assert!(context.acquire_basic().await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn basic_check_writes_login_failed_after_the_caller_drops_the_future() {
    let hasher = Arc::new(BlockingHasher::new());
    let _release = ReleaseOnDrop(Arc::clone(&hasher));
    let (context, sink) = blocking_parts(&hasher);
    let task = {
        let context = context.clone();
        tokio::spawn(async move {
            authenticate_basic(&context, credentials("nobody", PASSWORD), &meta())
                .await
                .map(|_principal| ())
        })
    };
    assert!(hasher.wait_started(1).await, "the hash did not start");
    assert!(
        sink.events().is_empty(),
        "the event exists before the hash ends"
    );

    // The client disconnects after T1 committed: the caller drops the future during the hash.
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    hasher.release();

    assert!(wait_for_events(&sink, 1).await, "no event after the drop");
    let events = sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    let event = events.into_iter().next().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::LoginFailed);
    assert_eq!(event.request_id(), request_id());
    assert_eq!(
        event.detail(),
        Some(&json!({"reason": "unknown_login", "scheme": "basic"}))
    );
    assert!(
        wait_for_hash_permits(&context, 4).await,
        "permits stayed held"
    );
}
