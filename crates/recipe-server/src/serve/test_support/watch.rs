//! The observer of the stop (design D14 row T26).
//!
//! The stop turns readiness off and closes the sockets of the two listeners. Neither step passes
//! through a port of the host. [`Watch`] reads both from outside, whenever a decorator of the stub
//! runs, and writes one event the first time that it sees the change. A test can then read the order
//! of the stop from the event log.

use std::net::{Ipv4Addr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use recipe_web::HealthState;

use super::stub::{Event, Log, Ports};
use crate::serve::host::ListenerKind;

#[derive(Default)]
struct WatchState {
    health: Mutex<Option<HealthState>>,
    readiness_reported: AtomicBool,
    stopped: Mutex<Vec<ListenerKind>>,
}

/// Reads the state of the stop and writes an event for each change. A clone shares the state.
///
/// The observer does nothing until a test calls [`Watch::enable`]. A test of the start then sees
/// no extra event.
#[derive(Clone)]
pub(in crate::serve) struct Watch {
    log: Log,
    ports: Ports,
    state: Arc<WatchState>,
}

impl Watch {
    pub(in crate::serve) fn new(log: Log, ports: Ports) -> Self {
        Self {
            log,
            ports,
            state: Arc::default(),
        }
    }

    /// Start to watch the stop of the server that owns `health`.
    pub(in crate::serve) fn enable(&self, health: HealthState) {
        *self.state.health.lock().unwrap() = Some(health);
    }

    /// Write an event for each change since the last call: readiness off, a listener stopped.
    pub(in crate::serve) fn observe(&self) {
        let Some(health) = self.state.health.lock().unwrap().clone() else {
            return;
        };
        // The test pins the `Debug` text `draining: true` of `HealthState` on purpose. A change of
        // the `Debug` output must update this check and the test that reads it.
        let draining = format!("{health:?}").contains("draining: true");
        if draining && !self.state.readiness_reported.swap(true, Ordering::SeqCst) {
            self.log.push(Event::ReadinessOff);
        }
        for kind in [ListenerKind::Public, ListenerKind::Internal] {
            self.observe_listener(kind);
        }
    }

    fn observe_listener(&self, kind: ListenerKind) {
        let Some(port) = self.ports.of(kind) else {
            return;
        };
        let mut stopped = self.state.stopped.lock().unwrap();
        if !stopped.contains(&kind) && port_is_free(port) {
            stopped.push(kind);
            self.log.push(Event::ListenerStopped(kind));
        }
    }
}

/// Whether nothing listens on `port`. A bind on a listening port fails, so success means a stop.
fn port_is_free(port: u16) -> bool {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok()
}
