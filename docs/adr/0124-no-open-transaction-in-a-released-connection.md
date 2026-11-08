# ADR-0124: No open transaction in a released connection

Status: Accepted. Amends ADR-0058 (m)
Date: 2026-11-04
Deciders: project owner

## Context

ADR-0058 (m) states that an aborted request drops its connection and that an open transaction then rolls back. The review of 2026-11-04 (finding D4-1) traced the sqlx 0.8.6 source and found a window in which this is false:

1. `Transaction::begin` queues `BEGIN`, waits for the reply, and then sets the transaction depth of the connection to 1.
2. A drop of the future during that wait leaves the depth at 0. The drop guard sends a `ROLLBACK` only when the depth is above 0, so it sends nothing.
3. The release of the connection runs `ping`. The ping reads the reply of `BEGIN` and succeeds. The connection goes back to the pool with an open server transaction.
4. The next `BEGIN` on that connection gets only the warning `25001`. The next transaction then has the isolation and the access mode of the leaked `BEGIN`.

The request timeout (`http.request_timeout`), the export limit (`timeout.export`), and a closed client connection can drop the future in that window. The effects are a lost single snapshot of the queries of ADR-0042 and ADR-0050 and a `25006` for a write in a leaked `READ ONLY` transaction. A session touch can also stay uncommitted. The window is one round trip, and the next `COMMIT` or `idle_in_transaction_session_timeout` ends the state. No command write and no audit entry is lost.

`Connection::is_in_transaction()` of sqlx reads the depth, which is 0 in this state. Only the server knows that the transaction is open.

`pool_options` in `crates/recipe-db/src/database.rs` sets `test_before_acquire(true)` and `after_connect`. It sets no `after_release` hook and no `before_acquire` hook. Both pools of `serve` use `pool_options`.

## Decision drivers

- Each command runs in its own transaction with its own isolation (R29, `data-model.md` section 6).
- A drop of a request never leaves a state that the next request sees (R30).
- A15 stays unchanged: sqlx 0.8 (decision Q-A of the fix plan).
- No new setting in `reliability.md` section 7.11 and no new error code.
- No log line outside the places of `reliability.md` section 1.1, except the one `WARN` line of item (c).

## Considered options

### Option A: An `after_release` hook that asks the server

- Pros: one hook in one function covers every adapter of the crate. A connection with an open transaction never reaches the idle queue.
- Cons: one more round trip on each release. The release runs in a spawned task, so the request does not wait for it.

### Option B: An `after_release` hook that sends `ROLLBACK`

- Pros: the connection stays open.
- Cons: PostgreSQL writes the warning `25P01` on each release without a transaction. The log of `db` then gets one line per request.

### Option C: An upgrade to sqlx 0.9

- Pros: the newer pool can handle a cancellation inside `begin`.
- Cons: a change of A15 and of many call sites. The project owner rejected it (Q-A).

## Decision

The project uses **Option A**. It amends ADR-0058 (m).

- (a) `pool_options` sets an `after_release` hook. The hook runs `SELECT transaction_timestamp() <> statement_timestamp()` on the released connection. It sends the statement with `sqlx::raw_sql`, which uses the simple query protocol, and reads the one row.
  - `false` means no open transaction. The hook returns `Ok(true)`, and the connection goes back to the pool.
  - `true` means an open transaction. The hook returns `Ok(false)`, and sqlx closes the connection with `close()`. PostgreSQL then rolls the transaction back.
  - An error means a broken or aborted connection. The hook returns the error. sqlx then calls `close_hard()`, so a half-alive socket does not block in `close()`. sqlx also writes one `WARN` line with the text "error from `after_release`".
  - The simple query protocol sends one `Query` message. Outside a transaction, the implicit transaction starts with that statement, so the two timestamps are equal.
  - `sqlx::query_scalar` uses the extended protocol. PostgreSQL then starts the implicit transaction at Bind and sets a later statement time at Execute. The statement is then true on each release, and the hook closes every connection.
- (b) The hook does not use `Connection::is_in_transaction()`, because that method reads the depth of sqlx. The free function `release_verdict` maps the result of the statement to the answer of the hook, so a unit test needs no database.
- (c) The hook itself writes no log line. The owner accepted one `WARN` line of sqlx for a failed probe on 2026-11-04, because no other code logs that error.
- (d) The statement runs under the `statement_timeout` of the session (`db.statement_timeout`). The pool close of `shutdown.pool_close` bounds the wait for a release during the stop.
- (e) `reliability.md` section 7.5 names the hook. A15 stays unchanged.
- (f) A storage test with a pool of size 1 sends a raw `BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY` on an acquired connection and releases it. It acquires again and asserts that the server shows no open transaction. A second test sends a raw `BEGIN` and a statement that fails, and asserts a new connection. A guard test runs a command and a query on one connection and asserts one backend pid for all of them. The guard test fails with a hook that uses the extended protocol.

## Consequences

### Positive

- No connection with an open server transaction goes back to the pool.
- The rule of ADR-0058 (m) holds also inside the window of `BEGIN`.

### Negative

- Each release costs one more round trip to the server.
- A closed connection costs a new connect on the next acquire.
- The log does not show how often the hook closes a connection.
- A failed probe writes one `WARN` line from sqlx.

### Risks and mitigations

- Risk: the two timestamps are equal inside an open transaction. Mitigation: at least one round trip separates the leaked `BEGIN` and the hook statement. The server reads the clock in microseconds.
- Risk: a later edit sends the hook statement through the extended protocol. Mitigation: the guard test of item (f) fails, and the doc comment of the statement names the reason of item (a).
- Risk: the extra round trip slows the release under load. Mitigation: the release runs in a spawned task. The load test of `reliability.md` section 6 measures the latency.

## Related

- ADR-0058 (m) (abort of a connection during the drain), ADR-0087 (d) (abort without a wait)
- ADR-0042 and ADR-0050 (single snapshot of the draft, preview, and diff queries)
- `reliability.md` sections 7.4 and 7.5, `data-model.md` section 6
- `overview.md` section 7: R29, R30. Section 9: A15
