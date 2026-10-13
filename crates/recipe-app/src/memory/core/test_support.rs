//! Test-only helpers that write markers into the state.

use super::store::MemoryStore;
use super::transaction::MemoryTransaction;

impl MemoryTransaction {
    /// Write a marker into the working copy.
    pub(super) fn mark(&mut self, marker: &str) {
        self.state_mut().markers.insert(marker.to_owned());
    }
}

impl MemoryStore {
    /// Return the committed markers, in order.
    pub(super) fn markers(&self) -> Vec<String> {
        self.state().markers.into_iter().collect()
    }
}
