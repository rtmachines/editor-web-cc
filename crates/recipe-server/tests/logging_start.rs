#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Test of the start path of the binary with a bad `RUST_LOG` (ADR-0061 (c), ADR-0080 (c)).
//!
//! The binary must stop with exit code 1 and write exactly one JSON `ERROR` line to stdout. The
//! line names the variable. Then the std `Termination` writes `Error: ` and the error chain to
//! stderr. The test needs no database, because the logger starts first.

use std::process::Command;
use std::time::Duration;

mod bounded_run;
use bounded_run::run_bounded;

/// The longest run of the child process. A start that does not stop is killed after it.
const CHILD_LIMIT: Duration = Duration::from_secs(40);

#[test]
fn a_bad_rust_log_stops_the_start_with_one_json_error_line_and_exit_code_1() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_recipe-server"));
    command.arg("serve").env("RUST_LOG", "info,sqlx=loud");

    let output = run_bounded(command, CHILD_LIMIT);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(output.status.code(), Some(1), "{stdout}\n{stderr}");
    assert_eq!(lines.len(), 1, "{stdout}");
    let line = lines[0];
    assert!(line.starts_with('{') && line.ends_with('}'), "{line}");
    assert!(line.contains("\"level\":\"ERROR\""), "{line}");
    assert!(line.contains("RUST_LOG"), "{line}");
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(stderr.contains("RUST_LOG"), "{stderr}");
}
