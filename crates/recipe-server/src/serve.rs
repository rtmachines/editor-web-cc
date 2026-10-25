//! The start of the subcommand `serve` (design D4, ADR-0061).
//!
//! The module holds the ten steps of the start behind the trait [`Host`], so a test runs them with
//! stubs. The type [`Server`] is what a finished start returns. It waits for the stop signal and
//! stops the server.

mod host;
mod jobs;
mod opened;
mod plan;
mod process;
mod server;
mod start;
mod webhook_client;
mod wiring;

pub use host::{HealthPool, Host, ListenerKind, MainPool, PoolClose, Signal, WatchdogControl};
pub use plan::{Plan, ServeSettings, ShutdownSettings};
pub use process::{ProcessHost, run};
pub use server::{CloseReport, ListenerHandle, Server};
pub use start::{Outcome, start};
pub use webhook_client::build_webhook;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
