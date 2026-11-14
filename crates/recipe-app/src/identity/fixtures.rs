//! Test support of the identity use cases: a context over a `MemoryStore`.

use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};
use std::task::Poll;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::{RequestId, SecurityEvent};
use recipe_domain::identity::{
    CsrfToken, KeyRing, Login, NewPassword, OneTimePassword, Password, PasswordHash, Role,
    SessionId, SessionToken, UserId,
};
use time::OffsetDateTime;
use tokio::sync::Notify;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use tracing_core::span::Current;
use uuid::Uuid;

use super::{
    Authenticated, HashFailure, HashGate, IdentityContext, IdentitySettings, PasswordHasher,
    authenticate,
};
use crate::auth::RequestMeta;
use crate::blocking::AppSettings;
use crate::context::AppContext;
use crate::error::StorageError;
use crate::memory::fixtures::RaceStore;
use crate::memory::{JournalEntry, MemoryClock, MemorySink, MemoryStore};
use crate::ports::{Isolation, NewSession, NewUser, SecurityEventSink, UnitOfWork, UserRow};

/// A key file with one valid key of 64 lower-case hex characters.
const LOGIN_KEY: &str = "0101010101010101010101010101010101010101010101010101010101010101";
/// The start of the clock of a fixture, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;
/// The permits of the gate of a fixture.
const PERMITS: usize = 4;
/// The start of the text of a hash that [`FakeHasher`] makes.
const FAKE_PREFIX: &str = "$argon2id$fake$";

/// Return the identifier of the user `anna`.
pub(crate) fn anna_id() -> UserId {
    UserId::from(Uuid::from_u128(11))
}

/// Return the identifier of the acting user of [`IdentityFixture::actor`].
pub(crate) fn actor_id() -> UserId {
    UserId::from(Uuid::from_u128(100))
}

/// Assert that `password` is a well-formed one-time password.
///
/// It has 20 characters of the 32-character alphabet, and it passes the password policy, which
/// includes the breached-password list.
pub(crate) fn assert_one_time_password(password: &OneTimePassword) {
    const ALPHABET: &str = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let text = password.as_str();
    assert_eq!(text.chars().count(), 20, "{text}");
    assert!(text.chars().all(|c| ALPHABET.contains(c)), "{text}");
    assert!(NewPassword::parse(text).is_ok(), "{text}");
}

/// Count the transactions that the store opened with `isolation`.
///
/// A `Begin` that fails opens no transaction and leaves no line in the journal.
fn begin_count(store: &MemoryStore, isolation: Isolation) -> usize {
    store
        .journal()
        .iter()
        .filter(|entry| matches!(entry, JournalEntry::Begin(level) if *level == isolation))
        .count()
}

/// Count the `SERIALIZABLE` transactions that the store opened, a retried run included.
///
/// Only `bootstrap_administrator` opens such a transaction.
pub(crate) fn serializable_runs(store: &MemoryStore) -> usize {
    begin_count(store, Isolation::Serializable)
}

/// Count the `READ COMMITTED` transactions that the store opened, a failed run included.
///
/// `SetUserRole` and `DeactivateUser` open one such transaction and never run again. The session
/// store also opens such transactions, so a test compares the count before and after the command.
pub(crate) fn read_committed_runs(store: &MemoryStore) -> usize {
    begin_count(store, Isolation::ReadCommitted)
}

/// Return the lines of the journal after the first `start` lines.
pub(crate) fn journal_since(store: &MemoryStore, start: usize) -> Vec<JournalEntry> {
    store.journal().into_iter().skip(start).collect()
}

/// Return the hash that [`FakeHasher`] makes of `password`.
pub(crate) fn fake_hash(password: &str) -> PasswordHash {
    PasswordHash::parse(&format!("{FAKE_PREFIX}{password}")).unwrap()
}

/// A hasher that is fast and counts its calls.
///
/// The hash of a password is the prefix `$argon2id$fake$` and the password itself. A check
/// passes only when the stored hash is the hash of the typed password. A check without a stored
/// hash (the dummy check) fails. Every call to `hash` or `verify` counts, also a dummy check.
#[derive(Debug, Default)]
pub(crate) struct FakeHasher {
    verifies: AtomicUsize,
    hashes: AtomicUsize,
}

impl FakeHasher {
    /// Return the number of calls to `verify`, also the dummy checks.
    pub(crate) fn verify_calls(&self) -> usize {
        self.verifies.load(Ordering::SeqCst)
    }

    /// Return the number of calls to `hash`.
    pub(crate) fn hash_calls(&self) -> usize {
        self.hashes.load(Ordering::SeqCst)
    }
}

impl PasswordHasher for FakeHasher {
    fn hash(&self, password: &NewPassword) -> Result<PasswordHash, HashFailure> {
        self.hashes.fetch_add(1, Ordering::SeqCst);
        PasswordHash::parse(&format!("{FAKE_PREFIX}{}", password.as_str()))
            .map_err(|_malformed| HashFailure::Compute)
    }

    fn verify(&self, password: &Password, hash: Option<&PasswordHash>) -> bool {
        self.verifies.fetch_add(1, Ordering::SeqCst);
        hash.is_some_and(|stored| stored.as_str() == format!("{FAKE_PREFIX}{}", password.as_str()))
    }
}

/// An identity context over a store, a sink, and a clock that a test controls.
pub(crate) struct IdentityFixture {
    pub(crate) store: MemoryStore,
    pub(crate) sink: MemorySink,
    pub(crate) hasher: Arc<FakeHasher>,
    pub(crate) context: IdentityContext,
}

impl IdentityFixture {
    /// Build a fixture with the default settings and a gate over a [`FakeHasher`].
    pub(crate) fn new() -> Self {
        Self::build(|store| Arc::new(store.clone()))
    }

    /// Build a fixture whose unit of work runs `hook` once, just before the first
    /// `READ COMMITTED` transaction opens.
    pub(crate) fn with_race(hook: impl FnOnce(&MemoryStore) + Send + 'static) -> Self {
        Self::build(|store| Arc::new(RaceStore::new(store.clone(), hook)))
    }

    /// Build a fixture in which `change` edits the row of the acting user [`actor_id`] after the
    /// permission check and before the first `READ COMMITTED` transaction opens.
    ///
    /// This models a second Administrator who commits while the command waits for the guard lock.
    pub(crate) fn with_actor_race(change: fn(&mut UserRow)) -> Self {
        Self::with_race(move |store| {
            store.update_state(|state, _now| change(state.users.get_mut(&actor_id()).unwrap()));
        })
    }

    /// Seed the active Operator `anna` with the hash of `password`.
    pub(crate) fn seed_anna(&self, password: &str, must_change_password: bool) {
        self.store.seed_user(&NewUser {
            id: anna_id(),
            login: Login::parse("anna").unwrap(),
            password_hash: fake_hash(password),
            role: Role::Operator,
            must_change_password,
        });
    }

    /// Seed a session of `anna` with epoch 0 for the token of `[byte; 16]`. Return the token.
    pub(crate) fn seed_anna_session(&self, byte: u8) -> SessionToken {
        let token = SessionToken::from_bytes([byte; 16]);
        self.store.seed_session(&NewSession {
            id: SessionId::of(&token),
            user_id: anna_id(),
            epoch: 0,
            csrf_token: CsrfToken::from_bytes([byte; 32]),
            idle_timeout: self.context.settings.session_idle_timeout,
        });
        token
    }

    /// Seed the active user `actor` with `role` and authenticate a session of that user.
    ///
    /// The user has the identifier of [`actor_id`]. The request meta carries the session.
    pub(crate) async fn actor(&self, role: Role) -> (Authenticated, RequestMeta) {
        self.store.seed_user(&NewUser {
            id: actor_id(),
            login: Login::parse("actor").unwrap(),
            password_hash: fake_hash("actor-password"),
            role,
            must_change_password: false,
        });
        let token = SessionToken::from_bytes([100; 16]);
        self.store.seed_session(&NewSession {
            id: SessionId::of(&token),
            user_id: actor_id(),
            epoch: 0,
            csrf_token: CsrfToken::from_bytes([100; 32]),
            idle_timeout: self.context.settings.session_idle_timeout,
        });
        let auth = self.authenticated(&token).await;
        let meta = RequestMeta::new(
            RequestId::from(Uuid::from_u128(0x55)),
            None,
            None,
            Some(auth.session_ref().clone()),
        );
        (auth, meta)
    }

    /// Authenticate a request with `token`, as `recipe-web` does before a use case.
    pub(crate) async fn authenticated(&self, token: &SessionToken) -> Authenticated {
        authenticate(&self.context, token).await.unwrap().unwrap()
    }

    fn build(unit_of_work: impl FnOnce(&MemoryStore) -> Arc<dyn UnitOfWork>) -> Self {
        let start = OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap();
        let clock = MemoryClock::new(start);
        let store = MemoryStore::with_clock(clock.clone());
        let sink = store.sink();
        let settings = IdentitySettings::default();
        let app = AppContext::new(
            unit_of_work(&store),
            Arc::new(sink.clone()),
            Arc::new(clock),
            AppSettings::default(),
        );
        let hasher = Arc::new(FakeHasher::default());
        let gate = HashGate::new(PERMITS, settings.argon2_acquire_timeout, hasher.clone());
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
}

/// The wait of a test for the drain of the detached tasks.
pub(crate) const DRAIN_WAIT: StdDuration = StdDuration::from_secs(5);

/// Poll `future` once and drop it, as a timeout or a closed connection drops a request.
///
/// The first poll must end at a wait. A future that completes on it makes the drop test nothing,
/// so the function panics then.
pub(crate) async fn poll_once_then_drop<F: Future>(future: F) {
    let mut future = std::pin::pin!(future);
    std::future::poll_fn(|cx| {
        let first_poll = future.as_mut().poll(cx);
        assert!(
            first_poll.is_pending(),
            "poll_once_then_drop: the future completed on its first poll, so the drop tests nothing"
        );
        Poll::Ready(())
    })
    .await;
}

/// One event that the code wrote.
#[derive(Clone, Debug)]
pub(crate) struct Logged {
    pub(crate) level: Level,
    pub(crate) fields: Vec<(String, String)>,
}

impl Logged {
    pub(crate) fn field(&self, name: &str) -> Option<&str> {
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

/// The metadata that `current_span` reports for every span.
///
/// `Span::current()` needs `'static` metadata, and a `new_span` call gives only a borrowed one.
/// The test code reads no metadata of a current span, so one probe span serves for all.
/// `install` builds it while the capture is the default, so it holds metadata and not `None`.
static CURRENT_SPAN_METADATA: LazyLock<Option<&'static Metadata<'static>>> =
    LazyLock::new(|| tracing::error_span!("log_capture").metadata());

/// The fields of the open spans and the stack of the entered span ids.
#[derive(Default)]
struct SpanState {
    next_id: u64,
    fields: HashMap<u64, Vec<(String, String)>>,
    entered: Vec<u64>,
}

/// A subscriber that keeps every event of its thread.
///
/// An event also holds the fields of the spans that are entered when it fires. A field of the
/// event wins over a span field of the same name. A test thus sees the `request_id` of a span.
#[derive(Clone, Default)]
pub(crate) struct LogCapture {
    events: Arc<Mutex<Vec<Logged>>>,
    spans: Arc<Mutex<SpanState>>,
}

impl LogCapture {
    pub(crate) fn install() -> (Self, DefaultGuard) {
        LazyLock::force(&KEEPALIVE);
        let capture = Self::default();
        let guard = tracing::subscriber::set_default(capture.clone());
        LazyLock::force(&CURRENT_SPAN_METADATA);
        (capture, guard)
    }

    pub(crate) fn events(&self) -> Vec<Logged> {
        self.events.lock().clone()
    }

    pub(crate) fn count(&self) -> usize {
        self.events.lock().len()
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

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut copy = FieldCopy::default();
        span.record(&mut copy);
        let mut state = self.spans.lock();
        state.next_id = state.next_id.saturating_add(1);
        let id = state.next_id;
        state.fields.insert(id, copy.fields);
        Id::from_u64(id)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut copy = FieldCopy::default();
        values.record(&mut copy);
        let mut state = self.spans.lock();
        if let Some(fields) = state.fields.get_mut(&span.into_u64()) {
            fields.extend(copy.fields);
        }
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut copy = FieldCopy::default();
        event.record(&mut copy);
        let mut fields = copy.fields;
        let state = self.spans.lock();
        for id in state.entered.iter().rev() {
            for (name, value) in state.fields.get(id).into_iter().flatten().rev() {
                if !fields.iter().any(|(key, _)| key == name) {
                    fields.push((name.clone(), value.clone()));
                }
            }
        }
        drop(state);
        self.events.lock().push(Logged {
            level: *event.metadata().level(),
            fields,
        });
    }

    fn current_span(&self) -> Current {
        let Some(metadata) = *CURRENT_SPAN_METADATA else {
            return Current::none();
        };
        match self.spans.lock().entered.last() {
            Some(id) => Current::new(Id::from_u64(*id), metadata),
            None => Current::none(),
        }
    }

    fn enter(&self, span: &Id) {
        self.spans.lock().entered.push(span.into_u64());
    }

    fn exit(&self, span: &Id) {
        let mut state = self.spans.lock();
        if let Some(position) = state.entered.iter().rposition(|id| *id == span.into_u64()) {
            state.entered.remove(position);
        }
    }
}

/// A sink that stops in `record` until a test releases it, then writes into `inner`.
#[derive(Clone)]
pub(crate) struct HeldSink {
    inner: MemorySink,
    pub(crate) entered: Arc<Notify>,
    pub(crate) release: Arc<Notify>,
}

impl HeldSink {
    pub(crate) fn over(inner: MemorySink) -> Self {
        Self {
            inner,
            entered: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
        }
    }
}

#[async_trait]
impl SecurityEventSink for HeldSink {
    async fn record(&self, event: SecurityEvent) -> Result<(), StorageError> {
        self.entered.notify_one();
        self.release.notified().await;
        self.inner.record(event).await
    }
}
