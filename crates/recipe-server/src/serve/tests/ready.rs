//! The startup gate of `/readyz` (ADR-0061 step 10).
//!
//! Step 8 opens the internal listener. Step 9 reads the state before the public bind of step 10.
//! A request to `/readyz` in that window must get `503`. The test holds step 9, sends the request
//! through a real socket, and sends a second request after step 10.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use super::expect_running;
use crate::serve::host::ListenerKind;
use crate::serve::test_support::stub::{Event, Hold, StubHost};

/// The pause between two looks at the event log.
const WAIT_STEP: Duration = Duration::from_millis(5);
/// The time after which a test thread gives up. A hang then fails the test.
const WAIT_LIMIT: Duration = Duration::from_secs(30);
/// The time that one socket call may take.
const SOCKET_LIMIT: Duration = Duration::from_secs(10);

/// Release the hold of step 9 when the thread ends, also after a panic.
struct ReleaseOnDrop(Hold);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// Wait until `done` is true. Fail the test after `WAIT_LIMIT`.
fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let begin = Instant::now();
    while !done() {
        assert!(begin.elapsed() < WAIT_LIMIT, "{what}");
        thread::sleep(WAIT_STEP);
    }
}

/// Send `GET path` to the port and return the whole answer as text.
pub(super) fn get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    stream.set_read_timeout(Some(SOCKET_LIMIT)).unwrap();
    stream.set_write_timeout(Some(SOCKET_LIMIT)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    text
}

/// The status code of the first line of an answer.
pub(super) fn status(answer: &str) -> u16 {
    answer
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_default()
}

#[test]
fn readyz_answers_503_until_the_public_listener_is_bound() {
    let mut host = StubHost::new();
    host.realtime = true;
    host.hold.arm();
    let (log, ports, hold) = (host.log.clone(), host.ports.clone(), host.hold.clone());
    let held = thread::spawn(move || {
        let _release = ReleaseOnDrop(hold);
        wait_until("step 9 never began", || log.contains(&Event::StateRead));
        let port = ports.of(ListenerKind::Internal).unwrap();
        get(port, "/readyz")
    });
    let server = expect_running(&host);
    let before = held.join().unwrap();
    assert_eq!(status(&before), 503, "{before}");
    assert!(before.ends_with("not ready"), "{before}");

    let port = host.ports.of(ListenerKind::Internal).unwrap();
    let after = server
        .runtime()
        .block_on(async { tokio::task::spawn_blocking(move || get(port, "/readyz")).await })
        .unwrap();
    assert_eq!(status(&after), 200, "{after}");
    assert!(after.ends_with("ok"), "{after}");
    let _report = server.close();
}
