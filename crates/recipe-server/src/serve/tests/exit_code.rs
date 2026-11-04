//! Tests of the exit code that `main` returns after a failed `serve` (ADR-0125 (d)).

use std::process::ExitCode;

use anyhow::anyhow;

use crate::serve::host::ListenerKind;
use crate::serve::process::{AcceptLoopEnded, exit_code};

#[test]
fn an_ended_accept_loop_gives_the_exit_code_70() {
    let error = anyhow::Error::from(AcceptLoopEnded {
        kind: ListenerKind::Public,
        panicked: false,
    });

    assert_eq!(exit_code(&error), Some(ExitCode::from(70)));
}

#[test]
fn an_ended_accept_loop_under_a_context_gives_the_exit_code_70() {
    let error = anyhow::Error::from(AcceptLoopEnded {
        kind: ListenerKind::Internal,
        panicked: true,
    })
    .context("the server stopped");

    assert_eq!(exit_code(&error), Some(ExitCode::from(70)));
}

#[test]
fn every_other_error_gives_no_exit_code() {
    let error = anyhow!("the port is in use").context("the start failed");

    assert_eq!(exit_code(&error), None);
}

/// An error that holds another error as its `source()`, like a wrapper of a library.
#[derive(Debug)]
struct Wrapper(AcceptLoopEnded);

impl std::fmt::Display for Wrapper {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the wrapper failed")
    }
}

impl std::error::Error for Wrapper {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
fn an_ended_accept_loop_as_the_source_of_another_error_gives_the_exit_code_70() {
    let error = anyhow::Error::from(Wrapper(AcceptLoopEnded {
        kind: ListenerKind::Public,
        panicked: true,
    }));

    assert_eq!(exit_code(&error), Some(ExitCode::from(70)));
}
