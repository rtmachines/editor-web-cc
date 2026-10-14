//! Test support of the identity use cases: a context over a `MemoryStore`.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use recipe_domain::audit::RequestId;
use recipe_domain::identity::{
    CsrfToken, KeyRing, Login, NewPassword, OneTimePassword, Password, PasswordHash, Role,
    SessionId, SessionToken, UserId,
};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    Authenticated, HashFailure, HashGate, IdentityContext, IdentitySettings, PasswordHasher,
    authenticate,
};
use crate::auth::RequestMeta;
use crate::blocking::AppSettings;
use crate::context::AppContext;
use crate::memory::fixtures::RaceStore;
use crate::memory::{JournalEntry, MemoryClock, MemorySink, MemoryStore};
use crate::ports::{Isolation, NewSession, NewUser, UnitOfWork};

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

/// Count the `SERIALIZABLE` transactions that the store opened, a failed run included.
pub(crate) fn serializable_runs(store: &MemoryStore) -> usize {
    store
        .journal()
        .iter()
        .filter(|entry| matches!(entry, JournalEntry::Begin(Isolation::Serializable)))
        .count()
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
        let sink = MemorySink::default();
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
