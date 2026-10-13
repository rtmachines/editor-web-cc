//! The application context over a fixture.

use std::sync::Arc;

use super::race::RaceStore;
use super::seed::Fixture;
use crate::blocking::AppSettings;
use crate::context::AppContext;
use crate::memory::MemoryStore;

impl Fixture {
    /// Return a context over the store, the sink, and the clock of the fixture.
    ///
    /// The settings are `AppSettings::default()`.
    pub(crate) fn context(&self) -> AppContext {
        self.context_with(AppSettings::default())
    }

    /// Return a context over the fixture with the given settings.
    pub(crate) fn context_with(&self, settings: AppSettings) -> AppContext {
        AppContext::new(
            Arc::new(self.store.clone()),
            Arc::new(self.sink.clone()),
            Arc::new(self.clock.clone()),
            settings,
        )
    }

    /// Return a context whose unit of work runs `hook` once before the first command transaction.
    ///
    /// The command has then read its input (step 2 of design D10), and the hook changes the
    /// store before the locks (step 4). The hook gets the store of the fixture.
    pub(crate) fn context_racing(
        &self,
        hook: impl FnOnce(&MemoryStore) + Send + 'static,
    ) -> AppContext {
        let base = self.context();
        AppContext::new(
            Arc::new(RaceStore::new(self.store.clone(), hook)),
            base.security,
            base.clock,
            base.settings,
        )
    }
}
