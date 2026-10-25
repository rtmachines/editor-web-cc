//! Tests of the start of `serve` (design D14 row T26).
//!
//! The tests run the ten steps of ADR-0061 on a [`StubHost`]. They read the order of the steps from
//! the event log of the stub. They never open a database, a real signal, or a port of the plan.

mod catchup;
mod close;
mod counters;
mod intervals;
mod order;
mod ready;
mod refusals;
mod signal;
mod signal_at_once;
mod started;
mod stop;

use std::time::Duration;

use anyhow::Result;
use recipe_app::operations::AlertCounters;

use super::plan::Plan;
use super::server::Server;
use super::start::{Outcome, start};
use super::test_support::stub::StubHost;
use super::test_support::{self, LogCapture};

/// Start with the default plan of the tests.
fn start_default(host: &StubHost) -> Result<Outcome> {
    start(host, test_support::plan(), AlertCounters::default())
}

/// Turn the result of a start into a running server, or into the text of the reason.
fn running(result: Result<Outcome>) -> Result<Box<Server>, String> {
    match result {
        Ok(Outcome::Running(server)) => Ok(server),
        Ok(Outcome::Interrupted) => Err("the start ended on a signal".to_owned()),
        Err(error) => Err(format!("the start failed: {error:#}")),
    }
}

/// Start with `plan` and the shared `counters`, and expect a running server.
fn running_with_counters(host: &StubHost, plan: Plan, counters: AlertCounters) -> Box<Server> {
    running(start(host, plan, counters)).expect("the start must run")
}

/// Start with `plan` and expect a running server.
fn running_with(host: &StubHost, plan: Plan) -> Box<Server> {
    running_with_counters(host, plan, AlertCounters::default())
}

/// Let `seconds` of virtual time pass on the runtime of the server.
fn advance(server: &Server, seconds: u64) {
    server.runtime().block_on(async {
        tokio::time::sleep(Duration::from_secs(seconds)).await;
    });
}

/// A plan in which the job `alerts` runs once in seven days, so it adds no read to a test.
fn quiet_plan() -> Plan {
    let mut plan = test_support::plan();
    plan.settings.alerts.interval = Duration::from_secs(7 * 24 * 60 * 60);
    plan
}

/// Start and expect a running server.
fn expect_running(host: &StubHost) -> Box<Server> {
    running(start_default(host)).expect("the start must run")
}

/// Start and expect that a signal ended the start.
fn expect_interrupted(host: &StubHost) {
    let outcome = start_default(host);
    assert!(
        matches!(outcome, Ok(Outcome::Interrupted)),
        "the start must end on a signal: {:?}",
        outcome.as_ref().err()
    );
}

/// Start and expect a refusal. Return the message with the whole chain of causes.
///
/// The start itself writes no `ERROR` line. The function `main` writes the one line of the exit.
fn expect_refusal(host: &StubHost) -> String {
    let logs = LogCapture::start();
    let message = start_default(host)
        .err()
        .map(|error| format!("{error:#}"))
        .expect("the start must refuse");
    assert!(
        logs.at_level("ERROR").is_empty(),
        "the start must leave the one ERROR line to main: {:?}",
        logs.lines()
    );
    message
}
