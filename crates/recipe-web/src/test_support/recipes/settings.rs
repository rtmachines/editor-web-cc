//! A fixture with the settings of the use cases of a test: `Fixture::with_app_settings`.
//!
//! The function solves one problem: the tests of the diff pages need a small `page.diff` and the
//! failing blocking tasks of `Blocking::elapsed` and `Blocking::panicked`. `Fixture::new` always
//! takes `AppSettings::default()`.

use std::sync::Arc;

use recipe_app::blocking::AppSettings;
use recipe_app::context::AppContext;
use recipe_app::identity::{Argon2Hasher, HashGate, IdentityContext, IdentitySettings};
use recipe_app::memory::{MemoryClock, MemoryStore};
use recipe_domain::identity::KeyRing;
use time::OffsetDateTime;

use crate::test_support::{
    CSRF_KEY_NEW, Fixture, LOGIN_KEY, START_SECONDS, TEST_PARAMS, install_global_subscriber,
};
use crate::{WebSettings, WebState};

impl Fixture {
    /// Build a fixture with the default web settings and the settings `app` of the use cases.
    pub(crate) fn with_app_settings(app: AppSettings) -> Self {
        install_global_subscriber();
        let start = OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap();
        let clock = MemoryClock::new(start);
        let store = MemoryStore::with_clock(clock.clone());
        let sink = store.sink();
        let settings = IdentitySettings::default();
        let context = AppContext::new(
            Arc::new(store.clone()),
            Arc::new(sink.clone()),
            Arc::new(clock),
            app,
        );
        let hasher = Arc::new(Argon2Hasher::new(TEST_PARAMS).unwrap());
        let gate = HashGate::new(
            settings.argon2_permits,
            settings.argon2_acquire_timeout,
            hasher,
        );
        let identity = IdentityContext::new(
            context,
            Arc::new(store.clone()),
            Arc::new(store.clone()),
            gate,
            KeyRing::parse(LOGIN_KEY).unwrap(),
            settings,
        );
        let keys = KeyRing::parse(CSRF_KEY_NEW).unwrap();
        let state = WebState::new(identity, WebSettings::default(), keys).unwrap();
        Self { state, store, sink }
    }
}
