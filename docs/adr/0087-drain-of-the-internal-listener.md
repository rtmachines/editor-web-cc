# ADR-0087: Drain of the internal listener

Status: Accepted. Amended by ADR-0088 (the cut count of the jobs as the third number of the `WARN` line of the stop)
Date: 2026-10-24
Deciders: project owner

## Context

ADR-0058 gives the accept loop and the stop of `app`. Task 7.4 writes the loop. The architecture review of the loop found three gaps:

1. Step 5 of `reliability.md` section 7.3 stops the internal listener after the pool close. `AcceptLoop::run` always drains with `ListenerSettings::drain_deadline`. ADR-0058 (c) gives both listeners the same timeouts.
2. With `shutdown.drain` on both listeners, the stop takes 30 s + 5 s + 30 s + 3 s = 68 s. Docker sends `SIGKILL` after `app.stop_grace_period`, which is 40 s. The sum rule of section 7.3 does not count the drain of the internal listener.
3. After the drain deadline, the loop calls `JoinSet::shutdown`. This call aborts each task and then waits for each task to reach an `.await` point, with no bound. A task that does not yield keeps that wait for ever. The process then ends only by `SIGKILL`.

Facts of the stop:

- `Pool::close` of sqlx 0.8.6 waits until each checked-out connection returns or closes. Step 5 bounds this wait with `shutdown.pool_close`.
  - After a pool close that completes, no transaction of `app` is open.
  - When `shutdown.pool_close` expires, a task that does not yield can keep its connection until the process exits. PostgreSQL then rolls its transaction back.
- The health router of the internal listener opens no write transaction (ADR-0059).
- The watchdog stops at the signal (ADR-0058 (l)). After the pool close, the only requests on the internal listener are probes. A probe comes from the subcommand `healthcheck`, or from the watchdog before the signal.

## Decision drivers

- After `SIGTERM`, the process exits with code 0 within `app.stop_grace_period` (`reliability.md` section 7.3, R30).
- Every number has one row in `reliability.md` section 7.11.
- No new configuration key. Section 7.12 keeps its 18 keys (ADR-0062 (a)).
- An abort never leaves a partial write (R29). After a pool close that completes, no aborted handler holds an open transaction (ADR-0058 (m)).
  - When `shutdown.pool_close` expires, a task that does not yield can keep its transaction until the process exits. PostgreSQL then rolls that transaction back.

## Considered options for the drain of the internal listener

### Option A: A drain of 0 s as a code default

- Pros: the stop stays below `app.stop_grace_period`. No new configuration key. A cut loses no data. After a pool close that completes, no transaction is open. When `shutdown.pool_close` expires, PostgreSQL rolls back each open transaction at the exit of the process.
- Cons: a probe at that time loses its connection and gets no answer.

### Option B: A new configuration key inside the sum rule

- Pros: a probe at that time can finish.
- Cons: a new key with no known use. Another value of the sum rule must decrease to keep the stop below 40 s.

The user chose **Option A** on 2026-10-24.

## Considered options for the wait after the abort

### Option A: Abort and do not wait

- Pros: no new number. The drain of each listener ends at its deadline. The pool close of step 5 already waits for the connections of the aborted tasks, with the bound `shutdown.pool_close`.
- Cons: the loop does not join an aborted task. A panic during the abort reaches the panic hook only, not the `ERROR` line of the join.

### Option B: Abort and wait for the rest of an existing budget

- Pros: the loop collects each aborted task that yields in time.
- Cons: the loop does not own `shutdown.pool_close`. The wait and the pool close then share one budget, and the sum rule gets a hidden term.

### Option C: No change

- Pros: no work.
- Cons: a task that does not yield keeps the stop open until `SIGKILL`.

The project uses **Option A**. It needs no new number.

## Decision

- (a) The internal listener drains with 0 s. After the pool close of step 5, its loop aborts its open connections at once. The public listener keeps `shutdown.drain`.
- (b) Section 7.11 gets the row `shutdown.internal_drain` with the value 0 s. The value is a code default, not a configuration key (ADR-0062 (a)). The 18 keys of section 7.12 stay.
- (c) Each listener gets its own `ListenerSettings`. Both listeners keep `http.body_read_timeout` as the header read timeout and `http.accept_error_pause` as the pause. Only the drain deadline differs. This item amends ADR-0058 (c).
- (d) After the drain deadline, the loop aborts the open connection tasks with `JoinSet::abort_all`. It does not wait for them, and it returns the count of the cut connections at once. This item amends ADR-0058 (m).
  - An aborted task drops its transaction at its next `.await` point. sqlx then rolls the transaction back when the connection returns to the pool or closes.
  - The pool close of step 5 waits for these connections within `shutdown.pool_close`.
  - A task that does not yield keeps its connection. The pool close then does not complete. It ends when `shutdown.pool_close` expires, and the runtime stop ends at `shutdown.runtime_stop`. The exit of the process closes the socket, and PostgreSQL rolls the transaction back.
  - The drain of 0 s of item (a) uses the same path.
  - The `WARN` line of the cut count of ADR-0058 (o) gives two numbers. These numbers are the cuts of the public listener and the cuts of the internal listener.
- (e) The first sum rule of section 7.3 gets the new term: `shutdown.drain + shutdown.pool_close + shutdown.internal_drain + shutdown.runtime_stop < app.stop_grace_period`. The values give 30 s + 5 s + 0 s + 3 s < 40 s. The test of ADR-0058 (p) reads every value of the rule from section 7.11. This item amends ADR-0058 (n) and (p).
- (f) The order of the stop of ADR-0058 (l) to (o) stays. Only the internal listener changes: it stops after the pool close, and it cuts its connections at once.

## Consequences

### Positive

- The stop with both listeners takes at most 38 s, below the 40 s of `app.stop_grace_period`.
- Every number of the stop has a row in section 7.11.
- The drain of each listener ends at its deadline, also with a task that does not yield.

### Negative

- A probe during the stop of the internal listener loses its connection and gets no answer.
- The loop does not join an aborted task. A panic during the abort reaches only the panic hook of `reliability.md` section 2.4 rule 2.
- A task that does not yield keeps its database connection until the process exits.

### Risks and mitigations

- Risk: a task that does not yield blocks a worker thread of the runtime. Mitigation: `Runtime::shutdown_timeout` returns after `shutdown.runtime_stop`, and the exit of the process ends the thread. PostgreSQL rolls back the open transaction when the socket closes.

## Related

- Amends ADR-0058 (c) (one drain per listener), (m) (the abort without a wait), and (n) (the internal listener cuts at once). Amends ADR-0058 (o) (the cut count gives two numbers) and (p) (the sum rule).
- Amends ADR-0070 (v) (the first sum rule gets the term `shutdown.internal_drain`).
- ADR-0059 (health router), ADR-0062 (a) (configuration keys and code defaults), ADR-0085 (pause after an accept fault).
- `docs/architecture/reliability.md` sections 7.3 (steps 4 to 6, the sum rule) and 7.11 (row `shutdown.internal_drain`).
- `crates/recipe-server/src/listener.rs` (`ListenerSettings::drain_deadline`, `Connections::drain`).
- sqlx 0.8.6 `Pool::close`. tokio 1.54.1 `JoinSet::abort_all` and `JoinSet::shutdown`.
- Baseline requirement: `operations` "Graceful shutdown".
- `overview.md` section 7: R29, R30. Section 9: A41.
