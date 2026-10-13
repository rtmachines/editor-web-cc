//! In-memory security event sink.

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::SecurityEvent;

use super::Fault;
use crate::error::StorageError;
use crate::ports::SecurityEventSink;

#[derive(Debug, Default)]
struct SinkState {
    events: Vec<SecurityEvent>,
    failure: Option<Fault>,
}

/// A sink that keeps every recorded event in order.
///
/// The type solves one problem: a test must read the events that a use case wrote
/// and must make the write fail. A clone shares the state.
#[derive(Clone, Debug, Default)]
pub struct MemorySink {
    state: Arc<Mutex<SinkState>>,
}

impl MemorySink {
    /// Return a copy of the recorded events, in the order of the calls.
    #[must_use]
    pub fn events(&self) -> Vec<SecurityEvent> {
        self.state.lock().events.clone()
    }

    /// Make every later `record` call fail with `fault` and keep nothing.
    pub fn fail_with(&self, fault: Fault) {
        self.state.lock().failure = Some(fault);
    }

    /// Stop the failure that `fail_with` started.
    pub fn clear_failure(&self) {
        self.state.lock().failure = None;
    }
}

#[async_trait]
impl SecurityEventSink for MemorySink {
    async fn record(&self, event: SecurityEvent) -> Result<(), StorageError> {
        let mut state = self.state.lock();
        if let Some(fault) = state.failure {
            return Err(fault.into());
        }
        state.events.push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
