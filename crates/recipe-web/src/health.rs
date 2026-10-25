//! The health router and the readiness cache (design D5, ADR-0059, `reliability.md` section 7.2).
//!
//! The internal listener serves this router. `/healthz` answers without the database. `/readyz`
//! answers from a cache of the readiness probe. The router has the layers `request_id`, the
//! trace layer with a finish line at `DEBUG`, and the security headers of `api.md` section 9. It
//! has no `client_address`, no session, no CSRF check, and no `CatchPanicLayer`. A panic in a
//! handler ends only its connection (ADR-0058 (f), ADR-0059 (m)).

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, fence};
use std::time::Duration;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use recipe_app::operations::ReadinessState;
use recipe_app::ports::{Clock, NotReadyCause, Readiness, ReadinessProbe};
use tokio::sync::Mutex;
use tokio::time::Instant;
use tower_http::trace::TraceLayer;
use tracing::Span;

use crate::pipeline::{request_id, request_id_of, request_id_state, security_headers};
use crate::settings::WebSettings;

/// Default of `health.readiness_cache`: 5 s (`reliability.md` section 7.11).
const DEFAULT_READINESS_CACHE: Duration = Duration::from_secs(5);
/// Default of `health.db_check_timeout`: 2 s (`reliability.md` section 7.11).
const DEFAULT_DB_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
/// The body of a live answer and of a ready answer (ADR-0059 (g)).
const OK_BODY: &str = "ok";
/// The body of a not ready answer. It names no cause (ADR-0059 (g)).
const NOT_READY_BODY: &str = "not ready";

/// The numeric settings of the health router.
///
/// The type solves one problem: the web layer must not read a configuration file.
/// `recipe-server` builds this value and passes it down. `Default` equals the table of
/// `reliability.md` section 7.11, and a test compares both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealthSettings {
    /// How long `/readyz` keeps the result of a check (`health.readiness_cache`).
    pub readiness_cache: Duration,
    /// The time that one readiness check may take (`health.db_check_timeout`).
    pub db_check_timeout: Duration,
    /// The `max-age` of `Strict-Transport-Security` (`http.hsts_max_age`).
    pub hsts_max_age: Duration,
}

impl Default for HealthSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            readiness_cache: DEFAULT_READINESS_CACHE,
            db_check_timeout: DEFAULT_DB_CHECK_TIMEOUT,
            hsts_max_age: WebSettings::default().hsts_max_age,
        }
    }
}

/// The handles of the health router that `serve` keeps.
///
/// The type solves one problem: `serve` must turn readiness off at the signal (ADR-0058 (l)).
/// The router and the job `alerts` must share the readiness state.
/// A clone shares all handles.
#[derive(Clone)]
pub struct HealthState {
    inner: Arc<StateParts>,
}

/// The parts that every clone of [`HealthState`] shares.
struct StateParts {
    /// The readiness state that the router writes after each check.
    readiness: ReadinessState,
    /// The source of the instant that the readiness state stores.
    clock: Arc<dyn Clock>,
    /// The numeric settings.
    settings: HealthSettings,
    /// True after the signal. `/readyz` then answers `not ready` without a check.
    draining: AtomicBool,
    /// False until `serve` binds the public listener (ADR-0061 step 10, ADR-0091 (e)).
    /// `/readyz` then answers `not ready` without a check.
    gate_open: AtomicBool,
}

impl HealthState {
    /// Build the state. The startup gate is closed, so readiness stays off until `serve` calls
    /// [`HealthState::open_gate`] and the first check succeeds (ADR-0061, ADR-0091 (e)).
    #[must_use]
    pub fn new(readiness: ReadinessState, clock: Arc<dyn Clock>, settings: HealthSettings) -> Self {
        Self {
            inner: Arc::new(StateParts {
                readiness,
                clock,
                settings,
                draining: AtomicBool::new(false),
                gate_open: AtomicBool::new(false),
            }),
        }
    }

    /// Open the startup gate for the rest of the process. `serve` calls it after the bind of the
    /// public listener (ADR-0061 step 10, ADR-0091 (e)).
    ///
    /// The gate solves one problem: the internal listener opens at step 8.
    /// `/readyz` must not answer `ok` before the public port is open.
    /// A closed gate starts no probe and caches no result.
    /// The first request after the gate opens runs a fresh check.
    /// The drain still wins (ADR-0091 (b)).
    pub fn open_gate(&self) {
        self.inner.gate_open.store(true, Ordering::SeqCst);
    }

    fn is_gate_open(&self) -> bool {
        self.inner.gate_open.load(Ordering::SeqCst)
    }

    /// Turn readiness off for the rest of the process. `serve` calls it at the signal.
    ///
    /// `/healthz` keeps answering `ok`. `/readyz` answers `not ready` without a check
    /// (`reliability.md` section 7.3).
    pub fn begin_shutdown(&self) {
        self.inner.draining.store(true, Ordering::SeqCst);
        // This fence pairs with the fence in `apply`. The drain must win over a late `Ready`,
        // because the state uses `Relaxed` accesses.
        fence(Ordering::SeqCst);
        self.inner.readiness.mark_not_ready(self.inner.clock.now());
    }

    fn is_draining(&self) -> bool {
        self.inner.draining.load(Ordering::SeqCst)
    }

    /// Store the result of a check in the readiness state, unless the shutdown has begun.
    fn record(&self, readiness: Readiness) {
        if self.is_draining() {
            return;
        }
        self.apply(readiness);
    }

    /// Write the result to the readiness state. The shutdown wins over a late result.
    ///
    /// The signal can come between the check in `record` and the write. The second read of the
    /// flag then turns readiness off again (ADR-0091 (b)).
    fn apply(&self, readiness: Readiness) {
        match readiness {
            Readiness::Ready => self.inner.readiness.mark_ready(),
            Readiness::NotReady(cause) => {
                tracing::debug!(?cause, "readiness check failed");
                self.inner.readiness.mark_not_ready(self.inner.clock.now());
            }
        }
        // This fence pairs with the fence in `begin_shutdown`. The drain must win over a late
        // `Ready`, because the state uses `Relaxed` accesses.
        fence(Ordering::SeqCst);
        if self.is_draining() {
            self.inner.readiness.mark_not_ready(self.inner.clock.now());
        }
    }
}

impl fmt::Debug for HealthState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HealthState")
            .field("draining", &self.is_draining())
            .finish_non_exhaustive()
    }
}

/// One result of the readiness probe and the instant when it arrived.
struct Cached {
    at: Instant,
    readiness: Readiness,
}

/// The state of the two handlers: the probe, the shared handles, and the cache.
#[derive(Clone)]
struct Checker {
    probe: Arc<dyn ReadinessProbe>,
    state: HealthState,
    /// The async lock is held during the probe call, so only one call runs at a time. A request
    /// that waits for the lock then reads the result that the first call stored.
    cache: Arc<Mutex<Option<Cached>>>,
}

impl Checker {
    /// Return true when the server is ready.
    ///
    /// The answer is `false` while the startup gate is closed or after the signal, with no probe
    /// call. Otherwise the cache keeps the result, ready or not, for `health.readiness_cache`. Only
    /// one probe call runs at a time, so a slow database never makes the calls pile up.
    async fn is_ready(&self) -> bool {
        if self.state.is_draining() || !self.state.is_gate_open() {
            return false;
        }
        let ttl = self.state.inner.settings.readiness_cache;
        let mut cache = self.cache.lock().await;
        // The signal can come while this request waits for the lock. Then no probe call starts
        // (ADR-0091 (b)).
        if self.state.is_draining() {
            return false;
        }
        if let Some(entry) = cache.as_ref().filter(|entry| entry.at.elapsed() < ttl) {
            return entry.readiness.is_ready() && !self.state.is_draining();
        }
        let readiness = self.run_probe().await;
        self.state.record(readiness);
        *cache = Some(Cached {
            at: Instant::now(),
            readiness,
        });
        readiness.is_ready() && !self.state.is_draining()
    }

    /// Run the probe and stop it after `health.db_check_timeout`.
    ///
    /// The port stops the check itself. The outer limit covers a probe that breaks that rule.
    async fn run_probe(&self) -> Readiness {
        let limit = self.state.inner.settings.db_check_timeout;
        tokio::time::timeout(limit, self.probe.check(limit))
            .await
            .unwrap_or(Readiness::NotReady(NotReadyCause::TimedOut))
    }
}

/// Answer `GET /healthz`. The process is up and the router answers.
async fn live() -> Response {
    (StatusCode::OK, OK_BODY).into_response()
}

/// Answer `GET /readyz` from the cache of the readiness probe.
async fn ready(State(checker): State<Checker>) -> Response {
    if checker.is_ready().await {
        (StatusCode::OK, OK_BODY).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, NOT_READY_BODY).into_response()
    }
}

/// Create the span of a request. It sits at `INFO`, so the filter `info` keeps it.
///
/// A `WARN` or `ERROR` event inside the request then carries the `request_id`
/// (`reliability.md` section 3.2). The span writes no line. Only the finish line is `DEBUG`.
fn make_span(request: &Request) -> Span {
    tracing::info_span!(
        "http_request",
        request_id = %request_id_of(request),
        method = %request.method(),
        status = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// Record the status and the latency, then write the one `DEBUG` line of the request.
///
/// The level is `DEBUG` for every status, a 503 included (`reliability.md` section 3.3).
fn finish_span(response: &Response, latency: Duration, span: &Span) {
    let millis = u64::try_from(latency.as_millis().min(u128::from(u64::MAX))).unwrap_or(u64::MAX);
    span.record("status", response.status().as_u16());
    span.record("latency_ms", millis);
    tracing::debug!("request finished");
}

/// Add the request id, the trace layer, and the security headers of `api.md` section 9.
///
/// The request id and the security headers come from `pipeline`, so both routers share one rule.
/// A layer that is added later runs earlier. The order, outer first, is `request_id`, the trace
/// layer, the CSP, `nosniff`, the referrer policy, HSTS, and `Cache-Control`.
fn wrap(router: Router, hsts_max_age: Duration) -> Router {
    security_headers(router, hsts_max_age)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_span)
                .on_request(())
                .on_response(finish_span)
                .on_eos(())
                .on_failure(()),
        )
        .layer(from_fn_with_state(request_id_state(), request_id))
}

/// Build the health router of the internal listener.
///
/// The router serves `GET /healthz` and `GET /readyz` (ADR-0059 (g)). It reads the readiness
/// from `probe` through a cache with one call at a time. The readiness state and the shutdown
/// flag live in `state`, which `serve` keeps a clone of.
pub fn health_router(probe: Arc<dyn ReadinessProbe>, state: HealthState) -> Router {
    let hsts_max_age = state.inner.settings.hsts_max_age;
    let checker = Checker {
        probe,
        state,
        cache: Arc::default(),
    };
    let routes = Router::new()
        .route("/healthz", get(live))
        .route("/readyz", get(ready))
        .with_state(checker);
    wrap(routes, hsts_max_age)
}

#[cfg(test)]
mod tests;
