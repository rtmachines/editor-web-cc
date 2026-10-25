//! The startup `INFO` event (`reliability.md` 3.3, finding CRX-S8-8).
//!
//! A start that passes step 10 writes exactly one event. A refusal and a stop signal before step 10
//! write none.

use super::{expect_interrupted, expect_running, start_default};
use crate::serve::test_support::stub::{Fail, Point, StubHost};
use crate::serve::test_support::{INTERNAL_PORT, LogCapture, PUBLIC_PORT};

const MESSAGE: &str = "the server started";

#[test]
fn a_successful_start_writes_one_info_event_with_the_two_ports() {
    let logs = LogCapture::start();
    let host = StubHost::new();
    let _server = expect_running(&host);
    let lines = logs.lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = lines.first().unwrap();
    assert!(line.contains("\"level\":\"INFO\""), "{line}");
    assert!(
        line.contains(&format!("\"message\":\"{MESSAGE}\"")),
        "{line}"
    );
    assert!(
        line.contains(&format!("\"public_port\":{PUBLIC_PORT}")),
        "{line}"
    );
    assert!(
        line.contains(&format!("\"internal_port\":{INTERNAL_PORT}")),
        "{line}"
    );
}

#[test]
fn a_refused_start_writes_no_startup_event() {
    for step in [
        Fail::Webhook,
        Fail::MainPool,
        Fail::BindInternal,
        Fail::BindPublic,
    ] {
        let logs = LogCapture::start();
        let mut host = StubHost::new();
        host.fail = Some(step);
        assert!(start_default(&host).is_err(), "{step:?} must refuse");
        let lines = logs.lines();
        assert!(lines.is_empty(), "{lines:?}");
    }
}

#[test]
fn a_signal_before_step_10_writes_no_startup_event() {
    for point in [Point::OpenMain, Point::UnitOfWork, Point::BindPublic] {
        let logs = LogCapture::start();
        let host = StubHost::new();
        host.fire.at(point);
        expect_interrupted(&host);
        let lines = logs.lines();
        assert!(lines.is_empty(), "{lines:?}");
    }
}
