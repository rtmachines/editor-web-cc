# ADR-0172: Dropped future after a committed step

Status: Accepted. Amends ADR-0125 (f), ADR-0130 (b) and (c), and ADR-0133 (c). Amended by ADR-0173 (the event of a T2 refusal, a lost error result, the slot order, the slot after the drain)
Date: 2026-11-14
Deciders: project owner

## Context

A request future can stop at each `.await`. `request_timeout` in `crates/recipe-web/src/timeout.rs` drops the inner future at `http.request_timeout`. A client that closes the connection also drops it. The export paths skip `request_timeout`, so only a closed connection drops their future.

The review of 2026-11-13 found three effects of a dropped future after a committed step (findings F-06, F-07, and F-22).

- F-06 (a): `CreateUser` and `ResetPassword` await the commit as their last await (`create_user.rs:112`, `reset_password.rs:105`). After a late COMMIT reply, the client gets `503 E-WEB-009` and no one-time password.
- F-06 (b): the form `user-new-form` (`templates/user-new.html:6`) has no `hx-boost` and no `hx-sync`. A second submit gets the refusal of a duplicate login.
- F-07: four use cases commit a step and then await `security.record` in the request future. A dropped future then leaves the committed step without its security event.
- F-22: `read_with_limit` (`crates/recipe-server/src/limited_read.rs`) marks a path only when the wait reaches the limit. A caller that drops before the limit leaves a running call without a mark.

The four use cases of F-07 are these:

- `UnlockLogin` commits `LoginUnlocked` and then awaits the event `unlock` (`unlock_login.rs:79`, `:87`).
- `Login` commits the login attempt in T1 (`begin_attempt`, `login.rs:131`). Then the Argon2 check runs, and `refuse` awaits the event.
- `ChangeOwnPassword` commits the attempt (`change_own_password.rs:86`). Then `reject_throttled` (`throttle.rs:103`) awaits the event.
- The Basic check of an export route commits the attempt in T1 (`authenticate_basic.rs:84`). Then the Argon2 check runs, and `refuse` writes the event.

A client with a short timeout can therefore leave login attempt rows without an event `login_failed` (A33). For F-07, the lost event also leaves no `WARN` line.

For F-22, each job waits for each read (ADR-0161 (b)). Only two paths drop a job run. `Supervisor::drain` aborts the open runs after its deadline. The start race drops the start steps on the stop signal (`serve/start.rs`). `shutdown_timeout(shutdown.runtime_stop)` in `serve/opened.rs` bounds the exit with a blocked thread.

## Decision drivers

- `security_event` records every authentication failure and every unlock (A33).
- The server logs a fault exactly once (`reliability.md` §1.1).
- A wait for the Argon2 gate stays cancellable, and `argon2.permits` bounds the Argon2 work.
- No step after the judge runs for a client that is gone.
- A known limit names the next step of the operator or of the user.

## Considered options

### Option A: Detach the span from T1 to the security event (F-07)

- Pros: the event and its `WARN` line survive a dropped future. The gate wait stays cancellable. T2 and `replace` do not run for a client that is gone.
- Cons: a new task set, a new setting, a new catch place, and a new step of the stop.

### Option B: Detach the whole body of each command (F-07)

- Pros: one rule for the full command.
- Cons: T2 and `replace` then run after the client is gone. The gate wait stops being cancellable.

### Option C: Record F-07 as a known limit

- Pros: no code change.
- Cons: the gap hides failed logins from `security_event`.

### Option D: Run the commit of F-06 (a) in a task that the request cannot cancel

- Pros: the commit always completes.
- Cons: the password still cannot reach a client that is gone.

### Option E: A timer task that marks the path after the limit (F-22)

- Pros: a dropped call also gets its mark.
- Cons: a new catch place and a new task for a case of the stop only.

## Decision

The project uses **Option A** for F-07. F-06 (a) and F-22 are known limits. F-06 (b) gets a fix.

### One-time password after a dropped request (F-06)

- (a) Known limit. The window of F-06 (a) is one COMMIT round trip. Option D does not help, because the password cannot reach a client that is gone.
- (b) After a `503 E-WEB-009` of `POST /users` or of a reset, the Administrator opens the user list. When the user exists, the Administrator runs `ResetPassword` to get a new one-time password.
- (c) `docs/ops/runbook.md` states the step of item (b).
- (d) The form `user-new-form` gets `hx-boost="true"`, `hx-sync="closest form:drop"`, and `hx-push-url="false"`. The answer stays the full result page (`api.md` §2.8).
- (e) `.claude/rules/web.md` adds the form `user-new-form` to the list of `hx-sync` (ADR-0049).
- (f) A browser test sends a double submit. It finds one `POST /users`, and the page shows the one-time password.

### Security event after a dropped request (F-07)

- (g) The rule covers four use cases: `Login`, `ChangeOwnPassword`, `UnlockLogin`, and `AuthenticateBasic`.
- (h) The request future takes the `HashGate` permit or the Basic permit, as today. The gate wait therefore stays cancellable.
- (i) A detached task runs only the span from T1 to the write of the security event. The request future moves the permit into the task, so `argon2.permits` still bounds the Argon2 work.
- (j) The task does not run T2 of `Login`, `replace` of `ChangeOwnPassword`, or the step after the judge of `AuthenticateBasic`.
- (k) The task sends its judge result through a `oneshot`. The request future then runs T2 or `replace`.
- (l) When the receiver is closed, the task skips that step, because no future waits for it.
- (m) The four use cases share one function that writes the event. On a failed event write, that function writes one `WARN` line.
- (n) The task writes this `WARN` line itself, before it sends its result. A cancel before the send, after the send, or after the receive gives exactly one line.
- (o) The use case does not return the storage fault of the event. The handler never logs a lost event.
- (p) Owner decision 2026-11-14: a failed event write gives the one `WARN` line of the task. `UnlockLogin` completes as ADR-0130 (a) states.
- (q) After a failed event write, `Login`, `ChangeOwnPassword`, and the Basic check give their refusal or their throttle answer, as without the fault.
- (r) The handler of `POST /users/{login}/unlock` loses its call of `log_lost_unlock_event`. This amends ADR-0130 (b) and (c) for the log place of a lost event.
- (s) `reliability.md` §1.1, `CLAUDE.md`, and `.claude/rules/rust.md` name the task as the log place of a lost event.
- (t) The task runs with `.instrument(Span::current())`, so its lines keep the span and the `request_id` of the request.

### Bound of the detached tasks (F-07)

- (u) A new setting `identity.detached_tasks` bounds the count of open detached tasks. `reliability.md` §7.11 gets its row.
- (v) Owner decision 2026-11-14: `identity.detached_tasks` is 8, two times `argon2.permits`.
- (w) The request future takes a slot before T1. A slot that does not come within `argon2.acquire_timeout` gives `503 E-WEB-009`, and nothing commits. The timeout gets a new stage of `TimeoutStage`.
- (x) The slot is an owned semaphore permit that the task holds. The permit drops when the task ends, also after an error and during the unwind of a panic.
- (y) The detached tasks run in one `JoinSet` behind a `parking_lot::Mutex`. Each spawn first reaps every finished task with `try_join_next`.
- (z) The `JoinSet` therefore holds at most `identity.detached_tasks` open tasks and the tasks that ended since the last spawn.

### Panic of a detached task (F-07)

- (aa) A panic closes the sender. The use case maps the closed sender to `AppError::TaskFailed` with a new stage of `TaskStage` (`E-WEB-011`).
- (ab) Owner decision 2026-11-14: the reaper writes no line for a panic. A reaped panic only frees its slot.
- (ac) The panic hook line of `reliability.md` §2.4 rule 2 stays. While the client waits, the `WebError` mapping also logs the `TaskFailed` of the request, as for `spawn_blocking` today.
- (ad) The join of a detached task is a ninth place that catches a panic in `reliability.md` §2.4 rule 6. This amends ADR-0125 (f), which counts eight places.
- (ae) `reliability.md` §2.2 (row `JoinHandle`) and §2.4 rule 7 name the join of a detached task next to `spawn_blocking`.

### Stop of the server (F-07)

- (af) The stop drains the detached tasks before it closes the pools (`serve/opened.rs`). They run in step 3 of `reliability.md` §7.3 under `shutdown.drain`.
- (ag) The connections and the detached tasks share the one deadline of `shutdown.drain`.
- (ah) Step 4 aborts the open detached tasks with one `WARN` line that gives their count. Their transactions roll back, and then step 5 closes the pools.

### Dropped call of `read_with_limit` (F-22)

- (ai) Known limit. The mark of ADR-0133 (c) covers a call whose caller waits to the limit. This amends ADR-0133 (c).
- (aj) The operator needs no step. The doc comment of `read_with_limit` states the limit.
- (ak) The decision holds for the current callers only. A new caller that can drop the call early needs a new ADR.

## Consequences

### Positive

- `security_event` gets the event of a committed attempt or unlock, also after a dropped request.
- A lost event gives exactly one `WARN` line, with the `request_id` of the request.
- A second submit of the create form of a user does not hide the one-time password.

### Negative

- A detached task can outlive its request.
- The project has a new setting, a ninth catch place, and a new step of the stop.
- A late COMMIT reply of `CreateUser` or `ResetPassword` still loses the one-time password. The Administrator runs `ResetPassword` again.
- A caller of `read_with_limit` that drops before the limit leaves no mark.

### Risks and mitigations

- Risk: many detached tasks run at the same time. Mitigation: `identity.detached_tasks`, `argon2.permits`, and the drain of `reliability.md` §7.3 bound the count and the time.
- Risk: step 4 of the stop aborts a detached task before its event. Mitigation: the `WARN` line of the drain gives the count. This case needs a stop of the server.
- Risk: a new caller drops a call of `read_with_limit` early. Mitigation: item (ak) requires a new ADR for that caller.

## Related

- ADR-0014, ADR-0015, ADR-0022, ADR-0049, ADR-0125 (f), ADR-0130 (a) to (c), ADR-0133 (c), ADR-0161 (b)
- `reliability.md` §1.1, §2.2, §2.4, §7.3, §7.11, `api.md` §2.8
- Change `review-design-fixes`, capabilities `identity-access` and `web-ui`
- `overview.md` §7: R20, R30, R35. §9: A33
