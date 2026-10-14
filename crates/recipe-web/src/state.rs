//! The state of the router (design D2, D14).

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use recipe_app::identity::IdentityContext;
use recipe_app::ports::SecurityEventSink;
use recipe_domain::identity::KeyRing;

use crate::i18n::{BundleReason, Bundles};
use crate::settings::WebSettings;

/// The flag of the read-only mode (`reliability.md` 7.8, Assumption A24).
///
/// The type solves one problem: the disk check of a background job and the gate of every request
/// must share one flag without a lock. A clone shares the flag. It starts off.
#[derive(Clone, Debug, Default)]
pub struct ReadOnlyMode(Arc<AtomicBool>);

impl ReadOnlyMode {
    /// Return true when the system is in read-only mode.
    #[must_use]
    pub fn is_on(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Turn the read-only mode on or off. The disk check calls it.
    pub fn set(&self, on: bool) {
        self.0.store(on, Ordering::SeqCst);
    }
}

/// The shared state of every handler and layer.
///
/// The type solves one problem: axum clones the state for each request, so the clone must be cheap
/// and the state must be `Send + Sync`. Every field is a handle that a clone shares.
#[derive(Clone)]
pub struct WebState {
    identity: IdentityContext,
    settings: Arc<WebSettings>,
    read_only: ReadOnlyMode,
    bundles: Arc<Bundles>,
    csrf_keys: Arc<KeyRing>,
    security: Arc<dyn SecurityEventSink>,
}

impl WebState {
    /// Build the state. The read-only mode is off.
    ///
    /// `csrf_keys` is the key file `csrf_signing_key`, the newest key first. The sink that records
    /// the event `csrf_rejected` is the sink of the `AppContext` inside `identity`, so the use
    /// cases and the CSRF layer always write to one sink.
    ///
    /// # Errors
    ///
    /// Returns the [`BundleReason`] of the first text bundle that does not load.
    pub fn new(
        identity: IdentityContext,
        settings: WebSettings,
        csrf_keys: KeyRing,
    ) -> Result<Self, BundleReason> {
        let security = identity.app().security();
        Ok(Self {
            identity,
            settings: Arc::new(settings),
            read_only: ReadOnlyMode::default(),
            bundles: Arc::new(Bundles::load()?),
            csrf_keys: Arc::new(csrf_keys),
            security,
        })
    }

    /// Return the keys that sign the pre-session CSRF cookie.
    #[must_use]
    pub fn csrf_keys(&self) -> &KeyRing {
        &self.csrf_keys
    }

    /// Return the sink of security events.
    #[must_use]
    pub fn security(&self) -> &dyn SecurityEventSink {
        self.security.as_ref()
    }

    /// Return the text bundles of both languages.
    #[must_use]
    pub fn bundles(&self) -> &Bundles {
        &self.bundles
    }

    /// Return the context of the identity use cases.
    #[must_use]
    pub const fn identity(&self) -> &IdentityContext {
        &self.identity
    }

    /// Return the web settings.
    #[must_use]
    pub fn settings(&self) -> &WebSettings {
        &self.settings
    }

    /// Return the flag of the read-only mode.
    #[must_use]
    pub const fn read_only(&self) -> &ReadOnlyMode {
        &self.read_only
    }
}

impl fmt::Debug for WebState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebState")
            .field("read_only", &self.read_only)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
