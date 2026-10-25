# ADR-0085: Pause after an accept fault

Status: Accepted
Date: 2026-10-24
Deciders: project owner

## Context

ADR-0058 (a) gives `recipe-server` an own accept loop. Task 7.4 writes it. The review of the loop found a number without a row of `reliability.md` section 7.11:

1. A call of `accept` can fail with a fault of the listener or of the host, for example too many open files (`EMFILE`). The next call then fails again at once.
2. A loop without a pause then takes a full CPU and writes one log line for each failed call.
3. A connection fault, for example a reset before `accept` completes, concerns one connection only. The loop continues at once.
4. The loop therefore pauses after a fault that is not a connection fault. The code has a field `accept_error_pause` with no default, because no row of section 7.11 gives the number.

## Decision drivers

- Every numeric setting has one row in section 7.11.
- One fault of the host cannot take the CPU of `app` or fill its log (R30, A1).
- No new configuration key. Section 7.12 keeps its 18 keys (ADR-0062 (a)).

## Considered options

### Option A: A code default of 1 s

- Pros: the same pause as `axum::serve` of axum 0.8.9. One line in section 7.11 and no new key.
- Cons: the loop accepts no connection for 1 s after each such fault.

### Option B: A configuration key

- Pros: an operator can change the pause.
- Cons: ADR-0062 and section 7.12 change. The operator gets a value with no known use.

### Option C: No pause

- Pros: no new number.
- Cons: a hot loop on `EMFILE` takes a full CPU and fills the log.

## Decision

The project uses **Option A**.

- (a) Section 7.11 gets the row `http.accept_error_pause` with the value 1 s. The value is a code default, not a configuration key (ADR-0062 (a)). The 18 keys of section 7.12 stay.
- (b) After a fault of `accept` that is not a connection fault, the loop writes one `WARN` line. Then it pauses for `http.accept_error_pause` and continues. A stop during the pause ends the loop.
- (c) After a connection fault (`ConnectionAborted`, `ConnectionReset`, or `ConnectionRefused`), the loop continues without a pause.
- (d) This ADR amends ADR-0058 by this one number. The other items of ADR-0058 stay.

## Consequences

### Positive

- A fault of the host gives at most one `WARN` line per second for each listener.
- Every number of the accept loop has a row in section 7.11.

### Negative

- After such a fault, the listener accepts no connection for 1 s.

### Risks and mitigations

- Risk: a long fault of the host stops the service of new connections. Mitigation: the watchdog of `reliability.md` section 2.4 probes the internal listener. A fault of the process, such as `EMFILE`, may also stop that probe. The watchdog then ends the process, and Docker restarts the container.

## Related

- Amends ADR-0058 (the pause of the accept loop).
- ADR-0062 (a) (configuration keys and code defaults).
- `docs/architecture/reliability.md` section 7.11 (row `http.accept_error_pause`).
- `crates/recipe-server/src/listener.rs` (`ListenerSettings::accept_error_pause`).
- axum 0.8.9 `serve` (the pause of 1 s after an accept error that is not a connection error).
- Baseline requirement: `operations` "Graceful shutdown".
- `overview.md` section 7: R30. Section 9: A1.
