#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Test of the subcommand `healthcheck` in the binary (ADR-0059 (a), (c), design D14 row T18).
//!
//! The binary must exit with code 0 for the status 200 and with code 1 for every other result.
//! The test needs no database. A listener of the standard library plays the internal listener.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

mod bounded_run;
use bounded_run::run_bounded;

/// The longest run of the child process. A probe that does not stop is killed after it.
///
/// This limit is a hang guard only. No assertion measures the run of the whole child process,
/// because that time includes the start of the process.
const CHILD_LIMIT: Duration = Duration::from_secs(30);

/// The longest wait of the fake listener for the connection of the child process.
const ACCEPT_WAIT: Duration = Duration::from_secs(10);

/// The longest wait of the fake listener for the request of the child process.
const REQUEST_WAIT: Duration = Duration::from_secs(10);

/// The pause between two polls of the non-blocking `accept`.
const ACCEPT_POLL: Duration = Duration::from_millis(5);

/// The shortest run of a probe that waits for the whole budget.
///
/// This is the value of `watchdog.probe_timeout` in `reliability.md` section 7.11. The test does
/// not read the constant `PROBE_BUDGET`, so a change of that constant does not move the floor.
/// The clock runs in this process from before the spawn of the child to its exit. That span holds
/// the whole probe, also on a slow runner. No ceiling exists, because a slow runner can only
/// make the span longer. `CHILD_LIMIT` is the hang guard.
const PROBE_BUDGET_FLOOR: Duration = Duration::from_secs(3);

const PORT_VARIABLE: &str = "RECIPE_HTTP__INTERNAL_PORT";

/// Poll a non-blocking listener until a client connects.
///
/// The function fails an assertion when the wait ends, so a join of the server thread cannot hang.
fn accept_within(listener: &TcpListener, accept_wait: Duration) -> TcpStream {
    let deadline = Instant::now().checked_add(accept_wait).unwrap();
    loop {
        match listener.accept() {
            Ok((stream, _peer)) => {
                // Some systems pass the non-blocking mode of the listener on to the stream.
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(REQUEST_WAIT)).unwrap();
                return stream;
            }
            Err(error) => {
                assert_eq!(
                    error.kind(),
                    ErrorKind::WouldBlock,
                    "accept failed: {error}"
                );
                assert!(
                    Instant::now() < deadline,
                    "no connection in {accept_wait:?}"
                );
                std::thread::sleep(ACCEPT_POLL);
            }
        }
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 256];
    while !request.ends_with(b"\r\n\r\n") {
        let count = stream.read(&mut chunk).unwrap();
        if count == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..count]);
    }
    String::from_utf8_lossy(&request).into_owned()
}

fn bind_nonblocking() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    (listener, address)
}

/// Answer one request with the given bytes. The handle returns the request text.
fn answer_once(bytes: &'static [u8]) -> (SocketAddr, JoinHandle<String>) {
    answer_once_within(bytes, ACCEPT_WAIT)
}

fn answer_once_within(
    bytes: &'static [u8],
    accept_wait: Duration,
) -> (SocketAddr, JoinHandle<String>) {
    let (listener, address) = bind_nonblocking();
    let handle = std::thread::spawn(move || {
        let mut stream = accept_within(&listener, accept_wait);
        let request = read_request(&mut stream);
        stream.write_all(bytes).unwrap();
        request
    });
    (address, handle)
}

/// Read until the peer closes the connection. A read error counts as a close.
///
/// The read timeout of the stream ends the wait, so the function cannot hang.
fn wait_for_close(stream: &mut TcpStream) {
    let mut chunk = [0_u8; 16];
    while let Ok(count) = stream.read(&mut chunk) {
        if count == 0 {
            return;
        }
    }
}

/// Accept one connection, read the request, send nothing, and hold the connection open.
///
/// The server holds the connection open until the client closes it. The handle returns the
/// request text.
fn silent_once() -> (SocketAddr, JoinHandle<String>) {
    let (listener, address) = bind_nonblocking();
    let handle = std::thread::spawn(move || {
        let mut stream = accept_within(&listener, ACCEPT_WAIT);
        let request = read_request(&mut stream);
        wait_for_close(&mut stream);
        request
    });
    (address, handle)
}

fn healthcheck(port: &str, extra: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_recipe-server"));
    command
        .arg("healthcheck")
        .args(extra)
        .env(PORT_VARIABLE, port)
        .env_remove("RUST_LOG");
    run_bounded(command, CHILD_LIMIT)
}

#[test]
fn a_ready_server_gives_exit_code_0_and_the_request_names_the_readiness_path() {
    let (address, server) = answer_once(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok");

    let output = healthcheck(&address.port().to_string(), &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(stdout.is_empty(), "a success writes no log line: {stdout}");
    let request = server.join().unwrap();
    assert!(request.starts_with("GET /readyz HTTP/1.1\r\n"), "{request}");
}

#[test]
fn the_path_option_selects_the_path() {
    let (address, server) = answer_once(b"HTTP/1.1 200 OK\r\n\r\n");

    let output = healthcheck(&address.port().to_string(), &["--path", "/healthz"]);

    assert_eq!(output.status.code(), Some(0));
    let request = server.join().unwrap();
    assert!(
        request.starts_with("GET /healthz HTTP/1.1\r\n"),
        "{request}"
    );
}

#[test]
fn a_503_answer_gives_exit_code_1_and_one_error_line() {
    let (address, server) = answer_once(b"HTTP/1.1 503 Service Unavailable\r\n\r\nnot ready");

    let output = healthcheck(&address.port().to_string(), &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "{stdout}");
    assert!(lines[0].contains("\"level\":\"ERROR\""), "{stdout}");
    assert!(lines[0].contains("503"), "{stdout}");
    server.join().unwrap();
}

#[test]
fn a_fake_server_without_a_connection_ends_at_its_accept_bound() {
    let (_address, server) =
        answer_once_within(b"HTTP/1.1 200 OK\r\n\r\n", Duration::from_millis(100));

    let outcome = server.join();

    let payload = outcome.expect_err("the server ended without a connection");
    let text = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .expect("the panic text is a string");
    assert!(text.contains("no connection in 100ms"), "{text}");
}

#[test]
fn a_listener_that_stays_silent_gives_exit_code_1_at_the_budget_of_3_s() {
    let (address, server) = silent_once();

    let started = Instant::now();
    let output = healthcheck(&address.port().to_string(), &[]);
    let elapsed = started.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("the health probe failed"), "{stdout}");
    let request = server.join().unwrap();
    assert!(request.starts_with("GET /readyz HTTP/1.1\r\n"), "{request}");
    assert!(
        elapsed >= PROBE_BUDGET_FLOOR,
        "the child gave up early after {elapsed:?}"
    );
}

#[test]
fn a_bad_port_value_gives_exit_code_1_and_names_the_variable() {
    for bad in ["abc", "0", "70000"] {
        let output = healthcheck(bad, &[]);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{bad:?}: {stdout}");
        assert!(stdout.contains(PORT_VARIABLE), "{bad:?}: {stdout}");
    }
}

#[test]
fn a_closed_port_gives_exit_code_1() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };

    let output = healthcheck(&port.to_string(), &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("the health probe failed"), "{stdout}");
}
