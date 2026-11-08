# ADR-0125: The end of an accept loop stops the server

Status: Accepted. Amends ADR-0057 (h), ADR-0058 (l) and (o), and ADR-0090 (b)
Date: 2026-11-04
Deciders: project owner

## Context

`serve` starts one accept loop for the public listener and one for the internal listener (ADR-0058 (a)). `spawn_listener` in `crates/recipe-server/src/serve/start.rs` spawns each loop as a Tokio task. The handle of each task stays in `ListenerHandle`.

The review of 2026-11-04 (finding D2-1) found this gap:

- `Server::wait_for_signal` (`serve/server.rs`) waits only for the stop signal.
- Only `join_listener` (`serve/opened.rs`) awaits the task of a loop, and only during the stop.
- A panic of the public loop closes the public port. Caddy then gets "connection refused".
- The watchdog probes only the internal listener, so `/healthz` stays `ok`. The process serves nothing until a person restarts it.

The loop body has no known panic path. The finding is a gap of containment, not a known fault.

Two joins already catch a panic, and `reliability.md` section 2.4 rule 6 names neither of them:

- `join_listener` reads the `JoinError` of a loop and writes one `WARN` line.
- `Watchdog::stop` (`crates/recipe-server/src/watchdog.rs`) joins the watchdog thread. `CloseReport` records a panic of the thread in `watchdog_panicked`.

ADR-0057 (h) defines the list of rule 6. ADR-0092 added the sixth place.

## Decision drivers

- The process serves requests or stops, so Docker can restart it (R30, A41).
- The stop keeps the order and the budget of `reliability.md` section 7.3.
- Each fault gets one log line (`reliability.md` section 1.1).
- Rule 6 names every place that catches a panic.
- No new setting in section 7.11. No new error code. The marker `AcceptLoopEnded` owns no code (item (c)).

## Considered options

### Option A: The server stops when a loop ends, and Docker restarts it

- Pros: one restart path for a stalled runtime and for a dead loop. The watchdog rule of the operations spec already uses the exit code 70 for the restart.
- Cons: the requests in flight on the other listener get the drain of the stop, not a full answer.

### Option B: The server opens the listener again

- Pros: the other listener and the jobs keep running.
- Cons: a loop that panicked can panic again. The bind needs a retry policy and new settings.

The project owner chose **Option A** on 2026-11-04 (decision Q-C (1) of the fix plan).

## Decision

The project uses **Option A**. It amends ADR-0057 (h), ADR-0058 (l) and (o), and ADR-0090 (b). It also amends `reliability.md` section 2.3 (the sentence on `main`), section 2.4 rules 5 and 6, and section 7.3 steps 1 and 6.

- (a) `Server::wait_for_signal` waits for the first of three events: the stop signal, the end of the public loop, and the end of the internal loop. It keeps the event as a `StopCause`. The cause holds the listener and whether its loop panicked, and no join result.
- (b) When a loop ends first, `ListenerHandle` keeps the join result of that loop. It is the only owner, because `JoinError` is not `Clone`. The stop then runs steps 2 to 6 of `reliability.md` section 7.3. The stop reads the kept result and does not await that join again.
- (c) After the stop, `serve` returns an error. The error names the listener and tells whether its loop panicked. `main` writes the one `ERROR` line of this fault. The stop writes no `WARN` line for that loop. The line of the panic hook does not count (ADR-0061 (j)). The error is the marker type `AcceptLoopEnded` of `recipe-server`. It owns no error code. It stays a marker in the `anyhow` chain, and `exit_code` reads it to give exit code 70. The project owner chose this exception on 2026-11-04.
- (d) `recipe-server::main` returns `anyhow::Result<std::process::ExitCode>`. For the error of item (c), `main` writes its `ERROR` line and returns `ExitCode::from(70)`. Every other error keeps the `ERROR` line and the exit code 1, also a failed start. No new function gets `#[allow(clippy::exit)]`, and the watchdog function stays the only one. The project owner chose this mechanism on 2026-11-04. This amends the sentence on `main` in `reliability.md` section 2.3.
- (e) The setting `restart: always` of `app` in `deploy/compose.yaml` makes Docker start the container again.
- (f) Section 2.4 rule 6 names eight places. Place 3, the supervisor of rule 4, also covers the `JoinSet` of the job loops in `Supervisor::drain`. The seventh place is the join of the task of an accept loop. A panic there ends the loop, and item (a) stops the server. The eighth place is the join of the watchdog thread in `Watchdog::stop`. A panic there sets `watchdog_panicked`, and the stop continues. This amends ADR-0057 (h). `.claude/rules/rust.md` names the same eight places.
- (g) A stop after the signal keeps the exit code 0 of ADR-0058 (o) and ADR-0090 (b). A stop after item (a) uses the exit code of item (d), also after a cut drain or a pool close that reached its limit. Section 7.3 step 6 gets this case.
- (h) The tests reach the loop through a new method of the `Host` trait, `spawn_accept_loop`. `ProcessHost` spawns the loop with `tokio::spawn`. The stub host can end one loop with a return or with a panic. A test on the stub host ends a loop without a signal. It asserts the error of item (c), the stop order of section 7.3, and no `WARN` line. A second test ends a loop with a panic and runs the full stop.
- (i) The stop starts at the signal or at the end of an accept loop. This amends ADR-0058 (l), section 2.4 rule 5, and section 7.3 step 1. In both cases, readiness turns off, the watchdog stops, the public listener stops accepting, and the jobs get the cancel.

## Exit code mechanism

`reliability.md` section 2.3 permits `std::process::exit(70)` only in the watchdog function, and `main` returned `anyhow::Result<()>`. The options were:

1. `main` returns `anyhow::Result<std::process::ExitCode>` and gives the code 70 for the error of item (c). This amends the sentence on `main`.
2. A second function in `serve` with `#[allow(clippy::exit)]` calls `std::process::exit(70)` after the stop. This amends the rule of the one exit function.
3. The process exits with code 1 through the error path of `main`. This changes decision Q-C (1) of the fix plan.

The project owner chose option 1 on 2026-11-04. `restart: always` restarts the container after each exit code. The code 70 tells the operator that the process ended for a restart.

## Consequences

### Positive

- A dead accept loop no longer leaves a process that serves nothing.
- Rule 6 names every join that catches a panic.

### Negative

- A panic of one loop stops the requests in flight on the other listener after the drain.
- A loop that panics at each start makes a restart loop. The runbook section "Restart loop" covers it.

### Risks and mitigations

- Risk: the stop awaits a finished join a second time, and Tokio panics. Mitigation: item (b) keeps the result, and the tests of item (h) run the full stop after a loop end and after a panic.

## Related

- ADR-0057 (h) (places that catch a panic), ADR-0058 (accept loop and stop), ADR-0061 (start and signal)
- ADR-0090 (b) (exit code after a pool close), ADR-0092 (sixth place)
- `reliability.md` sections 1.1, 2.3, 2.4, and 7.3
- Baseline requirements: operations "Graceful shutdown" and "Watchdog"
- `overview.md` section 7: R30. Section 9: A41
