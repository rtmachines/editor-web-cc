//! The context of the identity use cases (design D2).

use std::sync::Arc;

use recipe_domain::identity::{KeyRing, breached_password_count};

use super::{HashGate, IdentitySettings};
use crate::context::AppContext;
use crate::ports::{LoginAttemptStore, SessionStore};

/// The ports and settings that the identity use cases need.
///
/// The type solves one problem: `AppContext::new` must not change, so the use cases of P2.3 keep
/// their signatures, but the identity use cases also need the two pool ports, the Argon2 gate,
/// the key of the login HMAC, and the identity settings. A clone shares the adapters.
#[derive(Clone)]
pub struct IdentityContext {
    /// The unit of work, the event sink, the clock, and the general settings.
    pub(crate) app: AppContext,
    /// T1 of `Login` and of `ChangeOwnPassword`.
    pub(crate) attempts: Arc<dyn LoginAttemptStore>,
    /// The check of the session of a request.
    pub(crate) sessions: Arc<dyn SessionStore>,
    /// The gate in front of every Argon2 call.
    pub(crate) gate: HashGate,
    /// The key ring of the login HMAC in security events.
    pub(crate) login_keys: KeyRing,
    /// The settings of `reliability.md` section 7.11 for the identity use cases.
    pub(crate) settings: IdentitySettings,
}

impl IdentityContext {
    /// Build the context from the adapters, the gate, the key ring, and the settings.
    ///
    /// The call loads the breached-password list, so the first password check of a request
    /// never pays for the load (design D4). `recipe-server` calls it once at startup.
    #[must_use]
    pub fn new(
        app: AppContext,
        attempts: Arc<dyn LoginAttemptStore>,
        sessions: Arc<dyn SessionStore>,
        gate: HashGate,
        login_keys: KeyRing,
        settings: IdentitySettings,
    ) -> Self {
        let _loaded = breached_password_count();
        Self {
            app,
            attempts,
            sessions,
            gate,
            login_keys,
            settings,
        }
    }

    /// Return the context of the general use cases.
    #[must_use]
    pub const fn app(&self) -> &AppContext {
        &self.app
    }

    /// Return the pool port of T1 of `Login`.
    #[must_use]
    pub fn attempts(&self) -> &dyn LoginAttemptStore {
        self.attempts.as_ref()
    }

    /// Return the gate in front of every Argon2 call.
    #[must_use]
    pub const fn gate(&self) -> &HashGate {
        &self.gate
    }

    /// Return the key ring of the login HMAC.
    #[must_use]
    pub const fn login_keys(&self) -> &KeyRing {
        &self.login_keys
    }

    /// Return the settings of the identity use cases.
    #[must_use]
    pub const fn settings(&self) -> &IdentitySettings {
        &self.settings
    }
}
