//! Tests of the accept loop with a raw TCP client (design D14 row T19).
//!
//! The client uses the standard library with socket timeouts, so a fault cannot hang a test. The
//! server runs on a multi-thread runtime, so a blocking client call does not stall it. The tests
//! set short timeouts and never use the production values of `reliability.md` section 7.11.

use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::ConnectInfo;
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::catch_panic::CatchPanicLayer;

use super::{AcceptLoop, Acceptor, DrainOutcome, ListenerSettings, OpenConnections};

/// The longest wait of any client call or any poll of the tests.
const IO_LIMIT: Duration = Duration::from_secs(5);

/// The pause between two polls of a condition.
const POLL: Duration = Duration::from_millis(10);

/// The header read timeout of the tests.
const HEADER_TIMEOUT: Duration = Duration::from_millis(150);

/// The pause after an accept fault in the tests.
const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

/// The number of connections that the leak test opens and closes.
const CLOSED_CONNECTIONS: usize = 100;

/// The time that a slow handler needs.
const SLOW: Duration = Duration::from_millis(300);

fn settings() -> ListenerSettings {
    ListenerSettings {
        header_read_timeout: HEADER_TIMEOUT,
        drain_deadline: Duration::from_secs(2),
        accept_error_pause: ACCEPT_PAUSE,
    }
}

async fn peer(ConnectInfo(address): ConnectInfo<SocketAddr>) -> String {
    address.to_string()
}

async fn boom() -> &'static str {
    // A failed parse stands for any bug of a handler that panics.
    let number: u8 = "not a number".parse().unwrap();
    if number == 0 { "zero" } else { "other" }
}

/// A router that stands for either listener.
fn app() -> Router {
    Router::new()
        .route("/ok", get(|| async { "ok" }))
        .route("/peer", get(peer))
        .route("/boom", get(boom))
}

/// A router whose handler reports its start through the semaphore and then waits for `delay`.
fn gated_app(entered: &Arc<Semaphore>, delay: Duration) -> Router {
    let entered = Arc::clone(entered);
    Router::new().route(
        "/gated",
        get(move || {
            let entered = Arc::clone(&entered);
            async move {
                entered.add_permits(1);
                tokio::time::sleep(delay).await;
                "done"
            }
        }),
    )
}

/// The signals between a test and a handler that blocks its thread.
///
/// The handler ends only after `release` is set. The test sets it after the drain returned, so no
/// host pause can end the handler before the cancel. The poll of the handler is a pause of one
/// millisecond and not a wall-clock limit.
#[derive(Clone)]
struct HandlerGate {
    entered: Arc<Semaphore>,
    release: Arc<AtomicBool>,
    ended: Arc<AtomicBool>,
}

impl HandlerGate {
    fn new() -> Self {
        Self {
            entered: Arc::new(Semaphore::new(0)),
            release: Arc::new(AtomicBool::new(false)),
            ended: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Release the handler when the guard drops, also when an assertion fails.
    fn release_on_drop(&self) -> ReleaseHandler {
        ReleaseHandler(Arc::clone(&self.release))
    }

    fn handler_ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }
}

/// Sets the release signal of a `HandlerGate` when it drops.
struct ReleaseHandler(Arc<AtomicBool>);

impl Drop for ReleaseHandler {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// A router whose handler reports its start and then blocks its thread without an `.await`.
///
/// An abort cannot reach such a handler before the gate releases it, so the router stands for a
/// task that does not yield (ADR-0087 (d)).
fn blocking_app(gate: &HandlerGate) -> Router {
    let gate = gate.clone();
    Router::new().route(
        "/blocked",
        get(move || {
            let gate = gate.clone();
            async move {
                gate.entered.add_permits(1);
                while !gate.release.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(1));
                }
                gate.ended.store(true, Ordering::SeqCst);
                "done"
            }
        }),
    )
}

async fn wait_entered(entered: &Semaphore, count: u32) {
    let permits = tokio::time::timeout(IO_LIMIT, entered.acquire_many(count))
        .await
        .unwrap()
        .unwrap();
    permits.forget();
}

/// A running accept loop and the handles that a test needs.
struct Running {
    address: SocketAddr,
    stop: CancellationToken,
    open: OpenConnections,
    task: JoinHandle<DrainOutcome>,
}

impl Running {
    /// Stop the loop and wait for its outcome, at most `IO_LIMIT`.
    async fn finish(self) -> DrainOutcome {
        self.stop.cancel();
        tokio::time::timeout(IO_LIMIT, self.task)
            .await
            .unwrap()
            .unwrap()
    }
}

fn spawn_loop<A: Acceptor>(
    acceptor: A,
    address: SocketAddr,
    router: Router,
    settings: ListenerSettings,
) -> Running {
    let stop = CancellationToken::new();
    let accept_loop = AcceptLoop::new(router, settings, stop.clone());
    let open = accept_loop.open_connections();
    let task = tokio::spawn(accept_loop.run_with(acceptor));
    Running {
        address,
        stop,
        open,
        task,
    }
}

async fn start(router: Router, settings: ListenerSettings) -> Running {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    spawn_loop(listener, address, router, settings)
}

/// An acceptor that fails a number of times before it accepts a real connection.
struct Faulty {
    listener: TcpListener,
    faults_left: AtomicUsize,
    kind: ErrorKind,
}

impl Acceptor for Faulty {
    async fn accept(&self) -> io::Result<(tokio::net::TcpStream, SocketAddr)> {
        let fails = self
            .faults_left
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                left.checked_sub(1)
            })
            .is_ok();
        if fails {
            return Err(io::Error::from(self.kind));
        }
        self.listener.accept().await
    }
}

async fn start_faulty(faults: usize, kind: ErrorKind, settings: ListenerSettings) -> Running {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let acceptor = Faulty {
        listener,
        faults_left: AtomicUsize::new(faults),
        kind,
    };
    spawn_loop(acceptor, address, app(), settings)
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

fn connect(address: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect_timeout(&address, IO_LIMIT).unwrap();
    stream.set_read_timeout(Some(IO_LIMIT)).unwrap();
    stream.set_write_timeout(Some(IO_LIMIT)).unwrap();
    stream
}

fn send_get(stream: &mut TcpStream, path: &str, close: bool) {
    let connection = if close { "Connection: close\r\n" } else { "" };
    let request = format!("GET {path} HTTP/1.1\r\nHost: test\r\n{connection}\r\n");
    stream.write_all(request.as_bytes()).unwrap();
}

/// A parsed answer of the server.
struct Reply {
    status: u16,
    body: String,
}

/// Read one answer with a `Content-Length` body. `None` means the server closed the connection.
fn read_reply(stream: &mut TcpStream) -> Option<Reply> {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => head.push(byte[0]),
            Err(error) => {
                assert!(!is_timeout(&error), "no answer within {IO_LIMIT:?}");
                return None;
            }
        }
    }
    let text = String::from_utf8(head).unwrap();
    let status = text.split(' ').nth(1).unwrap().parse().unwrap();
    let length: usize = text
        .lines()
        .find_map(|line| {
            let lower = line.to_ascii_lowercase();
            let value = lower.strip_prefix("content-length:")?;
            Some(value.trim().parse().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0_u8; length];
    stream.read_exact(&mut body).unwrap();
    Some(Reply {
        status,
        body: String::from_utf8(body).unwrap(),
    })
}

/// Read until the server closes the connection. Return the time that the wait took.
fn wait_for_close(stream: &mut TcpStream) -> Duration {
    let started = Instant::now();
    let mut sink = [0_u8; 256];
    loop {
        match stream.read(&mut sink) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                assert!(!is_timeout(&error), "no close within {IO_LIMIT:?}");
                break;
            }
        }
    }
    started.elapsed()
}

async fn wait_for_open(open: &OpenConnections, expected: usize) {
    let deadline = Instant::now().checked_add(IO_LIMIT).unwrap();
    while open.count() != expected {
        assert!(
            Instant::now() < deadline,
            "the loop holds {} connections, not {expected}",
            open.count()
        );
        tokio::time::sleep(POLL).await;
    }
}

/// Poll until no client can connect. A listener that accepts keeps the handshake going.
async fn wait_until_refused(address: SocketAddr) {
    let deadline = Instant::now().checked_add(IO_LIMIT).unwrap();
    while TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
        assert!(Instant::now() < deadline, "the listener still accepts");
        tokio::time::sleep(POLL).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handler_sees_the_peer_address_of_its_connection() {
    let server = start(app(), settings()).await;
    let mut client = connect(server.address);
    send_get(&mut client, "/peer", true);
    let reply = read_reply(&mut client).unwrap();
    assert_eq!(reply.status, 200);
    assert_eq!(reply.body, client.local_addr().unwrap().to_string());
    assert!(server.finish().await.is_complete());
}

/// Design T19: the loop puts `ConnectInfo` into EACH request, not once for the connection.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handler_sees_the_peer_address_in_each_request_of_one_connection() {
    let server = start(app(), settings()).await;
    let mut client = connect(server.address);
    let expected = client.local_addr().unwrap().to_string();
    send_get(&mut client, "/peer", false);
    let first = read_reply(&mut client).unwrap();
    send_get(&mut client, "/peer", false);
    let second = read_reply(&mut client).unwrap();
    send_get(&mut client, "/peer", true);
    let third = read_reply(&mut client).unwrap();
    for reply in [first, second, third] {
        assert_eq!(
            (reply.status, reply.body.as_str()),
            (200, expected.as_str())
        );
    }
    assert!(server.finish().await.is_complete());
}

async fn assert_header_stall_closes(router: Router) {
    let server = start(router, settings()).await;
    let mut client = connect(server.address);
    client.write_all(b"GET /ok HTTP/1.1\r\nHost: te").unwrap();
    let waited = wait_for_close(&mut client);
    assert!(
        waited >= Duration::from_millis(100),
        "closed after {waited:?}"
    );
    wait_for_open(&server.open, 0).await;
    assert!(server.finish().await.is_complete());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_listener_closes_a_client_that_sends_no_complete_header() {
    assert_header_stall_closes(app()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn internal_listener_closes_a_client_that_sends_no_complete_header() {
    let internal = Router::new().route("/healthz", get(|| async { "ok" }));
    assert_header_stall_closes(internal).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_keep_alive_connection_closes_after_the_header_read_timeout() {
    let server = start(app(), settings()).await;
    let mut client = connect(server.address);
    send_get(&mut client, "/ok", false);
    assert_eq!(read_reply(&mut client).unwrap().status, 200);
    let waited = wait_for_close(&mut client);
    assert!(
        waited >= Duration::from_millis(100),
        "closed after {waited:?}"
    );
    assert!(server.finish().await.is_complete());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drain_completes_when_the_request_in_flight_finishes() {
    let entered = Arc::new(Semaphore::new(0));
    let server = start(gated_app(&entered, SLOW), settings()).await;
    let mut client = connect(server.address);
    send_get(&mut client, "/gated", false);
    wait_entered(&entered, 1).await;
    server.stop.cancel();
    let reply = read_reply(&mut client).unwrap();
    assert_eq!((reply.status, reply.body.as_str()), (200, "done"));
    let outcome = tokio::time::timeout(IO_LIMIT, server.task)
        .await
        .unwrap()
        .unwrap();
    assert!(outcome.is_complete());
    assert_eq!(outcome.cut(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drain_closes_an_idle_keep_alive_connection_at_once() {
    let long_header = ListenerSettings {
        header_read_timeout: Duration::from_secs(30),
        ..settings()
    };
    let server = start(app(), long_header).await;
    let mut client = connect(server.address);
    send_get(&mut client, "/ok", false);
    assert_eq!(read_reply(&mut client).unwrap().status, 200);
    server.stop.cancel();
    assert!(wait_for_close(&mut client) < Duration::from_secs(1));
    let outcome = tokio::time::timeout(IO_LIMIT, server.task)
        .await
        .unwrap()
        .unwrap();
    assert!(outcome.is_complete());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drain_cut_aborts_the_connections_that_stay_open_and_counts_them() {
    let entered = Arc::new(Semaphore::new(0));
    let short_drain = ListenerSettings {
        drain_deadline: Duration::from_millis(200),
        ..settings()
    };
    let server = start(gated_app(&entered, Duration::from_secs(3600)), short_drain).await;
    let mut first = connect(server.address);
    let mut second = connect(server.address);
    send_get(&mut first, "/gated", false);
    send_get(&mut second, "/gated", false);
    wait_entered(&entered, 2).await;
    let outcome = server.finish().await;
    assert!(!outcome.is_complete());
    assert_eq!(outcome.cut(), 2);
    assert!(read_reply(&mut first).is_none());
    assert!(read_reply(&mut second).is_none());
}

/// The result of a drain while one handler does not yield.
struct BlockedDrain {
    outcome: DrainOutcome,
    waited: Duration,
    /// True when the handler had ended before the drain returned. A correct drain leaves it false.
    handler_ended: bool,
}

/// Stop a loop whose one handler does not yield. Release the handler after the drain returned.
///
/// `finish` limits the wait to `IO_LIMIT`, at least 25 times the deadlines of the tests. Only a
/// real hang reaches that limit, so a pause of the host cannot fail a correct drain.
async fn drain_with_blocked_handler(drain_deadline: Duration) -> BlockedDrain {
    let gate = HandlerGate::new();
    let _release = gate.release_on_drop();
    let settings = ListenerSettings {
        drain_deadline,
        ..settings()
    };
    let server = start(blocking_app(&gate), settings).await;
    let mut client = connect(server.address);
    send_get(&mut client, "/blocked", false);
    wait_entered(&gate.entered, 1).await;
    let started = Instant::now();
    let outcome = server.finish().await;
    BlockedDrain {
        outcome,
        waited: started.elapsed(),
        handler_ended: gate.handler_ended(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn drain_returns_at_the_deadline_when_a_handler_does_not_yield() {
    let deadline = Duration::from_millis(200);
    let drained = drain_with_blocked_handler(deadline).await;
    assert_eq!(drained.outcome.cut(), 1);
    assert!(!drained.handler_ended, "the drain waited for the handler");
    let waited = drained.waited;
    assert!(waited >= deadline, "ended after {waited:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn drain_of_zero_cuts_an_open_connection_at_once() {
    let drained = drain_with_blocked_handler(Duration::ZERO).await;
    assert_eq!(drained.outcome.cut(), 1);
    assert!(!drained.handler_ended, "the drain waited for the handler");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_closes_the_listener_before_the_drain_ends() {
    let entered = Arc::new(Semaphore::new(0));
    let one_second = ListenerSettings {
        drain_deadline: Duration::from_secs(1),
        ..settings()
    };
    let server = start(gated_app(&entered, Duration::from_secs(3600)), one_second).await;
    let mut client = connect(server.address);
    send_get(&mut client, "/gated", false);
    wait_entered(&entered, 1).await;
    server.stop.cancel();
    wait_until_refused(server.address).await;
    assert!(!server.task.is_finished(), "the drain ended too early");
    let outcome = tokio::time::timeout(IO_LIMIT, server.task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outcome.cut(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_connections_return_to_zero_after_100_closed_connections() {
    let long_header = ListenerSettings {
        header_read_timeout: Duration::from_secs(30),
        ..settings()
    };
    let server = start(app(), long_header).await;
    let idle: Vec<TcpStream> = (0..3).map(|_| connect(server.address)).collect();
    wait_for_open(&server.open, 3).await;
    drop(idle);
    wait_for_open(&server.open, 0).await;
    for _ in 0..CLOSED_CONNECTIONS {
        let mut client = connect(server.address);
        send_get(&mut client, "/ok", true);
        assert_eq!(read_reply(&mut client).unwrap().status, 200);
    }
    wait_for_open(&server.open, 0).await;
    assert!(server.finish().await.is_complete());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn panic_in_a_connection_task_ends_only_that_connection() {
    let server = start(app(), settings()).await;
    let mut first = connect(server.address);
    send_get(&mut first, "/boom", false);
    assert!(read_reply(&mut first).is_none());
    wait_for_open(&server.open, 0).await;
    let mut second = connect(server.address);
    send_get(&mut second, "/ok", true);
    assert_eq!(read_reply(&mut second).unwrap().status, 200);
    assert!(server.finish().await.is_complete());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keep_alive_connection_serves_the_next_request_after_a_caught_panic() {
    let server = start(app().layer(CatchPanicLayer::new()), settings()).await;
    let mut client = connect(server.address);
    send_get(&mut client, "/boom", false);
    assert_eq!(read_reply(&mut client).unwrap().status, 500);
    send_get(&mut client, "/ok", true);
    let reply = read_reply(&mut client).unwrap();
    assert_eq!((reply.status, reply.body.as_str()), (200, "ok"));
    assert!(server.finish().await.is_complete());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resource_fault_of_accept_pauses_the_loop_and_does_not_end_it() {
    let started = Instant::now();
    let server = start_faulty(2, ErrorKind::Other, settings()).await;
    let mut client = connect(server.address);
    send_get(&mut client, "/ok", true);
    assert_eq!(read_reply(&mut client).unwrap().status, 200);
    assert!(started.elapsed() >= ACCEPT_PAUSE.checked_mul(2).unwrap());
    assert!(server.finish().await.is_complete());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_of_a_connection_before_accept_does_not_pause_the_loop() {
    let long_pause = ListenerSettings {
        accept_error_pause: Duration::from_secs(2),
        ..settings()
    };
    let server = start_faulty(3, ErrorKind::ConnectionAborted, long_pause).await;
    let started = Instant::now();
    let mut client = connect(server.address);
    send_get(&mut client, "/ok", true);
    assert_eq!(read_reply(&mut client).unwrap().status, 200);
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(server.finish().await.is_complete());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_during_the_pause_after_an_accept_fault_ends_the_loop() {
    let long_pause = ListenerSettings {
        accept_error_pause: Duration::from_secs(3600),
        ..settings()
    };
    let server = start_faulty(usize::MAX, ErrorKind::Other, long_pause).await;
    tokio::time::sleep(HEADER_TIMEOUT).await;
    assert!(server.finish().await.is_complete());
}

/// The state of the log block of the next test. Only that test arms it.
static LOG_BLOCK_ARMED: AtomicBool = AtomicBool::new(false);
static LOG_BLOCK_ENTERED: AtomicBool = AtomicBool::new(false);
static LOG_BLOCK_RELEASED: AtomicBool = AtomicBool::new(false);

/// A subscriber that blocks one log call of a connection task until the test releases it.
///
/// The block applies only to the debug line of a bad request, so no other test meets it.
struct BlockingLog;

/// Finds the debug line of a connection that ended with a bad request method.
struct FindBadMethod(bool);

impl tracing::field::Visit for FindBadMethod {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "error" && format!("{value:?}").contains("invalid HTTP method") {
            self.0 = true;
        }
    }
}

impl tracing::Subscriber for BlockingLog {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        if !LOG_BLOCK_ARMED.load(Ordering::SeqCst) {
            return;
        }
        let mut found = FindBadMethod(false);
        event.record(&mut found);
        if !found.0 {
            return;
        }
        LOG_BLOCK_ENTERED.store(true, Ordering::SeqCst);
        while !LOG_BLOCK_RELEASED.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

/// Releases the blocked log call when the test ends, also when an assertion fails.
struct ReleaseLogBlock;

impl Drop for ReleaseLogBlock {
    fn drop(&mut self) {
        LOG_BLOCK_RELEASED.store(true, Ordering::SeqCst);
    }
}

/// Wait on the test thread, because a runtime timer may stall while a worker thread is blocked.
fn wait_for_log_block() {
    let deadline = Instant::now().checked_add(IO_LIMIT).unwrap();
    while !LOG_BLOCK_ENTERED.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "no task blocked in its log call");
        std::thread::sleep(POLL);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn drain_ends_at_the_deadline_when_a_finished_connection_blocks_in_its_log_call() {
    let _release = ReleaseLogBlock;
    tracing::subscriber::set_global_default(BlockingLog).unwrap();
    LOG_BLOCK_ARMED.store(true, Ordering::SeqCst);
    let deadline = Duration::from_millis(200);
    let short_drain = ListenerSettings {
        drain_deadline: deadline,
        ..settings()
    };
    let server = start(app(), short_drain).await;
    let mut client = connect(server.address);
    client
        .write_all(b"\x01 / HTTP/1.1\r\nHost: test\r\n\r\n")
        .unwrap();
    wait_for_log_block();
    server.stop.cancel();
    // A loose limit of 25 times the deadline. Only a drain that waits for the task reaches it.
    let outcome = tokio::time::timeout(IO_LIMIT, server.task)
        .await
        .expect("the drain ran past its deadline")
        .unwrap();
    assert_eq!(outcome.cut(), 1);
}
