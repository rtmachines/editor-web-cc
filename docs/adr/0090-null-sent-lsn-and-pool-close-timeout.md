# ADR-0090: A `NULL` `sent_lsn` and a pool close that reaches its limit

Status: Accepted
Date: 2026-10-24
Deciders: project owner

## Context

ADR-0089 lists open items. This ADR answers two of them:

1. A row of `walarchive` in `pg_stat_replication` can exist before `sent_lsn` has a value.
   - The function `replication_state()` of `data-model.md` section 5.5 then gives `connected` true and `lag_bytes` `NULL`. `pg_catalog.max` ignores the `NULL` value of the row.
   - The SQL comment of section 5.5 names `NULL` only for the case with no row. The doc of `ReplicationState::lag_bytes` gives `None` only when the receiver is not connected.
   - The adapter of `recipe-db` maps the `NULL` to `None`. The port therefore gives `connected: true, lag_bytes: None`, and no document says what the job `alerts` does with it.
2. `Database::close` returns `CloseOutcome::TimedOut` when `shutdown.pool_close` expires while a connection is still checked out (ADR-0087 (d)). No document says how `serve` reacts to that result (task 7.9).

Facts of the stop:

- After `TimedOut`, a task that does not yield can keep its connection until the process exits. PostgreSQL then rolls its transaction back (ADR-0087 (d)).
- The stop has a fixed budget. The first sum rule of `reliability.md` section 7.3 keeps it below `app.stop_grace_period` (ADR-0087 (e)).

## Decision drivers

- An alert shows each state in which the WAL archive can fall behind (R37, ADR-0020).
- An abort never leaves a partial write (R29).
- After `SIGTERM`, the process exits with code 0 within `app.stop_grace_period` (R30).
- A `WARN` line reports a degraded state (`reliability.md` section 3.3). Each fault gets one log line (`reliability.md` section 1.1).
- No new error code, no new number in section 7.11, no new configuration key, and no new alert condition.

## Considered options for a `NULL` `sent_lsn`

### Option A: The condition `replication` is active

- Pros: the job raises the alert in each state in which it cannot show a lag below `wal.max_lag_bytes`. ADR-0020 requires a connected receiver and a lag below that limit.
- Cons: a receiver that starts can give one alert before its first `sent_lsn`.

### Option B: The condition stays inactive

- Pros: a receiver that starts gives no alert.
- Cons: a receiver that stays in this state never gives an alert, although the archive can fall behind.

The user chose **Option A** on 2026-10-24.

## Considered options for `CloseOutcome::TimedOut`

### Option A: One `WARN` line and the exit code 0

- Pros: PostgreSQL rolls back the transactions of the open connections, so no data is at risk. The stop stays inside its budget.
- Cons: Docker does not see the state in the exit code. Only the `WARN` line shows it.

### Option B: One `ERROR` line and the exit code 1

- Pros: the exit code shows the state.
- Cons: the state is not a loss of data. `restart: always` starts the container again in both cases, so the exit code changes no action.

The user chose **Option A** on 2026-10-24.

## Decision

- (a) The port value `connected: true, lag_bytes: None` means a row of `walarchive` without a `sent_lsn`. The job `alerts` then makes the condition `replication` active.
  - The condition is inactive only when the receiver is connected and its lag is known and below `wal.max_lag_bytes`.
  - The condition is active in the three other states: no row, a lag at or above the limit, or a row without a lag.
  - `connected: false, lag_bytes: None` keeps its meaning: no row of `walarchive` exists.
  - The function `replication_state()` and its grant stay as they are. Only its SQL comment in `data-model.md` section 5.5 gets the new case.
  - The adapter of `recipe-db` stays. It already maps the `NULL` lag to `None`.
  - The doc of `ReplicationState::lag_bytes` names both cases of `None`.
  - The start grace of design D1 and the line rules of `reliability.md` section 3.7 apply to this state as to the other states.
- (b) When `Database::close` returns `CloseOutcome::TimedOut`, `serve` writes one `WARN` line for that pool. Then the stop continues with the stop of the internal listener and the runtime stop.
  - The process exits with code 0, as for a stop with a pool close that completes.
  - The line names the pool, the request pool or the health pool. It holds no error code, because `CloseOutcome` is no error type.
  - The line is the one line of this fault (`reliability.md` section 1.1). `Database::close` writes no line.
  - The stop uses the existing limit `shutdown.pool_close`. This item adds no number to section 7.11 and no term to the sum rules of section 7.3.

The rules live in `reliability.md` section 3.7 (the condition `replication`) and section 7.3 (steps 5 and 6).

## Consequences

### Positive

- The WAL archive cannot fall behind without an alert.
- A pool close that reaches its limit is visible in the event log.
- The stop keeps its budget and its exit code.

### Negative

- A receiver that starts can give one alert of `replication` and one webhook call before its first `sent_lsn`.
- Docker does not see a pool close that reached its limit. Only the `WARN` line shows it.

### Risks and mitigations

- Risk: a receiver stays in the state without a `sent_lsn`. Mitigation: the condition stays active, and the job writes its `WARN` line in each run.
- Risk: a task that does not yield keeps its connection after the pool close. Mitigation: the exit of the process closes the socket, and PostgreSQL rolls the transaction back (ADR-0087 (d)).

## Related

- Answers two open items of ADR-0089. The third open item, a back-off for a daily job that panics in each run, stays open.
- ADR-0075 keeps its name `replication`. The condition of that name gets one more state, and its name rules stay. ADR-0075 therefore gets no note.
- ADR-0069 (h) keeps the function and its result row. ADR-0069 therefore gets no note.
- ADR-0058 (n) and (o) and ADR-0087 (d) and (e) keep their text. Item (b) adds the reaction to a pool close that reaches its limit.
- ADR-0020 (WAL health), ADR-0068 (p), ADR-0060 (e).
- `docs/architecture/reliability.md` sections 1.1, 3.3, 3.7, 7.3, and 7.7. `docs/architecture/data-model.md` section 5.5.
- `crates/recipe-app/src/ports/operations_reads.rs` (`ReplicationState`), `crates/recipe-db/src/database.rs` (`Database::close`, `CloseOutcome`), `crates/recipe-db/src/operations.rs` (`into_snapshot`).
- Baseline requirements: operations "Alert conditions" and "Graceful shutdown".
- `overview.md` section 7: R29, R30, R37. Section 9: A41.
