# ADR-0058: HTTP serving loop and request timeouts

Status: Accepted. Amended by ADR-0072 (export routes outside the request timeout). Amended by ADR-0085 (the pause `http.accept_error_pause` after an accept fault). Amended by ADR-0087 (the drain of 0 s of the internal listener, the abort without a wait). Amended by ADR-0088 (the three numbers of the stop `WARN` line). Amended by ADR-0089 (c) (item (o): the stop `WARN` line only when a count is above 0)
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 adds the subcommand `serve`. `reliability.md` section 7.4 names the mechanism of each timeout, and section 7.3 gives the graceful shutdown. The plan found these gaps:

- `axum::serve` of axum 0.8.9 builds the hyper connection without a timer. Without a timer, hyper has no header read timeout.
- `axum::serve` starts each connection task as a detached task. Its graceful shutdown waits for these tasks without a bound.
- hyper has no body read timeout. Section 7.4 names "`hyper` header and body read timeouts" for `http.body_read_timeout`.
- `TimeoutLayer::new` of `tower-http` 0.7.1 is deprecated. The answer of `TimeoutLayer` has an empty body, so it does not give the fragment of `E-WEB-009`.
- `csrf.rs` maps every error of the body read to `E-WEB-003`. A body that stops arriving then shows "too large".
- Section 7.3 gives no bound for the pool close and the runtime stop after `shutdown.drain`. Section 7.11 has no key for them.
- The `Drop` of a Tokio runtime waits without a bound for `spawn_blocking` work.
- The watchdog of section 2.4 rule 5 probes the internal listener. When the internal listener stops during the drain, the watchdog stops the process with code 70.

## Decision drivers

- After `SIGTERM`, the process exits with code 0 within `app.stop_grace_period` (section 7.3, R30).
- Every timeout has one code, `E-WEB-009` (section 1.1). No new error code.
- A slow client cannot hold a connection without a bound (R30).
- Every number comes from section 7.11.
- The internal listener and the watchdog stay consistent during the drain (A41).

## Considered options

### Option A: An own accept loop, a typed request timeout, and a body deadline

- Pros: the header read timeout, a bounded drain, and one code for each timeout. The loop owns every connection task.
- Cons: new code on the request path. The tests must cover the drain, the cut, and keep-alive.

### Option B: `axum::serve`, `TimeoutLayer::with_status_code`, and the timeouts of Caddy

- Pros: less code in `recipe-server`.
- Cons: `app` itself has no header read timeout. A layer must turn the bare `503` of `TimeoutLayer` into `E-WEB-009`. The timeouts of Caddy need more rows in section 7.11.

### Option C: `axum::serve` without a change

- Pros: no new code.
- Cons: no header read timeout and no bound of the drain. A body timeout gives `E-WEB-003`.

## Decision

The project uses **Option A**.

Accept loop:

- (a) `recipe-server` serves each listener with one own accept loop. The loop uses `hyper::server::conn::http1::Builder` with `TokioTimer` and `header_read_timeout` set to `http.body_read_timeout`. HTTP/2 stays off.
- (b) The loop wraps each connection in `hyper_util::server::graceful::GracefulShutdown`. A `tokio::task::JoinSet` owns every connection task.
- (c) The public listener and the internal listener use the same loop and the same timeouts. A neighbor container therefore cannot stall port 9000 without a bound.
- (d) The loop puts `ConnectInfo<SocketAddr>` into each request, because the layer `client_address` gives `E-WEB-010` without it.
- (e) The loop calls `join_next` in its `select!`, so a finished connection task leaves the `JoinSet` at once. The memory of a 24/7 process then stays bounded.
- (f) The `JoinSet` catches a panic of a connection task. That panic ends only its connection, and the panic hook logs and counts it. The `JoinSet` is the fifth place of section 2.4 rule 6 (ADR-0057 (h)).

Timeouts:

- (g) The request timeout is a `from_fn` layer that wraps `next.run` in `tokio::time::timeout` with `http.request_timeout`. On the limit it returns `AppError::Timeout { stage: Request }`. This replaces `TimeoutLayer` in section 7.4 and in the timeout rule of section 1.1.
- (h) The request timeout sits directly inside the inner `CatchPanicLayer` (ADR-0057). `render_errors` gives `503 E-WEB-009` with `Retry-After` set to `http.retry_after`, and the JSON body on `/export/v1/`.
- (i) The body deadline is `tower_http::timeout::RequestBodyDeadlineLayer` with `http.body_read_timeout`. It sets one deadline for the whole body. `RequestBodyTimeoutLayer` is not used, because it starts its timer again on each frame.
- (j) `csrf.rs` walks the `source()` chain of a body error. A `tower_http::timeout::TimeoutError` gives `AppError::Timeout { stage: Request }` (`E-WEB-009`). Every other body error stays `E-WEB-003`.
- (k) hyper starts the header read timeout again when a keep-alive connection is idle. `app` therefore closes an idle connection after `http.body_read_timeout`. ADR-0065 keeps the idle time of the proxy below this value.

Stop:

- (l) On the signal, readiness turns off, the watchdog stops, the public listener stops accepting, and the jobs get the cancel (ADR-0061).
- (m) The drain of the public connections and of the open job iterations runs together under `shutdown.drain`. After the limit, `JoinSet::shutdown` aborts the open connections. An open transaction then rolls back.
- (n) Then both pools close within `shutdown.pool_close`, and the internal listener stops. The internal listener serves during the drain. `/healthz` answers `ok`, and `/readyz` answers `not ready`.
- (o) Last, `Runtime::shutdown_timeout` runs with `shutdown.runtime_stop`. The exit code is 0, also after a cut drain. One `WARN` line gives the count of the cut connections.
- (p) Section 7.3 gets two sum rules. The first rule is `shutdown.drain + shutdown.pool_close + shutdown.runtime_stop < app.stop_grace_period`, which is 30 s + 5 s + 3 s < 40 s. The second rule is `http.request_timeout <= shutdown.drain`. A test reads both from section 7.11.

Settings and fallback:

- (q) Section 7.11 gets the row `shutdown.pool_close` / `shutdown.runtime_stop` with the values 5 s / 3 s.
- (r) The fallback of this decision is `axum::serve` with a graceful shutdown under a bound. The fallback loses the header read timeout of item (a), so a switch to it is a new decision.

## Consequences

### Positive

- A client that does not send its headers within 10 s loses its connection. A body that stops arriving gives `E-WEB-009`.
- The stop of `app` ends within its grace period, also with a slow request.
- Every timeout of a request has one code and one answer shape.

### Negative

- `recipe-server` owns an accept loop. The code is on the path of every request.
- An idle keep-alive connection closes after 10 s. The proxy opens new connections more often.
- A request that starts just before `SIGTERM` can get cut at the end of the drain.

### Risks and mitigations

- Risk: the loop leaks connection tasks or loses `ConnectInfo`. Mitigation: unit tests of the loop cover the complete drain, the cut drain, the header read timeout on both listeners, and keep-alive after a panic. They also cover `ConnectInfo` in each request and an empty `JoinSet` after 100 closed connections.

## Related

- ADR-0057 (panic layers), ADR-0059 (health router), ADR-0060 (supervisor of the jobs), ADR-0061 (start and signal), ADR-0065 (keep-alive of the proxy)
- `docs/architecture/reliability.md` section 1.1 (the timeout rule), section 2.4 rule 6, sections 7.3, 7.4, and 7.11
- `.claude/rules/rust.md` (the count of the places that catch a panic)
- Baseline requirements: operations "Request timeout" and "Graceful shutdown"
- `overview.md` section 7: R30. Section 9: A41
