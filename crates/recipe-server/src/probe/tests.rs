//! Tests of the probe against a local listener of the standard library (design D14 row T18).

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::{ProbeFault, ProbePath, internal_port_from, probe};

/// A budget that is long enough for a local answer on a loaded machine.
const AMPLE: Duration = Duration::from_secs(5);

/// The longest wait of the fake server for the request of the probe.
const REQUEST_WAIT: Duration = Duration::from_secs(5);

/// What the fake server does after it has read the request.
enum Script {
    /// Write the bytes at once.
    Reply(Vec<u8>),
    /// Wait, then write the bytes.
    ReplyAfter(Duration, Vec<u8>),
    /// Write one byte at a time with a pause after each byte.
    Trickle(Vec<u8>, Duration),
    /// Write nothing and keep the connection open for the time.
    Silence(Duration),
    /// Close the connection without an answer.
    Close,
}

/// The longest wait of the fake server for the connection of the probe.
const ACCEPT_WAIT: Duration = Duration::from_secs(5);

/// The pause between two polls of the non-blocking `accept`.
const ACCEPT_POLL: Duration = Duration::from_millis(5);

/// Start a server that handles one connection. The handle returns the request text.
///
/// The thread fails an assertion when no connection comes in `ACCEPT_WAIT`, so a join cannot hang.
fn serve_once(script: Script) -> (SocketAddr, JoinHandle<String>) {
    serve_once_within(script, ACCEPT_WAIT)
}

fn serve_once_within(script: Script, accept_wait: Duration) -> (SocketAddr, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut stream = accept_within(&listener, accept_wait);
        let request = read_request(&mut stream);
        // A client that gave up early closes the socket, so write errors are not faults here.
        let _ignored = act(&mut stream, script);
        request
    });
    (address, handle)
}

/// Poll a non-blocking listener until a client connects. Panic when the wait ends.
fn accept_within(listener: &TcpListener, accept_wait: Duration) -> TcpStream {
    let deadline = Instant::now().checked_add(accept_wait).unwrap();
    loop {
        match listener.accept() {
            Ok((stream, _peer)) => {
                // Some systems pass the non-blocking mode of the listener on to the stream.
                stream.set_nonblocking(false).unwrap();
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
    stream.set_read_timeout(Some(REQUEST_WAIT)).unwrap();
    let mut text = Vec::new();
    let mut chunk = [0_u8; 256];
    while !text.ends_with(b"\r\n\r\n") {
        let count = stream.read(&mut chunk).unwrap();
        if count == 0 {
            break;
        }
        text.extend_from_slice(&chunk[..count]);
    }
    String::from_utf8_lossy(&text).into_owned()
}

fn act(stream: &mut TcpStream, script: Script) -> std::io::Result<()> {
    match script {
        Script::Reply(bytes) => stream.write_all(&bytes),
        Script::ReplyAfter(pause, bytes) => {
            std::thread::sleep(pause);
            stream.write_all(&bytes)
        }
        Script::Trickle(bytes, pause) => {
            for byte in bytes {
                stream.write_all(&[byte])?;
                stream.flush()?;
                std::thread::sleep(pause);
            }
            Ok(())
        }
        Script::Silence(pause) => {
            std::thread::sleep(pause);
            Ok(())
        }
        Script::Close => Ok(()),
    }
}

fn reply(text: &str) -> Script {
    Script::Reply(text.as_bytes().to_vec())
}

fn readiness() -> ProbePath {
    ProbePath::readiness()
}

#[test]
fn a_200_answer_is_success() {
    let (address, server) = serve_once(reply(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 2\r\n\r\nok",
    ));

    let result = probe(address, &readiness(), AMPLE);

    assert_eq!(result, Ok(()));
    server.join().unwrap();
}

#[test]
fn a_status_line_without_a_reason_phrase_is_success() {
    let (address, server) = serve_once(reply("HTTP/1.1 200\r\n\r\n"));

    let result = probe(address, &readiness(), AMPLE);

    assert_eq!(result, Ok(()));
    server.join().unwrap();
}

#[test]
fn a_503_answer_is_a_failure_with_the_status() {
    let (address, server) = serve_once(reply(
        "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 9\r\n\r\nnot ready",
    ));

    let result = probe(address, &readiness(), AMPLE);

    assert_eq!(result, Err(ProbeFault::Status(503)));
    server.join().unwrap();
}

#[test]
fn every_status_line_other_than_http_1_1_200_is_a_failure() {
    let cases = [
        ("HTTP/1.0 200 OK\r\n\r\n", ProbeFault::NotHttp),
        ("HTTP/1.1 2000 OK\r\n\r\n", ProbeFault::NotHttp),
        ("HTTP/1.1 20 OK\r\n\r\n", ProbeFault::NotHttp),
        ("HTTP/1.1 abc OK\r\n\r\n", ProbeFault::NotHttp),
        ("\u{0}\u{1}garbage garbage\r\n", ProbeFault::NotHttp),
        ("HTTP/1.1 204 No Content\r\n\r\n", ProbeFault::Status(204)),
        ("HTTP/1.1 500 Oops\r\n\r\n", ProbeFault::Status(500)),
        // The probe reads the first line only. A status line on a later line does not count.
        ("x-early: 1\r\nHTTP/1.1 200 OK\r\n\r\n", ProbeFault::NotHttp),
        ("\r\nHTTP/1.1 200 OK\r\n\r\n", ProbeFault::NotHttp),
    ];
    for (line, expected) in cases {
        let (address, server) = serve_once(reply(line));

        let result = probe(address, &readiness(), AMPLE);

        assert_eq!(result, Err(expected), "status line: {line:?}");
        server.join().unwrap();
    }
}

#[test]
fn a_status_line_without_an_end_inside_the_limit_is_a_failure() {
    let (address, server) = serve_once(Script::Reply(vec![b'x'; 300]));

    let result = probe(address, &readiness(), AMPLE);

    assert_eq!(result, Err(ProbeFault::LineTooLong));
    server.join().unwrap();
}

#[test]
fn a_closed_connection_without_an_answer_is_a_failure() {
    let (address, server) = serve_once(Script::Close);

    let result = probe(address, &readiness(), AMPLE);

    assert_eq!(result, Err(ProbeFault::Closed));
    server.join().unwrap();
}

#[test]
fn a_connection_that_stays_silent_ends_at_the_budget() {
    let budget = Duration::from_millis(150);
    let (address, server) = serve_once(Script::Silence(Duration::from_millis(700)));

    let started = Instant::now();
    let result = probe(address, &readiness(), budget);
    let elapsed = started.elapsed();

    assert_eq!(result, Err(ProbeFault::Timeout));
    assert!(elapsed >= budget, "ended early after {elapsed:?}");
    assert!(elapsed < Duration::from_millis(600), "took {elapsed:?}");
    server.join().unwrap();
}

#[test]
fn a_spent_budget_is_a_timeout_before_any_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();

    let result = probe(address, &readiness(), Duration::ZERO);

    assert_eq!(result, Err(ProbeFault::Timeout));
    let accepted = listener
        .accept()
        .map(|_connection| ())
        .map_err(|e| e.kind());
    assert_eq!(accepted, Err(ErrorKind::WouldBlock), "the probe connected");
}

/// The number of probes of the repeat test. One early wake of a socket timeout is rare, so the
/// test repeats the probe.
const REPEATS: usize = 30;

#[test]
fn a_silent_connection_never_ends_before_the_budget_in_repeated_probes() {
    // A socket read timeout can end a little before the requested time (seen on Windows). The
    // probe must go on to read until the deadline is spent (CI-W4-1).
    let budget = Duration::from_millis(40);
    for round in 0..REPEATS {
        let (address, server) = serve_once(Script::Silence(Duration::from_millis(200)));

        let started = Instant::now();
        let result = probe(address, &readiness(), budget);
        let elapsed = started.elapsed();

        assert_eq!(result, Err(ProbeFault::Timeout), "round {round}");
        assert!(
            elapsed >= budget,
            "round {round} ended early after {elapsed:?}"
        );
        server.join().unwrap();
    }
}

#[test]
fn a_slow_answer_inside_the_budget_is_success() {
    let (address, server) = serve_once(Script::ReplyAfter(
        Duration::from_millis(300),
        b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),
    ));

    let result = probe(address, &readiness(), AMPLE);

    assert_eq!(result, Ok(()));
    server.join().unwrap();
}

#[test]
fn a_slow_trickle_ends_at_the_budget_of_the_whole_probe() {
    // 17 bytes at 40 ms are 680 ms. Each single read stays below the budget of 250 ms.
    // Only a budget for the whole probe turns this answer into a failure.
    let budget = Duration::from_millis(250);
    let (address, server) = serve_once(Script::Trickle(
        b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),
        Duration::from_millis(40),
    ));

    let started = Instant::now();
    let result = probe(address, &readiness(), budget);
    let elapsed = started.elapsed();

    assert_eq!(result, Err(ProbeFault::Timeout));
    assert!(elapsed >= budget, "ended early after {elapsed:?}");
    assert!(elapsed < Duration::from_millis(600), "took {elapsed:?}");
    server.join().unwrap();
}

#[test]
fn a_fake_server_without_a_connection_ends_at_its_accept_bound() {
    let accept_wait = Duration::from_millis(100);
    let (_address, server) = serve_once_within(reply("HTTP/1.1 200 OK\r\n\r\n"), accept_wait);

    let started = Instant::now();
    let outcome = server.join();
    let elapsed = started.elapsed();

    assert!(outcome.is_err(), "the server ended without a connection");
    assert!(elapsed < Duration::from_secs(2), "took {elapsed:?}");
}

#[test]
fn a_refused_connection_is_a_connect_failure() {
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };

    let result = probe(address, &readiness(), AMPLE);

    assert!(
        matches!(result, Err(ProbeFault::Connect(_) | ProbeFault::Timeout)),
        "{result:?}"
    );
}

#[test]
fn the_request_is_a_get_with_connection_close() {
    let (address, server) = serve_once(reply("HTTP/1.1 200 OK\r\n\r\n"));

    probe(address, &ProbePath::liveness(), AMPLE).unwrap();

    let request = server.join().unwrap();
    assert!(
        request.starts_with("GET /healthz HTTP/1.1\r\n"),
        "{request}"
    );
    assert!(request.contains("\r\nConnection: close\r\n"), "{request}");
    assert!(request.contains("\r\nHost: 127.0.0.1:"), "{request}");
    assert!(request.ends_with("\r\n\r\n"), "{request}");
}

#[test]
fn the_default_path_is_the_readiness_path() {
    assert_eq!(ProbePath::readiness().as_str(), "/readyz");
    assert_eq!(ProbePath::liveness().as_str(), "/healthz");
}

#[test]
fn a_path_parses_only_when_it_is_safe_in_a_request_line() {
    let good = ["/readyz", "/healthz", "/a/b-c_d.e"];
    let bad = [
        "",
        "readyz",
        "/with space",
        "/line\r\nX-Injected: 1",
        "/tab\there",
        "/nul\u{0}",
        "/ünï",
    ];
    for text in good {
        let path = ProbePath::parse(text).unwrap();
        assert_eq!(path.as_str(), text);
    }
    for text in bad {
        assert!(ProbePath::parse(text).is_err(), "accepted {text:?}");
    }
}

#[test]
fn the_internal_port_comes_from_the_value_or_the_default() {
    assert_eq!(
        internal_port_from(None).unwrap(),
        super::DEFAULT_INTERNAL_PORT
    );
    assert_eq!(internal_port_from(Some("9100")).unwrap(), 9100);
    for bad in ["", "abc", "0", "65536", "-1", " 9000", "9000 "] {
        assert!(internal_port_from(Some(bad)).is_err(), "accepted {bad:?}");
    }
}

#[test]
fn a_fault_text_names_the_cause_and_no_server_text() {
    let faults = [
        ProbeFault::Connect(ErrorKind::ConnectionRefused),
        ProbeFault::Write(ErrorKind::BrokenPipe),
        ProbeFault::Read(ErrorKind::ConnectionReset),
        ProbeFault::Timeout,
        ProbeFault::Closed,
        ProbeFault::LineTooLong,
        ProbeFault::NotHttp,
        ProbeFault::Status(503),
    ];
    for fault in faults {
        let text = fault.to_string();
        assert!(!text.is_empty());
        assert!(!text.contains('\n'), "{text}");
    }
    assert!(ProbeFault::Status(503).to_string().contains("503"));
}
