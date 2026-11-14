//! The context of the identity use cases (design D2).

use std::sync::Arc;

use recipe_domain::identity::{KeyRing, breached_password_count};

use super::basic_permit::BasicGate;
use super::detached::DetachedTasks;
use super::{HashGate, HashPermit, IdentitySettings};
use crate::context::AppContext;
use crate::error::AppError;
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
    /// The gate in front of the Basic checks of the export routes (`export.basic_permits`).
    pub(crate) basic_gate: BasicGate,
    /// The detached tasks of the security steps (`identity.detached_tasks`, ADR-0172).
    pub(crate) detached: DetachedTasks,
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
        let basic_gate = BasicGate::new(
            settings.export_basic_permits,
            settings.argon2_acquire_timeout,
        );
        let detached = DetachedTasks::new(settings.detached_tasks, settings.argon2_acquire_timeout);
        Self {
            app,
            attempts,
            sessions,
            gate,
            basic_gate,
            detached,
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

    /// Take the Basic permit, then the Argon2 permit (design D2 step 1).
    ///
    /// At most `export.basic_permits` Basic checks wait for or hold an Argon2 permit at one time,
    /// so the UI logins keep the other permits of `argon2.permits` (ADR-0109 (n)). Each wait is at
    /// most `argon2.acquire_timeout`. The returned permit owns the Basic permit too. Both go back
    /// together when the hash ends, also when the caller drops its future first.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Timeout` with the stage `Argon2Permit` (`E-WEB-009`) when a wait
    /// exceeds the limit. The call then holds no permit.
    pub async fn acquire_basic(&self) -> Result<HashPermit, AppError> {
        let basic = self.basic_gate.acquire().await?;
        let hash = self.gate.acquire().await?;
        Ok(hash.holding(basic))
    }

    /// Return the detached tasks of the security steps.
    ///
    /// A clone shares the slots and the task set. The stop of the server drains them (ADR-0172 (af)).
    #[must_use]
    pub const fn detached(&self) -> &DetachedTasks {
        &self.detached
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
