//! In-memory security event sink.

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::SecurityEvent;
use recipe_domain::timestamp::Timestamp;

use super::{Fault, MemoryClock};
use crate::error::StorageError;
use crate::ports::{Clock as _, SecurityEventSink};

/// One recorded event with its time. Its id is its position in the list plus 1.
#[derive(Clone, Debug)]
pub(crate) struct RecordedEvent {
    /// The time of the clock of the sink at `record`.
    pub(crate) at: Timestamp,
    /// The event as the use case wrote it.
    pub(crate) event: SecurityEvent,
}

#[derive(Debug, Default)]
struct SinkState {
    events: Vec<RecordedEvent>,
    failure: Option<Fault>,
}

/// A sink that keeps every recorded event in order, with the time of its clock.
///
/// The type solves one problem: a test must read the events that a use case wrote
/// and must make the write fail. A clone shares the state. The sink of a `MemoryStore`
/// shares the clock of the store, so the read model gives each event the store time.
#[derive(Clone, Debug, Default)]
pub struct MemorySink {
    state: Arc<Mutex<SinkState>>,
    clock: MemoryClock,
}

impl MemorySink {
    /// Build an empty sink that takes the time of each event from `clock`.
    pub(crate) fn with_clock(clock: MemoryClock) -> Self {
        Self {
            state: Arc::default(),
            clock,
        }
    }

    /// Return a copy of the recorded events, in the order of the calls.
    #[must_use]
    pub fn events(&self) -> Vec<SecurityEvent> {
        self.state
            .lock()
            .events
            .iter()
            .map(|recorded| recorded.event.clone())
            .collect()
    }

    /// Return a copy of the recorded events with their times, in the order of the calls.
    pub(crate) fn recorded(&self) -> Vec<RecordedEvent> {
        self.state.lock().events.clone()
    }

    /// Return the number of recorded events.
    pub(crate) fn recorded_count(&self) -> usize {
        self.state.lock().events.len()
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
        let at = self.clock.now();
        let mut state = self.state.lock();
        if let Some(fault) = state.failure {
            return Err(fault.into());
        }
        state.events.push(RecordedEvent { at, event });
        Ok(())
    }
}

#[cfg(test)]
mod tests;
