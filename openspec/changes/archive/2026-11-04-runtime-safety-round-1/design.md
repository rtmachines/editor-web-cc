# Design

## Context

See `proposal.md`, section "Why". ADR-0123, ADR-0124, and ADR-0125 hold the decisions. This design links to them and does not repeat them.

The state of the code at HEAD `3885a26`. Each place was read in the source:

| Finding | State | Place |
|---|---|---|
| D7-1 | The session sets `log_statement` and `log_min_error_statement` on lines 70 and 71. The first `CREATE ROLE ... PASSWORD` is on line 72. `\connect recipe` starts a new session later, and no statement after it holds a password | `deploy/db/initdb/01-roles.sh`, `data-model.md` section 5.1 |
| D7-1 | `db` gets `log_min_duration_statement=1000` | `deploy/compose.yaml` line 218 |
| D7-1 | `container_request()` sets the label `org.recipe-editor.storage-tests=recipe-db` and waits only 100 ms. `boot()` calls `remove_old_containers()`, which removes every container with that label. `wait_for_login` waits for the TCP login | `crates/recipe-db/tests/storage/support/container.rs` |
| D4-1 | `pool_options` sets `test_before_acquire(true)` and `after_connect`. It sets no `after_release` hook. Both pools use it | `crates/recipe-db/src/database.rs` line 187 |
| D4-1 | `Database` gives only ports. No `sqlx` type leaves the crate. Items for the storage tests sit behind the feature `test-support` | `crates/recipe-db/src/lib.rs`, `crates/recipe-db/Cargo.toml` |
| D4-1 | `sqlx::query_scalar` sends Parse, Bind, and Execute. `sqlx::raw_sql` sends one `Query` message, and its `fetch_one` returns the row in text format | `sqlx-core-0.8.6` `src/raw_sql.rs`, `sqlx-postgres-0.8.6` `src/connection/executor.rs` |
| D2-1 | `wait_for_signal` blocks on the signal only | `crates/recipe-server/src/serve/server.rs` line 170 |
| D2-1 | `join_listener` awaits the task of a loop during the stop. A `JoinError` gives one `WARN` line | `crates/recipe-server/src/serve/opened.rs` line 183 |
| D2-1 | `spawn_listener` spawns each loop with `tokio::spawn`. The `Host` trait has no part in the spawn | `crates/recipe-server/src/serve/start.rs` line 386, `crates/recipe-server/src/serve/host.rs` |
| D2-1 | `serve_with` builds the `Server`, waits, and stops. It returns `Ok` after each stop. `main` maps an `Err` to one `ERROR` line and the exit code 1 | `crates/recipe-server/src/serve/process.rs` line 165, `crates/recipe-server/src/main.rs` |
| D2-1 | `app` has `restart: always` | `deploy/compose.yaml` line 126 |
| D2-1 | The stub host binds real listeners on port 0 and records events in `Log`. `Fire` fires the signal at a chosen point. The guard `FireOnDrop` fires the signal when a test thread ends. No hook ends one loop without the signal | `crates/recipe-server/src/serve/test_support/stub.rs`, `crates/recipe-server/src/serve/tests/stop.rs` |

Corrections of the review and of the fix plan:

- D7-1: the three `SET` lines must come before each statement with a password, not only before the first `CREATE ROLE`. A `\connect` drops the session settings. ADR-0123 (b) and (e) state the rule.
- D4-1: the review says that writes can be lost. The verdict of the adversarial check narrows this. Only the autocommit session touch is at risk. The next `COMMIT` commits command writes and audit entries.
- D4-1: the plan gives the test no way to reach a raw connection. Task 3.1 adds a `test-support` item for it.
- D4-1: the hook statement must use the simple query protocol. Under the extended protocol, Bind starts the implicit transaction and Execute sets a later statement time. The statement is then true on each release. ADR-0124 (a) states the reason.
- D2-1: the plan names only ADR-0090 (b). ADR-0057 (h) defines the list of rule 6, and ADR-0058 (l) and (o) name only the signal as the start of the stop. ADR-0125 therefore amends all four items.
- D2-1: the project owner picked the exit code mechanism on 2026-11-04. ADR-0125 (d) records it and amends `reliability.md` section 2.3.

## Goals / Non-Goals

**Goals:**

- Close D7-1, D4-1, and D2-1 with no migration, no new error code, and no new setting in `reliability.md` section 7.11.
- Write the failing test of each fault before the fix.
- Change the normative documents in the same change as the ADRs.

**Non-Goals:**

- An upgrade to sqlx 0.9 and a change of A15.
- A new bind of a listener after the end of its loop.
- A rotation of the passwords of an existing installation. The runbook step gives the operator the check.

## Decisions

### D1. The init script (ADR-0123)

1. Add `SET log_min_duration_statement = -1;` to `01-roles.sh` after line 71. Add the same line to the SQL block of `data-model.md` section 5.1.
2. The static test lives in `crates/recipe-db/tests/storage/grants/init_script.rs`, next to `script_text()`. It finds the line of each of the three `SET` statements, the first `\connect`, and each line with the token `PASSWORD :'`. The match uses case, so the `\set ..._password` lines and the comments do not match. Each password line must come after the three `SET` lines and before the first `\connect`. The test also asserts that `document_sql(&["5.1"], &[])` holds the three `SET` lines.
3. `container_request()` of `support/container.rs` gets two parameters: the label value and the extra server options. The harness passes `recipe-db` and no option. The log test passes its own label value `recipe-db-init-log` and `-c log_min_duration_statement=0`. `remove_old_containers()` gets the label value as a parameter. The harness cleanup thus never removes the log test container. The log test removes old containers of its own label before its start.
4. The log test waits until the init ends. `wait_for_login` gives the final TCP login, because the temporary server of the init accepts no TCP login. Then the test reads `pg_roles` and requires the five roles of the script.
5. Then the test reads the stdout and the stderr of the container with `stdout_to_vec` and `stderr_to_vec`. The temporary server of the init writes its log to stdout. It asserts at least one line with `duration:`, so the option was live. It asserts that no line holds `PASSWORD '` or `PASSWORD E'`. It asserts that no line holds the value of `Role::password` for a role of `Role::WITH_SECRET_FILE`. A failure message names the role and not the value. Today the test fails.
6. The log test container does not share the harness container, because the harness migrates its template database once per binary.
7. The runbook step goes into section "Change a database password" as "Check the log of db for a password". The command prints the role names and no password:

   ```bash
   docker compose logs --no-log-prefix db | grep -B1 -E "PASSWORD +E?'" | grep -o -E 'ROLE (postgres|recipe_[a-z]+)' | sort -u
   ```

   `-B1` covers the statement of `recipe_replicator`, which spans two lines. psql writes `E'...'` for a value with a backslash, so the pattern also finds that form. An empty output means that the log holds no password. The operator rotates each named role with the steps of that section. The operator also checks the copy of the log collector, when the site forwards the log.

### D2. The release hook (ADR-0124)

1. `pool_options` adds `.after_release(...)` next to `after_connect`. The hook sends `SELECT transaction_timestamp() <> statement_timestamp()` with `sqlx::raw_sql(...).fetch_one(...)`. It reads column 0 of the row as `bool`. `raw_sql` sends one `Query` message, and the text format decodes `t` and `f`.
2. The SQL text is one `const` with a doc comment that names ADR-0124 (a) and the reason for `raw_sql`.
3. A free function `fn release_verdict(result: Result<bool, sqlx::Error>) -> Result<bool, sqlx::Error>` holds the decision. `Ok(false)` gives `Ok(true)`, and `Ok(true)` gives `Ok(false)`. `Err(_)` passes on unchanged. The hook returns `release_verdict(result)`. On `Err(_)` sqlx calls `close_hard()` and writes one `WARN` line, which the owner accepted (ADR-0124 (a), (c)). A unit test calls the function with no database.
4. The `test-support` items of task 3.1 hold no `sqlx` type in their signatures. The first item acquires one connection, sends a raw statement with `sqlx::raw_sql`, and releases the connection. The second item acquires a connection and reads one row with `sqlx::raw_sql`. The row holds the backend pid, `transaction_read_only`, `transaction_isolation`, and whether `transaction_timestamp()` differs from `statement_timestamp()`. The read must use the simple query protocol for the reason of item 1.
5. With a pool of size 1, each acquire waits until the spawned release ends. The tests therefore need no sleep and have no race.
6. The leak test asserts four facts after the second acquire: a new backend pid, `transaction_read_only` `off`, `transaction_isolation` `read committed`, and no open transaction. Without the hook, the pid stays and the session is read-only.
7. The guard test keeps one connection over several uses. It reads the pid, commits one command through `UnitOfWork`, runs one query, and reads the pid again. Both reads must give the same pid. The test passes without the hook. It fails with a hook that sends the statement through `sqlx::query_scalar`, because that probe closes each released connection.

### D3. The end of an accept loop (ADR-0125)

1. A free async function races three futures: the signal, `&mut` of the public task, and `&mut` of the internal task. A `JoinHandle` is `Unpin`, so `&mut handle` is a future. A `biased` `select!` polls the signal first, so a signal and a loop end at the same time give a normal stop.
2. The function returns a `StopCause`: `Signal`, or `LoopEnded { kind, panicked }`. `StopCause` holds no join result. `Server::wait_for_signal` keeps this value.
3. A second poll of a finished `JoinHandle` panics in Tokio. `ListenerHandle` therefore owns the join result of a loop that ended first. `JoinError` is not `Clone`, so `ListenerHandle` is the only owner. `join_listener` reads the kept result and does not await the task again. A kept `JoinError` gives no `WARN` line, because the `ERROR` line of item 4 covers that fault.
4. `serve_with` returns the typed error `AcceptLoopEnded { kind, panicked }` after the stop of a `LoopEnded`. The error lives in `crates/recipe-server/src/serve/process.rs`. Its `Display` names the listener and tells whether the loop panicked. `main` writes the one `ERROR` line from the chain, as for each other error.
5. The free function `serve::exit_code(error: &anyhow::Error) -> Option<std::process::ExitCode>` lives in `serve/process.rs`. It gives `Some(ExitCode::from(RESTART_EXIT_CODE))` when the chain holds `AcceptLoopEnded`, and `None` otherwise. `RESTART_EXIT_CODE: u8 = 70` sits next to `STALLED_EXIT_CODE` in `crates/recipe-server/src/watchdog.rs`. A unit test asserts that the two constants hold the same value.
6. `main` returns `anyhow::Result<std::process::ExitCode>` (ADR-0125 (d)). `Ok(())` of the subcommand gives `ExitCode::SUCCESS`. An `Err` gives the one `ERROR` line as today. Then `main` returns `Ok(code)` for `Some(code)` of item 5 and `Err(error)` for `None`, so a failed start keeps the exit code 1. No new `#[allow(clippy::exit)]`.
7. The injection point of the tests is a new method of the `Host` trait in `crates/recipe-server/src/serve/host.rs`: `fn spawn_accept_loop(&self, kind: ListenerKind, accept_loop: AcceptTask) -> JoinHandle<DrainOutcome>`. `AcceptTask` is a boxed `Send` future, like `Signal`. `spawn_listener` in `serve/start.rs` calls it. `ProcessHost` in `serve/process.rs` calls `tokio::spawn`.
8. `StubHost` in `serve/test_support/stub.rs` gets a new slot `LoopEnd`, shared by clones like `Fire`. A test chooses one `ListenerKind` and one way: `Return` or `Panic`. `LoopEnd::now()` fires it. The task of the stub races the real loop and the slot. On `Return`, it drops the loop and returns a drain result with no cut connection. On `Panic`, it panics, and the join gives a `JoinError`.
9. Each way of item 8 gets two tests in `crates/recipe-server/src/serve/tests/stop.rs`. The guard `FireOnDrop` fires the signal when a test thread ends, so a run without the fix fails and does not hang.
10. The server test calls `expect_running`, enables `Watch` with `server.health()`, and calls `LoopEnd::now()`. A guard thread fires the signal after twice `SECOND_WAIT_LIMIT`. `server.wait_for_signal()` must return within `SECOND_WAIT_LIMIT`. Then `server.close()` runs, and the test compares the events with the order of `the_stop_follows_the_order_of_the_design`. The comparison leaves out the `ListenerStopped` event of the ended loop, because its socket closes when the loop ends.
11. The process test runs `serve_with` on the stub host. A test thread waits until `/readyz` answers 200, as `fire_when_ready` does. Then it calls `LoopEnd::now()` and waits for `SECOND_WAIT_LIMIT` before its guard fires. The test asserts the error of item 4 and reads the lines of `LogCapture`. `serve_with` builds its own `Server`, so this test cannot enable `Watch`. The server test of item 10 covers the order.

### D4. Documents

| Document | Change | ADR |
|---|---|---|
| `data-model.md` section 5.1 | One `SET` line in the SQL block | ADR-0123 (c) |
| `reliability.md` section 7.5 | One sentence: the `after_release` hook closes a released connection with an open transaction | ADR-0124 (e) |
| `reliability.md` section 2.3 | The sentence on `main`: `anyhow::Result<std::process::ExitCode>`, the exit code 1 of a failed start, and the exit code 70 after an early loop end | ADR-0125 (d) |
| `reliability.md` section 2.4 rule 5 | The watchdog stops at the signal or at the end of an accept loop | ADR-0125 (i) |
| `reliability.md` section 2.4 rule 6 | Eight places. Place 3 also covers the `JoinSet` of the job loops. The seventh is the join of an accept loop. The eighth is the join of the watchdog thread | ADR-0125 (f) |
| `reliability.md` section 7.3 | The intro names ADR-0125. Step 1 gets the end of an accept loop as a second start of the stop. Step 6 gets the exit code 70 of that case | ADR-0125 (g), (i) |
| `.claude/rules/rust.md` | The list of the places that catch a panic, and the sentence on `main` | ADR-0125 (d), (f) |
| `CLAUDE.md` | The sentence on `main` in "Hard rules" | ADR-0125 (d) |
| `docs/ops/runbook.md` | The step of D1 item 7 | ADR-0123 (d) |

### D5. Run order

1. Group 1 first. No task waits for an owner answer.
2. Groups 2, 3, and 4 touch disjoint files and can run in parallel. Task 2.3 runs after 2.1 and 2.2. The tasks of group 3 and of group 4 run in their number order.
3. Group 5 and group 6 run last.

## Risks / Trade-offs

- [The log test starts one more container] → It adds about 10 s to the storage suite. The test runs once per binary.
- [Each release costs one round trip] → The release runs in a spawned task. The load test of `reliability.md` section 6 measures the latency.
- [A later edit sends the hook statement through `sqlx::query`] → The guard test of D2 item 7 fails. The doc comment of the `const` names the reason.
- [A loop that panics at each start makes a restart loop] → The runbook section "Restart loop" covers it.
- [The `Host` trait gets a method that only tests vary] → The trait already exists for the stub host. `ProcessHost` keeps the behavior of `tokio::spawn`.

## Rejected review points

None. The plan review of 2026-11-04 found H1, M1 to M4, and L1 to L8. This design applies each of them.

## Open Questions

None.
