#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Test of the exit of the watchdog (`reliability.md` section 2.4 rule 5, ADR-0061 (g)).
//!
//! A call of `std::process::exit` ends the test binary, so the test starts its own executable
//! as a child process. The child runs the test `child_entry` with a port in the environment. It
//! starts the real watchdog against a listener that closes every connection, arms it, and waits.
//! The parent expects exit code 70 and exactly one `ERROR` line.

use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;

use recipe_server::logging;
use recipe_server::watchdog::{Watchdog, WatchdogSettings};

mod bounded_run;
use bounded_run::run_bounded;

/// The longest run of the child process. A watchdog that does not exit is killed after it.
const CHILD_LIMIT: Duration = Duration::from_secs(60);

/// The environment variable that turns the test `child_entry` into the child.
const CHILD_PORT: &str = "RECIPE_WATCHDOG_TEST_PORT";

/// How long the child waits for its own exit. The test fails when this time ends.
const CHILD_WAIT: Duration = Duration::from_secs(40);

/// The entry of the child process. In a normal run of the test binary, it does nothing.
#[test]
fn child_entry() {
    let Ok(port) = std::env::var(CHILD_PORT) else {
        return;
    };
    let port: u16 = port.parse().unwrap();
    logging::init_logging().unwrap();
    let settings = WatchdogSettings {
        interval: Duration::from_millis(20),
        failures: 3,
        window: Duration::from_secs(30),
    };
    let watchdog = Watchdog::start(settings, port).unwrap();
    watchdog.arm();
    std::thread::sleep(CHILD_WAIT);
    // The watchdog ends the process before this line. When the child reaches it, the child
    // returns with exit code 0 and the parent test fails on the code.
    watchdog.stop().unwrap();
}

#[test]
fn a_hung_server_ends_the_process_with_exit_code_70_and_one_error_line() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    // The thread ends with the test process. The listener has no other owner.
    let _closer = std::thread::spawn(move || {
        // Accept and close at once, so every probe fails without waiting for the budget.
        while let Ok((stream, _peer)) = listener.accept() {
            drop(stream);
        }
    });
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "child_entry",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env(CHILD_PORT, port.to_string());

    let output = run_bounded(command, CHILD_LIMIT);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(70), "{stdout}");
    let errors: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("\"level\":\"ERROR\""))
        .collect();
    assert_eq!(errors.len(), 1, "{stdout}");
    assert!(errors[0].contains("watchdog"), "{}", errors[0]);
}
