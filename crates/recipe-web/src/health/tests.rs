//! Tests of the health router and the readiness cache (design D14 row T11, ADR-0059).
//!
//! Every test with time runs on a paused clock. The clock jumps to the next timer when no task is
//! ready, so no test waits for a real second. A future that never ends fails the test through
//! `bounded` and does not hang it.

use std::future::Future;
use std::hint::black_box;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use parking_lot::Mutex;
use recipe_app::memory::{MemoryClock, MemoryReadiness};
use recipe_app::operations::ReadinessState;
use recipe_app::ports::{Clock as _, NotReadyCause, Readiness, ReadinessProbe};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use tokio::sync::Notify;
use tokio::time::Instant;
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::fmt::MakeWriter;
use uuid::Uuid;

use super::{HealthSettings, HealthState, health_router};
use crate::settings::WebSettings;
use crate::test_support::{Fixture, PEER, Reply, call, capture_logs, get_with, request};

/// The start of the clock, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;
/// One second.
const SECOND: Duration = Duration::from_secs(1);
/// `health.readiness_cache` of `reliability.md` section 7.11.
const READINESS_CACHE: Duration = Duration::from_secs(5);
/// `health.db_check_timeout` of `reliability.md` section 7.11.
const DB_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
/// The bound of a test future. A request that never ends fails the test and does not hang it.
const TEST_BOUND: Duration = Duration::from_secs(7200);
/// The body of a ready answer and of a live answer (ADR-0059 (g)).
const OK_BODY: &str = "ok";
/// The body of a not ready answer (ADR-0059 (g)).
const NOT_READY_BODY: &str = "not ready";
/// The headers of `api.md` section 9 that every health answer carries.
const SECURITY_HEADERS: [&str; 5] = [
    "content-security-policy",
    "x-content-type-options",
    "referrer-policy",
    "strict-transport-security",
    "cache-control",
];

/// The gate of a probe that waits until the test lets it go.
#[derive(Default)]
struct Gate {
    /// The probe notifies it when the call starts.
    started: Notify,
    /// The test notifies it to let the call end.
    release: Notify,
}

/// What a scripted probe does before it answers.
enum Behavior {
    /// Answer at once.
    Plain,
    /// Wait for this time, then answer.
    Delay(Duration),
    /// Never answer.
    Hang,
    /// Wait at the gate, then answer.
    Gate(Arc<Gate>),
    /// Panic in the first call and answer in every later call.
    PanicOnFirstCall,
    /// Write one `WARN` event, then answer. It stands for the slow acquire of the health pool.
    Warn,
}

/// A probe over `MemoryReadiness` with a scripted delay or fault.
///
/// The `async_trait` macro is not a dependency of this crate, so the trait method has the
/// signature that the macro generates.
struct ScriptedProbe {
    /// The probe that gives the answer and counts the calls.
    inner: MemoryReadiness,
    /// What the probe does before it answers.
    behavior: Behavior,
    /// The number of calls that entered the probe.
    entered: AtomicU64,
    /// The probe notifies it at each entry, so a test can wait until a call has started.
    entry: Notify,
}

impl ScriptedProbe {
    fn new(behavior: Behavior) -> Self {
        Self {
            inner: MemoryReadiness::default(),
            behavior,
            entered: AtomicU64::new(0),
            entry: Notify::new(),
        }
    }

    async fn run(&self, timeout: Duration) -> Readiness {
        let earlier = self.entered.fetch_add(1, Ordering::SeqCst);
        self.entry.notify_one();
        match &self.behavior {
            Behavior::Plain => {}
            Behavior::Delay(wait) => tokio::time::sleep(*wait).await,
            Behavior::Hang => std::future::pending::<()>().await,
            Behavior::Gate(gate) => {
                gate.started.notify_one();
                gate.release.notified().await;
            }
            Behavior::PanicOnFirstCall => {
                if earlier == 0 {
                    black_box(None::<()>).expect("the test probe panics");
                }
            }
            Behavior::Warn => tracing::warn!("slow acquire in the probe"),
        }
        self.inner.check(timeout).await
    }
}

impl ReadinessProbe for ScriptedProbe {
    fn check<'life0, 'async_trait>(
        &'life0 self,
        timeout: Duration,
    ) -> Pin<Box<dyn Future<Output = Readiness> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(self.run(timeout))
    }
}

/// The parts of one test: the router and the handles behind it.
struct Harness {
    /// The health router.
    router: Router,
    /// The probe that the router calls.
    probe: Arc<ScriptedProbe>,
    /// The state that `serve` keeps.
    state: HealthState,
    /// The readiness state that the router writes.
    readiness: ReadinessState,
    /// The wall clock of the readiness state.
    clock: MemoryClock,
}

impl Harness {
    /// Build the harness with an open startup gate, like a server after step 10.
    fn new(behavior: Behavior) -> Self {
        let harness = Self::closed(behavior);
        harness.state.open_gate();
        harness
    }

    /// Build the harness with a closed startup gate, like a server before step 10.
    fn closed(behavior: Behavior) -> Self {
        let start = OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap();
        let clock = MemoryClock::new(start);
        let readiness = ReadinessState::new(Timestamp::from(start));
        let state = HealthState::new(
            readiness.clone(),
            Arc::new(clock.clone()),
            HealthSettings::default(),
        );
        let probe = Arc::new(ScriptedProbe::new(behavior));
        let router = health_router(probe.clone(), state.clone());
        Self {
            router,
            probe,
            state,
            readiness,
            clock,
        }
    }

    /// Return the number of calls that reached the probe.
    fn calls(&self) -> u64 {
        self.probe.inner.calls()
    }

    /// Send `GET` to the path and read the answer within the test bound.
    async fn get(&self, path: &str) -> Reply {
        bounded(get_with(&self.router, path, &[])).await
    }
}

/// Run `future` and fail the test when it needs more than `TEST_BOUND` of paused time.
async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(TEST_BOUND, future)
        .await
        .expect("the test future did not end within its bound")
}

/// Check that the answer is the plain text `body` with the status `status`.
fn assert_plain(reply: &Reply, status: StatusCode, body: &str) {
    assert_eq!(reply.status, status, "{}", reply.body);
    assert_eq!(reply.body, body);
    assert_eq!(reply.header("content-type"), "text/plain; charset=utf-8");
}

#[test]
fn default_settings_equal_the_table_of_section_7_11() {
    let settings = HealthSettings::default();
    assert_eq!(settings.readiness_cache, READINESS_CACHE);
    assert_eq!(settings.db_check_timeout, DB_CHECK_TIMEOUT);
    assert_eq!(
        settings.hsts_max_age,
        WebSettings::default().hsts_max_age,
        "http.hsts_max_age"
    );
}

#[tokio::test(start_paused = true)]
async fn healthz_answers_ok_without_a_call_of_the_probe() {
    let harness = Harness::new(Behavior::Plain);
    let reply = harness.get("/healthz").await;
    assert_plain(&reply, StatusCode::OK, OK_BODY);
    assert_eq!(harness.calls(), 0);
}

#[tokio::test(start_paused = true)]
async fn healthz_answers_ok_while_a_readyz_probe_hangs() {
    let harness = Harness::new(Behavior::Hang);
    let router = harness.router.clone();
    let start = Instant::now();
    let readyz = tokio::spawn(async move { get_with(&router, "/readyz", &[]).await });
    bounded(harness.probe.entry.notified()).await;
    let live = harness.get("/healthz").await;
    assert_plain(&live, StatusCode::OK, OK_BODY);
    assert!(
        start.elapsed() < DB_CHECK_TIMEOUT,
        "/healthz waited for the probe"
    );
    assert_eq!(harness.probe.entered.load(Ordering::SeqCst), 1);
    let ready = bounded(readyz).await.unwrap();
    assert_plain(&ready, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    assert_eq!(
        start.elapsed(),
        DB_CHECK_TIMEOUT,
        "the check timeout ends it"
    );
}

#[tokio::test(start_paused = true)]
async fn readyz_answers_ok_when_the_probe_is_ready() {
    let harness = Harness::new(Behavior::Plain);
    let reply = harness.get("/readyz").await;
    assert_plain(&reply, StatusCode::OK, OK_BODY);
    assert_eq!(harness.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn readyz_gives_the_check_timeout_of_section_7_11_to_the_probe() {
    let harness = Harness::new(Behavior::Plain);
    harness.get("/readyz").await;
    assert_eq!(harness.probe.inner.timeouts(), vec![DB_CHECK_TIMEOUT]);
}

#[tokio::test(start_paused = true)]
async fn readyz_answers_503_without_the_cause_for_every_cause() {
    for cause in [
        NotReadyCause::Unreachable,
        NotReadyCause::TimedOut,
        NotReadyCause::SchemaMismatch,
    ] {
        let harness = Harness::new(Behavior::Plain);
        harness.probe.inner.set(Readiness::NotReady(cause));
        let reply = harness.get("/readyz").await;
        assert_plain(&reply, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
        assert!(!reply.body.contains(&format!("{cause:?}")), "{cause:?}");
    }
}

/// The startup gate (finding CRX-S8-2, ADR-0061 step 10). `/readyz` stays `not ready` until
/// `serve` opens the gate after the bind of the public listener. A closed gate starts no probe.
#[tokio::test(start_paused = true)]
async fn readyz_answers_503_without_a_probe_call_while_the_gate_is_closed() {
    let harness = Harness::closed(Behavior::Plain);
    for _ in 0..2 {
        let reply = harness.get("/readyz").await;
        assert_plain(&reply, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
        tokio::time::advance(READINESS_CACHE).await;
    }
    assert_eq!(harness.calls(), 0, "a closed gate starts no probe");
    assert!(!harness.readiness.is_ready());
    let live = harness.get("/healthz").await;
    assert_plain(&live, StatusCode::OK, OK_BODY);
}

#[tokio::test(start_paused = true)]
async fn readyz_runs_a_fresh_probe_at_once_when_the_gate_opens() {
    let harness = Harness::closed(Behavior::Plain);
    let closed = harness.get("/readyz").await;
    assert_plain(&closed, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    harness.state.open_gate();
    let open = harness.get("/readyz").await;
    assert_plain(&open, StatusCode::OK, OK_BODY);
    assert_eq!(harness.calls(), 1, "the closed gate left no cached entry");
    assert!(harness.readiness.is_ready());
}

#[tokio::test(start_paused = true)]
async fn the_shutdown_wins_over_an_open_gate() {
    let harness = Harness::closed(Behavior::Plain);
    harness.state.begin_shutdown();
    harness.state.open_gate();
    let reply = harness.get("/readyz").await;
    assert_plain(&reply, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    assert_eq!(harness.calls(), 0);
    assert!(!harness.readiness.is_ready());
}

#[tokio::test(start_paused = true)]
async fn readyz_keeps_the_result_for_the_cache_time() {
    let harness = Harness::new(Behavior::Plain);
    harness.get("/readyz").await;
    tokio::time::advance(READINESS_CACHE - SECOND).await;
    let reply = harness.get("/readyz").await;
    assert_plain(&reply, StatusCode::OK, OK_BODY);
    assert_eq!(harness.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn readyz_calls_the_probe_again_when_the_cache_time_has_passed() {
    let harness = Harness::new(Behavior::Plain);
    harness.get("/readyz").await;
    tokio::time::advance(READINESS_CACHE).await;
    harness.get("/readyz").await;
    assert_eq!(harness.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn readyz_keeps_a_not_ready_result_for_the_cache_time() {
    let harness = Harness::new(Behavior::Plain);
    harness
        .probe
        .inner
        .set(Readiness::NotReady(NotReadyCause::Unreachable));
    harness.get("/readyz").await;
    harness.probe.inner.set(Readiness::Ready);
    tokio::time::advance(READINESS_CACHE - SECOND).await;
    let cached = harness.get("/readyz").await;
    assert_plain(&cached, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    tokio::time::advance(SECOND).await;
    let fresh = harness.get("/readyz").await;
    assert_plain(&fresh, StatusCode::OK, OK_BODY);
    assert_eq!(harness.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn parallel_readyz_requests_share_one_call_of_the_probe() {
    let harness = Harness::new(Behavior::Delay(SECOND));
    let (first, second, third) = bounded(async {
        tokio::join!(
            get_with(&harness.router, "/readyz", &[]),
            get_with(&harness.router, "/readyz", &[]),
            get_with(&harness.router, "/readyz", &[]),
        )
    })
    .await;
    for reply in [first, second, third] {
        assert_plain(&reply, StatusCode::OK, OK_BODY);
    }
    assert_eq!(harness.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn readyz_ends_a_probe_that_hangs_after_the_check_timeout() {
    let harness = Harness::new(Behavior::Hang);
    let start = Instant::now();
    let reply = harness.get("/readyz").await;
    assert_plain(&reply, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    assert_eq!(start.elapsed(), DB_CHECK_TIMEOUT);
    let cached = harness.get("/readyz").await;
    assert_plain(&cached, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    assert_eq!(start.elapsed(), DB_CHECK_TIMEOUT, "the cache answers");
    assert_eq!(harness.probe.entered.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn readyz_turns_the_readiness_state_on_after_the_first_successful_check() {
    let harness = Harness::new(Behavior::Plain);
    assert!(!harness.readiness.is_ready(), "off before the first check");
    harness.get("/readyz").await;
    assert!(harness.readiness.is_ready());
}

#[tokio::test(start_paused = true)]
async fn readyz_turns_the_readiness_state_off_after_a_failed_check() {
    let harness = Harness::new(Behavior::Plain);
    harness.get("/readyz").await;
    harness
        .probe
        .inner
        .set(Readiness::NotReady(NotReadyCause::SchemaMismatch));
    tokio::time::advance(READINESS_CACHE).await;
    harness.clock.advance(time::Duration::seconds(30));
    harness.get("/readyz").await;
    assert!(!harness.readiness.is_ready());
    let since = harness.readiness.not_ready_for(harness.clock.now());
    assert_eq!(since, Some(Duration::ZERO));
}

#[tokio::test(start_paused = true)]
async fn shutdown_turns_readyz_off_at_once_and_keeps_healthz_on() {
    let harness = Harness::new(Behavior::Plain);
    harness.get("/readyz").await;
    tokio::time::advance(READINESS_CACHE).await;
    harness.state.begin_shutdown();
    let ready = harness.get("/readyz").await;
    assert_plain(&ready, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    let live = harness.get("/healthz").await;
    assert_plain(&live, StatusCode::OK, OK_BODY);
    assert_eq!(harness.calls(), 1, "no call during the shutdown");
    assert!(!harness.readiness.is_ready());
}

#[tokio::test(start_paused = true)]
async fn a_check_that_ends_after_the_shutdown_does_not_turn_readiness_on() {
    let gate = Arc::new(Gate::default());
    let harness = Harness::new(Behavior::Gate(Arc::clone(&gate)));
    let router = harness.router.clone();
    let request = tokio::spawn(async move { get_with(&router, "/readyz", &[]).await });
    bounded(gate.started.notified()).await;
    harness.state.begin_shutdown();
    gate.release.notify_one();
    let reply = bounded(request).await.unwrap();
    assert_plain(&reply, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    assert!(!harness.readiness.is_ready());
}

/// A request that waits for the lock gets it after the signal. The cache is still empty then, but
/// the request must not start a probe call (ADR-0091 (b)).
#[tokio::test(start_paused = true)]
async fn a_request_that_waits_for_the_lock_starts_no_probe_after_the_shutdown() {
    let gate = Arc::new(Gate::default());
    let harness = Harness::new(Behavior::Gate(Arc::clone(&gate)));
    let router = harness.router.clone();
    let holder = tokio::spawn(async move { get_with(&router, "/readyz", &[]).await });
    bounded(gate.started.notified()).await;
    let router = harness.router.clone();
    let waiter = tokio::spawn(async move { get_with(&router, "/readyz", &[]).await });
    bounded(tokio::time::sleep(SECOND)).await;
    harness.state.begin_shutdown();
    holder.abort();
    assert!(
        bounded(holder)
            .await
            .is_err_and(|error| error.is_cancelled())
    );
    let reply = bounded(waiter).await.unwrap();
    assert_plain(&reply, StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY);
    assert_eq!(harness.probe.entered.load(Ordering::SeqCst), 1);
    assert!(!harness.readiness.is_ready());
}

/// `record` reads the flag and then stores. A signal between both steps acts like `apply` after it.
#[test]
fn a_result_that_lands_after_the_signal_never_leaves_readiness_on() {
    let start = OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap();
    let results = [
        Readiness::Ready,
        Readiness::NotReady(NotReadyCause::TimedOut),
    ];
    for result in results {
        let readiness = ReadinessState::new(Timestamp::from(start));
        let state = HealthState::new(
            readiness.clone(),
            Arc::new(MemoryClock::new(start)),
            HealthSettings::default(),
        );
        state.begin_shutdown();
        state.apply(result);
        assert!(!readiness.is_ready(), "late result: {result:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_panic_in_the_probe_ends_only_its_request() {
    let harness = Harness::new(Behavior::PanicOnFirstCall);
    let router = harness.router.clone();
    let failed = bounded(tokio::spawn(async move {
        get_with(&router, "/readyz", &[]).await
    }))
    .await;
    assert!(
        failed.is_err_and(|error| error.is_panic()),
        "no CatchPanicLayer"
    );
    let next = harness.get("/readyz").await;
    assert_plain(&next, StatusCode::OK, OK_BODY);
}

#[tokio::test(start_paused = true)]
async fn every_answer_has_the_request_id_and_the_security_headers() {
    let harness = Harness::new(Behavior::Plain);
    let mut seen = Vec::new();
    for path in ["/healthz", "/readyz", "/missing"] {
        let reply = harness.get(path).await;
        let id = reply.header("x-request-id");
        let parsed = Uuid::parse_str(&id).unwrap();
        assert_eq!(parsed.get_version_num(), 7, "{path}: {id}");
        assert!(!seen.contains(&id), "{path}: the id repeats");
        seen.push(id);
        for name in SECURITY_HEADERS {
            assert!(!reply.header(name).is_empty(), "{path}: {name}");
        }
    }
}

#[tokio::test(start_paused = true)]
async fn two_health_requests_get_two_different_version_7_ids() {
    let harness = Harness::new(Behavior::Plain);
    let first = harness.get("/healthz").await.header("x-request-id");
    let second = harness.get("/healthz").await.header("x-request-id");
    assert_ne!(first, second);
    for id in [first, second] {
        let parsed = Uuid::parse_str(&id).unwrap();
        assert_eq!(parsed.get_version_num(), 7, "{id}");
        assert!(!parsed.is_nil(), "{id}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_security_headers_equal_those_of_the_public_router() {
    let harness = Harness::new(Behavior::Plain);
    let fixture = Fixture::new();
    let public = crate::router(fixture.state.clone()).layer(MockConnectInfo(PEER));
    let reference = bounded(get_with(&public, "/missing", &[])).await;
    let health = harness.get("/healthz").await;
    for name in SECURITY_HEADERS {
        assert!(!reference.header(name).is_empty(), "{name}");
        assert_eq!(health.header(name), reference.header(name), "{name}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_public_router_answers_404_for_both_health_paths() {
    let fixture = Fixture::new();
    let public = crate::router(fixture.state.clone()).layer(MockConnectInfo(PEER));
    for path in ["/healthz", "/readyz"] {
        let reply = bounded(get_with(&public, path, &[])).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_health_router_needs_no_client_address() {
    let harness = Harness::new(Behavior::Plain);
    let headers = [("x-forwarded-for", "not an address")];
    for path in ["/healthz", "/readyz"] {
        let reply = bounded(get_with(&harness.router, path, &headers)).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    }
}

#[tokio::test(start_paused = true)]
async fn the_health_router_has_no_csrf_check_and_no_session() {
    let harness = Harness::new(Behavior::Plain);
    let post = request(Method::POST, "/healthz")
        .body(Body::empty())
        .unwrap();
    let reply = bounded(call(&harness.router, post)).await;
    assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
    assert!(reply.set_cookies().is_empty());
    let live = harness.get("/healthz").await;
    assert!(live.set_cookies().is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_head_request_gets_the_status_without_a_body() {
    let harness = Harness::new(Behavior::Plain);
    let head = request(Method::HEAD, "/healthz")
        .body(Body::empty())
        .unwrap();
    let reply = bounded(call(&harness.router, head)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, "");
}

/// Return the lines of `text` that the trace layer writes at the end of a request.
fn finish_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.contains("request finished"))
        .collect()
}

#[tokio::test(start_paused = true)]
async fn the_finish_line_has_the_level_debug_for_every_status() {
    let harness = Harness::new(Behavior::Plain);
    harness
        .probe
        .inner
        .set(Readiness::NotReady(NotReadyCause::Unreachable));
    let (logs, _guard) = capture_logs();
    let live = harness.get("/healthz").await;
    let not_ready = harness.get("/readyz").await;
    let missing = harness.get("/missing").await;
    let text = logs.text();
    let lines = finish_lines(&text);
    assert_eq!(lines.len(), 3, "{text}");
    for (line, reply) in lines.iter().zip([&live, &not_ready, &missing]) {
        assert!(line.contains("DEBUG"), "{line}");
        let id = reply.header("x-request-id");
        assert!(line.contains(&format!("request_id={id}")), "{line}");
        assert!(
            line.contains(&format!("status={}", reply.status.as_u16())),
            "{line}"
        );
    }
    for level in ["INFO", "WARN", "ERROR"] {
        assert!(!text.contains(level), "{level}: {text}");
    }
}

/// A writer that copies every log line into a buffer that only one test owns.
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl io::Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Sink {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Sink {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock()).into_owned()
    }
}

/// Install a subscriber for this thread with the production filter `info` (`reliability.md`
/// section 3.3). Keep the guard while the test runs.
fn production_logs() -> (Sink, DefaultGuard) {
    let sink = Sink::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(sink.clone())
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .finish();
    (sink, tracing::subscriber::set_default(subscriber))
}

#[tokio::test(start_paused = true)]
async fn a_warn_event_in_a_health_request_has_the_request_id_at_the_filter_info() {
    let harness = Harness::new(Behavior::Warn);
    let (sink, _guard) = production_logs();
    let reply = harness.get("/readyz").await;
    let id = reply.header("x-request-id");
    let text = sink.text();
    let warns: Vec<&str> = text.lines().filter(|line| line.contains("WARN")).collect();
    assert_eq!(warns.len(), 1, "{text}");
    assert!(
        warns
            .iter()
            .all(|line| line.contains(&format!("request_id={id}"))),
        "{text}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_successful_health_request_writes_no_line_at_the_filter_info() {
    let harness = Harness::new(Behavior::Plain);
    let (sink, _guard) = production_logs();
    harness.get("/healthz").await;
    harness.get("/readyz").await;
    assert_eq!(sink.text(), "", "the span and the finish line stay silent");
}
