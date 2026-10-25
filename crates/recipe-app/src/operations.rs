//! Shared state and internal operations of the supervised server.
//!
//! `AlertCounters` and `ReadinessState` are the in-process signals that the job `alerts` reads
//! (`reliability.md` section 3.7).

mod alert_counters;
#[cfg(test)]
mod alert_counters_tests;
mod compare_settings;
#[cfg(test)]
mod compare_settings_tests;
mod readiness_state;
#[cfg(test)]
mod readiness_state_tests;

pub use alert_counters::AlertCounters;
pub use compare_settings::{CompareSettings, ConfiguredSettings, WebhookUrlText};
pub use readiness_state::ReadinessState;
